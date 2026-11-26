//! 第三刀驗收測試：撤除櫃台保母硬鎖、冷爐主動發起購煤契收貨復燃、斷橋自主伐木搶修、缺糧轉行墾荒與九方針解析。

use super::actions::apply_world_action;
use super::appraisal::GoalStrategyKind;
use super::clock_and_lang::solar_clock_at;
use super::desire_effects::execute_arrived_desire_effect;
use super::desire_sim::{BellDecisionIndex, evaluate_personalized_desire};
use super::engine::load_static_world_doc;
use super::household_economy::{
    count_box_home_rations_only, deduct_box_home_rations, ensure_household_larders_seeded_once,
    find_town_tavern_and_keeper,
};
use super::live_commodities::{
    APOTHECARY_HERB_SLUGS, SMITHY_FUEL_SLUGS, consume_shop_stock_one, is_apothecary_out_of_herbs,
    is_smithy_forge_cold,
};
use super::live_land::{ensure_live_land_seeded, is_river_crossing_impassable_for_wheels};
use super::movement::add_carry_item;
use super::persona::derive_innate_persona;
use super::sim_tick::seed_genesis_actors_once;
use super::slow_think::parse_slow_think_chinese_output;
use super::types::{CarryItem, IslandWorldDelta, WorldStaticDoc};
use super::unified_kernel::{dispatch_extended_shared_verb, execute_player_parity_bell_actions};

fn fresh_world() -> (WorldStaticDoc, IslandWorldDelta) {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    ensure_household_larders_seeded_once(&mut delta);
    assert_eq!(delta.conservation_diff(&doc), 0);
    (doc, delta)
}

#[test]
fn cold_forge_releases_counter_lock_posts_coal_pact_and_fulfillment_relights_forge() {
    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let day_clk = solar_clock_at(epoch + 1_200);
    let abs_bell = day_clk.game_days * 12 + i64::from(day_clk.bell_index);

    let smith_id = "res:white_gull_harbor:shop_0_0".to_string();
    let forge_node = "oi:white_gull_harbor:bal_forge";
    let app_id = "res:white_gull_harbor:shop_0_1".to_string();

    // 移開學徒使鐵匠獨處於鋪內，並抽乾鍛爐全部燃煤木柴
    delta.relocate_actor(&app_id, "oi:white_gull_harbor:plaza", 4616, true);
    delta.relocate_actor(&smith_id, forge_node, 4616, true);
    while consume_shop_stock_one(&mut delta, None, forge_node, &SMITHY_FUEL_SLUGS).is_some() {}
    assert!(is_smithy_forge_cold(&delta, forge_node));

    // 1. 驗證舊的「搭檔不在即鎖死站櫃」已被撤除，熄火鐵匠不再死守冷爐 (`walk_work`)
    let bell_idx = BellDecisionIndex::build(&delta);
    let (tav_opt, keeper_opt) = find_town_tavern_and_keeper(&doc, &delta, 4616);
    {
        let smith = delta.actors.get_mut(&smith_id).unwrap();
        smith.mind.stm.affect.hunger = 0;
        smith.fatigue = 10;
        smith.mind.stm.affect.fatigue = 10;
        smith.copper = smith.copper.max(24);
    }
    let smith_ro = delta.actors.get(&smith_id).unwrap();
    let innate = derive_innate_persona(smith_ro.soul_seed.max(1));
    let dec = evaluate_personalized_desire(
        &doc,
        &delta,
        &bell_idx,
        smith_ro,
        &innate,
        tav_opt.as_deref(),
        keeper_opt.as_deref(),
        day_clk.bell_index,
        abs_bell,
    );
    assert_ne!(
        dec.kind, "walk_work",
        "cold lone smith must not be trapped at cold counter (got: {} - {})",
        dec.kind, dec.reason_zh
    );

    // 2. 鐵匠自主透過共享動詞核掛出「北林精煤塊」收購委託契，另一居民履約交煤即重燃鍛爐！
    let claimed =
        execute_player_parity_bell_actions(&doc, &mut delta, &day_clk, &[smith_id.clone()]);
    assert!(claimed.contains(&smith_id));
    let coal_pact_id = delta
        .contracts
        .iter()
        .find(|(_, c)| {
            c.creditor_id == smith_id
                && c.kind == "supply_commission"
                && c.status == "escrowed"
                && c.note_zh.starts_with("black_coal｜")
        })
        .map(|(cid, _)| cid.clone())
        .expect("cold smith must propose black_coal supply commission");

    let runner_id = "res:white_gull_harbor:shop_1_1".to_string();
    delta.relocate_actor(&runner_id, forge_node, 4616, true);
    add_carry_item(
        &mut delta.actors.get_mut(&runner_id).unwrap().backpack,
        CarryItem {
            slug: "black_coal".to_string(),
            name: "北林精煤塊".to_string(),
            count: 1,
            weight_units: 4,
            ..CarryItem::default()
        },
    );
    dispatch_extended_shared_verb(
        &doc,
        &mut delta,
        &day_clk,
        &runner_id,
        "fulfill_pact",
        &coal_pact_id,
    )
    .expect("delivering coal fulfills commission pact");
    assert!(
        !is_smithy_forge_cold(&delta, forge_node),
        "delivered coal must land in bal_forge locker and relight the forge"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn washed_out_bridge_is_autonomously_repaired_by_resident_reopening_road() {
    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let day_clk = solar_clock_at(epoch + 1_200);
    ensure_live_land_seeded(&doc, &mut delta, day_clk.game_days);

    // 沖毀 4527 號橋樑地塊
    let bridge_cid = 4527u32;
    {
        let cl = delta.live_land.cells.get_mut(&bridge_cid).unwrap();
        cl.has_river_crossing = true;
        cl.bridge_integrity = 0;
    }
    assert!(is_river_crossing_impassable_for_wheels(&delta, bridge_cid));

    let bridge_nid = doc
        .nodes
        .iter()
        .find(|(_, n)| n.cell_id == bridge_cid && !n.indoor && !n.dungeon)
        .map(|(nid, _)| nid.clone())
        .expect("bridge outdoor node exists");
    let forest_cid = delta
        .live_land
        .cells
        .iter()
        .find_map(|(&cid, cl)| {
            (cid != bridge_cid
                && cl.forest_density >= 30
                && doc
                    .nodes
                    .values()
                    .any(|n| n.cell_id == cid && !n.indoor && !n.dungeon))
            .then_some(cid)
        })
        .expect("forest cell exists");
    let forest_nid = doc
        .nodes
        .iter()
        .find(|(_, n)| n.cell_id == forest_cid && !n.indoor && !n.dungeon)
        .map(|(nid, _)| nid.clone())
        .unwrap();

    // 選一位未持木料的居民（掛修橋方針），先赴林地自主伐木，再往斷橋動工搶修！
    let worker_id = "res:white_gull_harbor:cooper_ln_3_0".to_string();
    {
        let w = delta.actors.get_mut(&worker_id).unwrap();
        w.backpack.clear();
        w.fatigue = 10;
        w.wound_part = None;
        w.route_plan.clear();
        w.mind.ltm.known_nodes.insert(forest_nid.clone());
        w.mind.ltm.known_nodes.insert(bridge_nid.clone());
        w.mind.apply_goal_strategy_with_snapshot(
            GoalStrategyKind::RepairCivicWorks,
            None,
            day_clk.game_days,
            4,
            0,
            false,
        );
    }
    delta.relocate_actor(&worker_id, &forest_nid, forest_cid, true);
    let c1 = execute_player_parity_bell_actions(&doc, &mut delta, &day_clk, &[worker_id.clone()]);
    assert!(c1.contains(&worker_id));
    assert!(
        delta
            .actors
            .get(&worker_id)
            .unwrap()
            .backpack
            .iter()
            .any(|i| i.slug == "timber_log" && i.count >= 1),
        "civic worker must autonomously fell timber when lacking bridge lumber"
    );

    let copper_before = delta.actors.get(&worker_id).unwrap().copper;
    delta.relocate_actor(&worker_id, &bridge_nid, bridge_cid, true);
    let c2 = execute_player_parity_bell_actions(&doc, &mut delta, &day_clk, &[worker_id.clone()]);
    assert!(c2.contains(&worker_id));
    assert!(
        delta
            .live_land
            .cells
            .get(&bridge_cid)
            .unwrap()
            .bridge_integrity
            >= 60,
        "resident must autonomously repair washed-out bridge"
    );
    assert!(!is_river_crossing_impassable_for_wheels(&delta, bridge_cid));
    assert_eq!(
        delta.actors.get(&worker_id).unwrap().copper,
        copper_before + 6
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn town_grain_shortage_drives_farmer_career_shift_and_autonomous_reclamation() {
    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let day_clk = solar_clock_at(epoch + 1_200);
    ensure_live_land_seeded(&doc, &mut delta, day_clk.game_days);

    // 1. 製造白鷗港酒館存糧吃緊（剩 4 份），並對破產改換方針的居民執行自主轉行
    let tav_node = "oi:white_gull_harbor:rusty_anchor_tavern";
    let (_, keeper_opt) = find_town_tavern_and_keeper(&doc, &delta, 4616);
    let keeper_id = keeper_opt.unwrap();
    {
        let ik = delta.actors.get_mut(&keeper_id).unwrap();
        let cur = count_box_home_rations_only(ik, tav_node);
        assert!(deduct_box_home_rations(ik, tav_node, cur - 4));
    }

    let shifter_id = "res:white_gull_harbor:copper_ln_1_0".to_string();
    {
        let a = delta.actors.get_mut(&shifter_id).unwrap();
        let move_c = a.copper;
        a.copper = 0;
        delta.municipal_treasury_copper += move_c;
        a.route_plan.clear();
        a.mind.apply_goal_strategy_with_snapshot(
            GoalStrategyKind::ShiftLivelihood,
            None,
            day_clk.game_days,
            4,
            0,
            false,
        );
    }
    let shifted_claim =
        execute_player_parity_bell_actions(&doc, &mut delta, &day_clk, &[shifter_id.clone()]);
    assert!(shifted_claim.contains(&shifter_id));
    let shifted_actor = delta.actors.get(&shifter_id).unwrap();
    assert!(
        matches!(shifted_actor.role.as_str(), "漁夫" | "農夫"),
        "short-grain town destitute resident should shift to food producer (got: {})",
        shifted_actor.role
    );
    assert!(
        shifted_actor
            .mind
            .ltm
            .turning_points
            .iter()
            .any(|tp| tp.anchor_event_id.starts_with("career:shift:")),
        "career shift must record life turning point"
    );

    // 2. 驗證可墾平坡地塊上的居民能自主翻土墾荒
    let rural_cid = doc
        .cells
        .values()
        .find(|c| {
            c.kind != "settlement"
                && c.height <= 24
                && doc
                    .nodes
                    .values()
                    .any(|n| n.cell_id == c.cell_id && !n.indoor && !n.dungeon)
                && delta.live_land.cells.get(&c.cell_id).is_some_and(|cl| {
                    cl.plague_miasmas < 30 && cl.flood_level == 0 && cl.reclaimed_plots == 0
                })
        })
        .map(|c| c.cell_id)
        .expect("reclaimable rural cell exists");
    let rural_nid = doc
        .nodes
        .iter()
        .find(|(_, n)| n.cell_id == rural_cid && !n.indoor && !n.dungeon)
        .map(|(nid, _)| nid.clone())
        .unwrap();

    let farmer_id = "res:white_gull_harbor:copper_ln_7_0".to_string();
    {
        let f = delta.actors.get_mut(&farmer_id).unwrap();
        f.fatigue = 12;
        f.wound_part = None;
        f.route_plan.clear();
        f.backpack.clear();
        f.mind.apply_goal_strategy_with_snapshot(
            GoalStrategyKind::ReclaimLand,
            None,
            day_clk.game_days,
            4,
            0,
            false,
        );
    }
    delta.relocate_actor(&farmer_id, &rural_nid, rural_cid, true);
    let reclaim_claim =
        execute_player_parity_bell_actions(&doc, &mut delta, &day_clk, &[farmer_id.clone()]);
    assert!(reclaim_claim.contains(&farmer_id));
    assert!(
        delta
            .live_land
            .cells
            .get(&rural_cid)
            .unwrap()
            .reclaimed_plots
            >= 1,
        "resident must autonomously reclaim farmland"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn nine_strategies_parse_and_herbalist_self_foraged_herb_reopens_apothecary() {
    assert_eq!(
        parse_slow_think_chinese_output(
            "定見：城南缺糧，不如往坡地開墾一畦麥田。｜方針：墾荒種糧",
            GoalStrategyKind::HoneCraft
        )
        .expect("ReclaimLand parses")
        .1,
        GoalStrategyKind::ReclaimLand
    );
    assert_eq!(
        parse_slow_think_chinese_output(
            "定見：溪橋沖壞了，先去扛木把橋修牢。｜方針：修橋繕屋",
            GoalStrategyKind::HoneCraft
        )
        .expect("RepairCivicWorks parses")
        .1,
        GoalStrategyKind::RepairCivicWorks
    );
    assert_eq!(
        parse_slow_think_chinese_output(
            "定見：這行掙不到銅板，不如改行下海打魚。｜方針：改換營生",
            GoalStrategyKind::HoneCraft
        )
        .expect("ShiftLivelihood parses")
        .1,
        GoalStrategyKind::ShiftLivelihood
    );

    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let day_clk = solar_clock_at(epoch + 1_200);
    let abs_bell = day_clk.game_days * 12 + i64::from(day_clk.bell_index);
    let herb_node = "oi:white_gull_harbor:herb_hall";
    let herbalist_id = "res:white_gull_harbor:shop_3_0";

    while consume_shop_stock_one(&mut delta, None, herb_node, &APOTHECARY_HERB_SLUGS).is_some() {}
    assert!(is_apothecary_out_of_herbs(&delta, herb_node));

    // 草藥師親自從山林採回一束止血草並帶回藥堂，經由 sell_foraged 直接入自家鋪倉重開包紮！
    delta.relocate_actor(herbalist_id, herb_node, 4616, true);
    add_carry_item(
        &mut delta.actors.get_mut(herbalist_id).unwrap().backpack,
        CarryItem {
            slug: "herb_bundle".to_string(),
            name: "山林止血草束".to_string(),
            count: 1,
            weight_units: 1,
            ..CarryItem::default()
        },
    );
    let restocked = execute_arrived_desire_effect(
        &doc,
        &mut delta,
        &day_clk,
        herbalist_id,
        "sell_foraged",
        None,
        Some("oi:white_gull_harbor:rusty_anchor_tavern"),
        None,
        abs_bell,
    );
    assert!(
        restocked,
        "herbalist carrying foraged herb at own hall must self-restock"
    );
    assert!(!is_apothecary_out_of_herbs(&delta, herb_node));
    assert!(
        apply_world_action(
            &doc,
            &mut delta,
            &day_clk,
            "res:white_gull_harbor:shop_3_1",
            "bandage_wound",
            ""
        )
        .is_ok(),
        "apothecary bandage_wound resumes once self-foraged herb restocks locker"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn thirty_day_coal_drain_triggers_forge_seal_venture_lease_and_roadside_alley_homestead() {
    use super::island_substrate::ROAD_COUNTRY_TRACK;
    use super::place_registry::{allocate_and_register_roadside_homestead, ruin_dynamic_homestead};
    use super::shop_leases::{ShopLeaseState, actor_operates_shop};
    use super::sim_tick::tick_outer_island_world;

    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    ensure_live_land_seeded(&doc, &mut delta, 0);
    // 先跑一次起始世界初始化，再模擬後續煤礦耗盡之真實現況
    tick_outer_island_world(
        &doc,
        &mut delta,
        &solar_clock_at(epoch),
        (epoch as u64) * 1000,
    );

    let forge_node = "oi:white_gull_harbor:bal_forge";
    let smith_id = "res:white_gull_harbor:shop_0_0".to_string();
    let app_id = "res:white_gull_harbor:shop_0_1".to_string();
    let orig_weapon = delta.actors[&smith_id].bound_weapon.clone();

    // 抽乾鍛爐精煤木炭並令學徒離鋪，鐵匠知曉秘境傳聞且手頭吃緊無法單靠懸賞無限續煤
    while consume_shop_stock_one(&mut delta, None, forge_node, &SMITHY_FUEL_SLUGS).is_some() {}
    assert!(is_smithy_forge_cold(&delta, forge_node));
    delta.relocate_actor(&app_id, "oi:white_gull_harbor:plaza", 4616, true);
    let excess = (delta.actors[&smith_id].copper - 14).max(0);
    if excess > 0 {
        assert!(delta.transfer_copper(
            0,
            2,
            "probe_drain",
            &smith_id,
            "municipal_treasury",
            excess,
            "30 日探針模擬盤虧".to_string(),
        ));
    }
    if let Some(s) = delta.actors.get_mut(&smith_id) {
        s.known_rumors.insert("pit_rumor".to_string());
        s.mind.stm.affect.anger = 60;
        s.mark_mutated();
    }

    let mut sealed_seen = false;
    let mut smith_ventured_out = false;
    let mut smith_delved_pit = false;
    for step in 1..=360i64 {
        let now_unix = epoch + step * 600;
        let clk = solar_clock_at(now_unix);
        tick_outer_island_world(&doc, &mut delta, &clk, (now_unix as u64) * 1000);

        if delta
            .shop_leases
            .get(forge_node)
            .is_some_and(|l| l.state == ShopLeaseState::SealedForLease)
        {
            sealed_seen = true;
        }
        if sealed_seen && let Some(s) = delta.actors.get(&smith_id) {
            if s.node_id != forge_node {
                smith_ventured_out = true;
            }
            if s.node_id.starts_with("oi:dng_pit:") || s.node_id == "oi:cell4529:0" {
                smith_delved_pit = true;
            }
        }
        if sealed_seen && smith_ventured_out && smith_delved_pit {
            break;
        }
    }

    assert!(
        sealed_seen,
        "empty-coal smith must seal forge for lease within 30 days"
    );
    assert!(
        smith_ventured_out,
        "sealed smith must step outdoors rather than stay glued to counter"
    );
    let lease = delta
        .shop_leases
        .get(forge_node)
        .expect("forge lease created");
    assert_eq!(lease.owner_actor_id, smith_id);

    // 確認全程從未免費發過工具：原鐵匠依然緊握原有鍛錘
    let smith_after = &delta.actors[&smith_id];
    assert_eq!(smith_after.bound_weapon.uid, orig_weapon.uid);
    assert_eq!(smith_after.bound_weapon.name, orig_weapon.name);
    assert!(
        smith_ventured_out
            && (smith_delved_pit || !actor_operates_shop(&delta, &smith_id, forge_node)),
        "sealed former shopkeeper unlocks counter lock and can pursue outdoor venture/delve"
    );

    // 2. 驗證店鋪承租頂讓（若 30 日內已有自由居民自主頂下，核對其主事權與原鐵匠不變；否則以有資歷居民手動付費頂下並檢查 100% 銅幣守恆）
    let lessee_id = "res:white_gull_harbor:artisan_1_0".to_string();
    let mut taker_id = if delta.actors.contains_key(&lessee_id) {
        lessee_id
    } else {
        app_id.clone()
    };
    let clk_take = solar_clock_at(epoch + 30 * 7200 + 2400);
    if let Some(op_id) = delta.shop_leases[forge_node].operator_actor_id.clone() {
        assert_eq!(delta.shop_leases[forge_node].state, ShopLeaseState::Active);
        assert_ne!(
            op_id, smith_id,
            "auto-lessee must not silently become old smith"
        );
        assert!(actor_operates_shop(&delta, &op_id, forge_node));
        assert_eq!(delta.conservation_diff(&doc), 0);
        taker_id = op_id;
    } else {
        let need_topup = (32 - delta.actors[&taker_id].copper).max(0);
        if need_topup > 0 {
            assert!(delta.transfer_copper(
                30,
                4,
                "probe_fund",
                "municipal_treasury",
                &taker_id,
                need_topup,
                "承租資金測試撥備".to_string(),
            ));
        }
        let fee = delta.shop_leases[forge_node].transfer_fee_copper;
        let owner_before = delta.actors[&smith_id].copper;
        dispatch_extended_shared_verb(
            &doc,
            &mut delta,
            &clk_take,
            &taker_id,
            "lease_vacant_shop",
            forge_node,
        )
        .expect("qualified resident pays transfer fee to lease vacant shop");
        assert_eq!(delta.actors[&smith_id].copper, owner_before + fee);
        assert!(actor_operates_shop(&delta, &taker_id, forge_node));
        assert_eq!(delta.conservation_diff(&doc), 0);
    }

    // 顯式封爐與過戶雙向檢查：草藥堂當家封爐不領新工具 -> 他人付費承租過戶且銅幣零誤差
    let herb_node = "oi:white_gull_harbor:herb_hall";
    let herb_keeper = "res:white_gull_harbor:shop_3_0";
    let herb_app = "res:white_gull_harbor:shop_3_1";
    let herb_weapon_uid = delta.actors[herb_keeper].bound_weapon.uid.clone();
    if actor_operates_shop(&delta, herb_keeper, herb_node) {
        dispatch_extended_shared_verb(
            &doc,
            &mut delta,
            &clk_take,
            herb_keeper,
            "seal_shop_venture",
            herb_node,
        )
        .expect("apothecary operator can seal shop for lease");
        assert_eq!(delta.actors[herb_keeper].bound_weapon.uid, herb_weapon_uid);
        assert!(!actor_operates_shop(&delta, herb_keeper, herb_node));
        let herb_need = (24 - delta.actors[herb_app].copper).max(0);
        if herb_need > 0 {
            assert!(delta.transfer_copper(
                30,
                4,
                "probe_herb_fund",
                "municipal_treasury",
                herb_app,
                herb_need,
                "藥堂頂讓測試撥備".to_string(),
            ));
        }
        let owner_copper_before = delta.actors[herb_keeper].copper;
        let hfee = delta.shop_leases[herb_node].transfer_fee_copper;
        dispatch_extended_shared_verb(
            &doc,
            &mut delta,
            &clk_take,
            herb_app,
            "lease_vacant_shop",
            herb_node,
        )
        .expect("apprentice/buyer pays transfer fee to lease apothecary");
        assert_eq!(delta.actors[herb_keeper].copper, owner_copper_before + hfee);
        assert!(actor_operates_shop(&delta, herb_app, herb_node));
        assert_eq!(delta.conservation_diff(&doc), 0);
    }

    // 3. 驗證路邊雙格融合屋基與滿額自動延伸 3..=5 格支巷，以及只增不減廢墟標記
    let mut saw_extended_alley = false;
    let mut created_ids = Vec::new();
    for idx in 0..16u32 {
        let pid = format!("homestead:probe:{idx}");
        let rec = allocate_and_register_roadside_homestead(
            &doc,
            &mut delta,
            31,
            &taker_id,
            &pid,
            "oi:white_gull_harbor:iron_anvil_ln",
            &format!("巷側工坊新宅第{idx}"),
        )
        .expect("roadside or extended-alley homestead must allocate cleanly");
        assert!(!rec.footprint_tiles.is_empty());
        if !rec.extended_road_tiles.is_empty() {
            assert!((3..=5).contains(&rec.extended_road_tiles.len()));
            for &[rx, ry] in &rec.extended_road_tiles {
                let pos = usize::from(ry) * super::island_substrate::GRID_W + usize::from(rx);
                assert_eq!(delta.tile_diffs[&(pos as u32)].road, ROAD_COUNTRY_TRACK);
            }
            saw_extended_alley = true;
        }
        created_ids.push(pid);
    }
    assert!(
        saw_extended_alley,
        "crowded roadside must auto-extend 3..=5 tile country alley"
    );
    let first_id = created_ids[0].clone();
    assert!(ruin_dynamic_homestead(&mut delta, &first_id, 32));
    assert!(
        delta.place_registry.contains_key(&first_id),
        "append-only registry never deletes ID"
    );
    assert!(delta.place_registry[&first_id].ruined);
    assert_eq!(delta.conservation_diff(&doc), 0);
}

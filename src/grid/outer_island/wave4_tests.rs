//! 第四刀整合單元測試：模擬人生生活感、農田藥圃開墾收成閉環、野外巨獸蹤跡追獵與城鎮動態活事件鏈。

use super::actions::apply_world_action;
use super::clock_and_lang::solar_clock_at;
use super::engine::load_static_world_doc;
use super::living_sim_and_farm::is_farmable_node;
use super::movement::apply_setup_origin;
use super::types::{IslandWorldDelta, WorldStaticDoc};
use super::views::build_view_json;

fn seeded_delta_with_player(doc: &WorldStaticDoc, pid: &str, name: &str) -> IslandWorldDelta {
    let mut delta = IslandWorldDelta {
        genesis_seeded: true,
        genesis_total_copper: doc.ledger_genesis.municipal_treasury_copper
            + doc.ledger_genesis.silver_bank_reserve_copper
            + 60,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    let clock = solar_clock_at(1_764_300_000);
    apply_setup_origin(
        doc,
        &mut delta,
        &clock,
        pid,
        name,
        "人類",
        "披著舊斗篷",
        "smith_apprentice",
    )
    .expect("setup player");
    let aid = delta.resolve_actor_id(pid).expect("aid");
    if let Some(a) = delta.actors.get_mut(&aid) {
        a.copper = 60;
        a.fatigue = 55;
        a.mind.stm.affect.hunger = 60;
        a.inn_nights_left = 2;
    }
    delta
}

#[test]
fn living_sim_hot_meal_bath_outfit_furnishing_conserve_copper_and_update_appearance() {
    let doc = load_static_world_doc();
    let mut delta = seeded_delta_with_player(&doc, "w4_life", "阿晴");
    let clock = solar_clock_at(1_764_300_000);
    let aid = delta.resolve_actor_id("w4_life").expect("aid");

    // 移動至白鷗市集館（可吃熱食、沐浴、換穿搭、購居所傢俱）
    let market_nd = doc
        .nodes
        .get("oi:white_gull_harbor:market_avenue")
        .expect("market_avenue");
    delta.relocate_actor(&aid, &market_nd.id, market_nd.cell_id, true);

    let meal_msg = apply_world_action(&doc, &mut delta, &clock, "w4_life", "enjoy_hot_meal", "")
        .expect("hot meal ok");
    assert!(meal_msg.contains("熱湯") || meal_msg.contains("濃湯"));
    assert_eq!(delta.conservation_diff(&doc), 0);

    let bath_msg = apply_world_action(&doc, &mut delta, &clock, "w4_life", "take_warm_bath", "")
        .expect("warm bath ok");
    assert!(bath_msg.contains("溫水"));
    assert_eq!(delta.conservation_diff(&doc), 0);

    let outfit_msg = apply_world_action(
        &doc,
        &mut delta,
        &clock,
        "w4_life",
        "change_outfit",
        "waxed_hunter",
    )
    .expect("change outfit ok");
    assert!(outfit_msg.contains("防風蠟布機能獵裝"));
    assert_eq!(delta.conservation_diff(&doc), 0);
    assert!(
        delta
            .actors
            .get(&aid)
            .expect("actor")
            .appearance
            .contains("防風蠟布")
    );

    let furn_msg = apply_world_action(
        &doc,
        &mut delta,
        &clock,
        "w4_life",
        "buy_home_furnishing",
        "warm_brazier",
    )
    .expect("buy furnishing ok");
    assert!(furn_msg.contains("鑄鐵烘衣炭火盆"));
    assert_eq!(delta.conservation_diff(&doc), 0);

    let viewer = delta.actors.get(&aid).expect("viewer");
    let view = build_view_json(&doc, &delta, viewer, &clock);
    let ls = &view["player"]["lifestyle"];
    assert_eq!(ls["outfit_name"].as_str(), Some("防風蠟布機能獵裝"));
    assert!(
        ls["furnishings"]
            .as_array()
            .is_some_and(|arr| arr.iter().any(|v| v.as_str() == Some("鑄鐵烘衣炭火盆")))
    );
    // 確認去數值化
    for key in ["cleanliness_sense", "meal_sense", "outfit_name"] {
        let s = ls[key].as_str().unwrap_or("");
        assert!(!s.chars().any(|c| c.is_ascii_digit()), "leak in {key}: {s}");
    }
}

#[test]
fn farm_plot_sow_tend_and_harvest_full_cycle() {
    let doc = load_static_world_doc();
    let mut delta = seeded_delta_with_player(&doc, "w4_farm", "老墾");
    let day0 = solar_clock_at(1_764_300_000);
    let aid = delta.resolve_actor_id("w4_farm").expect("aid");

    let farm_nd = doc
        .nodes
        .values()
        .find(|n| is_farmable_node(&doc, n))
        .expect("farmable outdoor node");
    delta.relocate_actor(&aid, &farm_nd.id, farm_nd.cell_id, true);

    let sow_msg = apply_world_action(
        &doc,
        &mut delta,
        &day0,
        "w4_farm",
        "sow_farm_plot",
        "herb_garden",
    )
    .expect("sow ok");
    assert!(sow_msg.contains("山林止血藥圃"));
    assert_eq!(delta.farm_plots.len(), 1);
    assert!(
        delta
            .live_land
            .cells
            .get(&farm_nd.cell_id)
            .is_some_and(|cl| cl.reclaimed_plots >= 1)
    );

    // 照料一次提早一日熟成
    let tend_msg =
        apply_world_action(&doc, &mut delta, &day0, "w4_farm", "tend_farm_plot", "")
            .expect("tend ok");
    assert!(tend_msg.contains("山林止血藥圃"));

    // 前進一個遊戲日熟成收割
    let mut day1 = day0.clone();
    day1.game_days += 1;
    let harv_msg =
        apply_world_action(&doc, &mut delta, &day1, "w4_farm", "harvest_farm_plot", "")
            .expect("harvest ok");
    assert!(harv_msg.contains("山林止血草束"));
    assert!(delta.farm_plots.is_empty());
    let herb_count: u32 = delta
        .actors
        .get(&aid)
        .expect("actor")
        .backpack
        .iter()
        .filter(|i| i.slug == "herb_bundle")
        .map(|i| i.count)
        .sum();
    assert!(herb_count >= 2, "expected >= 2 herb_bundle, got {herb_count}");
}

#[test]
fn wild_beast_trail_tracking_and_town_live_events_conserve_copper() {
    let doc = load_static_world_doc();
    let mut delta = seeded_delta_with_player(&doc, "w4_hunt", "巡獵者");
    let clock = solar_clock_at(1_764_300_000);
    let aid = delta.resolve_actor_id("w4_hunt").expect("aid");

    // 1. 港務北棧觸發並完成海潮搶護船貨活事件，檢查銅幣守恆與同日防刷
    let ev1 = apply_world_action(
        &doc,
        &mut delta,
        &clock,
        "w4_hunt",
        "resolve_town_event",
        "harbor_tide_salvage",
    )
    .expect("harbor event ok");
    assert!(ev1.contains("5 枚銅幣"));
    assert_eq!(delta.conservation_diff(&doc), 0);

    let ev1_dup = apply_world_action(
        &doc,
        &mut delta,
        &clock,
        "w4_hunt",
        "resolve_town_event",
        "harbor_tide_salvage",
    )
    .expect("dup guarded cleanly");
    assert!(ev1_dup.contains("今日你已盡過心力"));

    // 2. 市集廣場參與巡防巨獸急報
    let plaza = doc
        .nodes
        .get("oi:white_gull_harbor:plaza")
        .expect("plaza");
    delta.relocate_actor(&aid, &plaza.id, plaza.cell_id, true);
    let ev_watch = apply_world_action(
        &doc,
        &mut delta,
        &clock,
        "w4_hunt",
        "resolve_town_event",
        "watch_bounty_brief",
    )
    .expect("watch brief ok");
    assert!(ev_watch.contains("循跡追索"));
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 3. 野外循跡追獵與抵達巢穴正面觸發臨陣對峙
    let wild_nd = doc
        .nodes
        .values()
        .find(|n| n.kind == "wild" && !n.indoor && !n.dungeon)
        .expect("wild node");
    delta.relocate_actor(&aid, &wild_nd.id, wild_nd.cell_id, true);
    let track_msg = apply_world_action(&doc, &mut delta, &clock, "w4_hunt", "track_wild_beast", "")
        .expect("track beast ok");
    assert!(track_msg.contains("蹤跡") || track_msg.contains("臨陣對峙"));

    let apex_cell = doc
        .cells
        .values()
        .find(|c| c.apex_beast.is_some())
        .expect("apex cell");
    let lair_nd = doc
        .nodes
        .values()
        .find(|n| n.cell_id == apex_cell.cell_id && !n.indoor && !n.dungeon)
        .expect("apex lair node");
    delta.relocate_actor(&aid, &lair_nd.id, lair_nd.cell_id, true);
    let engage_msg =
        apply_world_action(&doc, &mut delta, &clock, "w4_hunt", "track_wild_beast", "")
            .expect("engage apex ok");
    assert!(engage_msg.contains("臨陣對峙"));
    assert!(
        delta
            .actors
            .get(&aid)
            .expect("actor")
            .active_skirmish
            .is_some()
    );
}

//! 第二刀驗收測試：鍛爐耗煤熄火與補炭重燃、藥堂耗草停業與補草復診、酒館存糧漲價與誠實竹簍對齊、斷橋阻糧、海港每日限量卸箱實物入庫。

use super::actions::apply_world_action;
use super::clock_and_lang::solar_clock_at;
use super::engine::load_static_world_doc;
use super::household_economy::{
    count_box_home_rations_only, deduct_box_home_rations, ensure_household_larders_seeded_once,
    find_town_tavern_and_keeper,
};
use super::live_commodities::{
    APOTHECARY_HERB_SLUGS, SMITHY_FUEL_SLUGS, can_deliver_surplus_to_town, consume_shop_stock_one,
    count_apothecary_herbs, count_shop_stock_by_slugs, count_smithy_fuel,
    is_apothecary_out_of_herbs, is_smithy_forge_cold, quote_tavern_ration_copper,
    quote_weapon_repair_copper, route_unloaded_freight_to_town_shops,
};
use super::movement::{add_carry_item, apply_setup_origin};
use super::ongoing_activity::{
    ensure_dock_cargo_piles_initialized, settle_dock_haul_delivery, start_claim_dock_haul,
};
use super::rural_migrate::nearest_core_town_cell_id;
use super::sim_tick::seed_genesis_actors_once;
use super::types::{CarryItem, IslandWorldDelta, WorldStaticDoc};
use super::views::build_view_json;

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
fn cold_forge_blocks_hot_work_until_refueled_and_raises_coal_buyback() {
    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let day_clk = solar_clock_at(epoch + 1_200);

    let forge_node = "oi:white_gull_harbor:bal_forge";
    assert!(
        count_smithy_fuel(&delta, forge_node) >= 14,
        "genesis smithy must hold initial coal + charcoal"
    );

    apply_setup_origin(
        &doc,
        &mut delta,
        &day_clk,
        "coal_tester",
        "焦臉亞當",
        "人類",
        "肩扛煤筐",
        "smith_apprentice",
    )
    .expect("setup actor");
    let pid = delta.resolve_actor_id("coal_tester").expect("player id");
    {
        let p = delta.actors.get_mut(&pid).unwrap();
        p.copper = 30;
        delta.municipal_treasury_copper -= 30;
        p.bound_weapon.wear_stage = 2;
        add_carry_item(
            &mut p.backpack,
            CarryItem {
                slug: "black_coal".to_string(),
                name: "北林精煤塊".to_string(),
                count: 1,
                weight_units: 4,
                ..CarryItem::default()
            },
        );
        p.mark_mutated();
    }
    delta.relocate_actor(&pid, forge_node, 4616, true);
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 清空鍛鐵鋪全部燃料至 0
    while consume_shop_stock_one(&mut delta, None, forge_node, &SMITHY_FUEL_SLUGS).is_some() {}
    assert_eq!(count_smithy_fuel(&delta, forge_node), 0);
    assert!(is_smithy_forge_cold(&delta, forge_node));

    // 確認視圖描述出現熄火文字、熱爐籤條隱藏，且精煤收購價上調為 4 + 2 = 6 銅
    let cold_view = build_view_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &day_clk);
    let desc = cold_view["scene"]["desc"].as_str().unwrap_or("");
    assert!(
        desc.contains("火炭已熄"),
        "cold forge scene desc must describe extinguished hearth: {desc}"
    );
    let npcs = cold_view["scene"]["npcs"].as_array().unwrap();
    let has_repair_deed = npcs.iter().any(|npc| {
        npc["deeds"]
            .as_array()
            .is_some_and(|ds| ds.iter().any(|d| d["id"] == "repair_weapon"))
    });
    assert!(!has_repair_deed, "cold forge must hide hot-work deeds");

    let coal_sell_deed = npcs
        .iter()
        .find_map(|npc| {
            npc["deeds"].as_array().and_then(|ds| {
                ds.iter()
                    .find(|d| {
                        d["param"]
                            .as_str()
                            .is_some_and(|p| p.contains("|black_coal|"))
                    })
                    .cloned()
            })
        })
        .expect("smith must offer coal buyback deed");
    assert_eq!(
        coal_sell_deed["param"].as_str().unwrap().split('|').nth(2),
        Some("6"),
        "cold forge urgent coal buyback must rise from 4 to 6 copper"
    );

    assert!(
        apply_world_action(
            &doc,
            &mut delta,
            &day_clk,
            "coal_tester",
            "repair_weapon",
            ""
        )
        .is_err(),
        "cold forge must reject repair_weapon"
    );
    assert!(
        apply_world_action(&doc, &mut delta, &day_clk, "coal_tester", "forge_help", "").is_err(),
        "cold forge must reject forge_help"
    );

    // 出售 1 塊精煤重開爐火
    let sell_param = coal_sell_deed["param"].as_str().unwrap().to_string();
    apply_world_action(
        &doc,
        &mut delta,
        &day_clk,
        "coal_tester",
        "sell_specialty",
        &sell_param,
    )
    .expect("selling coal to smithy succeeds");
    assert_eq!(delta.actors.get(&pid).unwrap().copper, 36);
    assert_eq!(count_smithy_fuel(&delta, forge_node), 1);
    assert!(!is_smithy_forge_cold(&delta, forge_node));
    assert_eq!(quote_weapon_repair_copper(&delta, forge_node), 6);

    apply_world_action(
        &doc,
        &mut delta,
        &day_clk,
        "coal_tester",
        "repair_weapon",
        "",
    )
    .expect("rekindled forge repairs weapon consuming the 1 coal");
    assert_eq!(delta.actors.get(&pid).unwrap().copper, 30);
    assert_eq!(delta.actors.get(&pid).unwrap().bound_weapon.wear_stage, 0);
    assert_eq!(count_smithy_fuel(&delta, forge_node), 0);
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn apothecary_herb_exhaustion_suspends_bandaging_until_restocked() {
    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let day_clk = solar_clock_at(epoch + 1_200);
    let herb_node = "oi:white_gull_harbor:herb_hall";

    assert!(count_apothecary_herbs(&delta, herb_node) >= 10);

    apply_setup_origin(
        &doc,
        &mut delta,
        &day_clk,
        "herb_tester",
        "採藥托比",
        "人類",
        "右腕裹麻布",
        "smith_apprentice",
    )
    .expect("setup herb tester");
    let pid = delta.resolve_actor_id("herb_tester").expect("player id");
    {
        let p = delta.actors.get_mut(&pid).unwrap();
        p.copper = 20;
        delta.municipal_treasury_copper -= 20;
        p.wound_part = Some("右腕".to_string());
        add_carry_item(
            &mut p.backpack,
            CarryItem {
                slug: "herb_bundle".to_string(),
                name: "山林止血草束".to_string(),
                count: 1,
                weight_units: 1,
                ..CarryItem::default()
            },
        );
        p.mark_mutated();
    }
    delta.relocate_actor(&pid, herb_node, 4616, true);

    while consume_shop_stock_one(&mut delta, None, herb_node, &APOTHECARY_HERB_SLUGS).is_some() {}
    assert!(is_apothecary_out_of_herbs(&delta, herb_node));
    assert!(
        apply_world_action(
            &doc,
            &mut delta,
            &day_clk,
            "herb_tester",
            "bandage_wound",
            ""
        )
        .is_err(),
        "empty apothecary must refuse bandaging"
    );

    let view = build_view_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &day_clk);
    let herb_sell_param = view["scene"]["npcs"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|npc| {
            npc["deeds"].as_array().and_then(|ds| {
                ds.iter()
                    .find(|d| {
                        d["param"]
                            .as_str()
                            .is_some_and(|p| p.contains("|herb_bundle|"))
                    })
                    .and_then(|d| d["param"].as_str().map(str::to_string))
            })
        })
        .expect("apothecary must quote herb_bundle");
    assert_eq!(
        herb_sell_param.split('|').nth(2),
        Some("6"),
        "herb_bundle buyback must rise from 4 to 6 when apothecary is dry"
    );

    apply_world_action(
        &doc,
        &mut delta,
        &day_clk,
        "herb_tester",
        "sell_specialty",
        &herb_sell_param,
    )
    .expect("sell herb bundle to restock apothecary");
    assert_eq!(delta.actors.get(&pid).unwrap().copper, 26);
    assert_eq!(count_apothecary_herbs(&delta, herb_node), 1);

    apply_world_action(
        &doc,
        &mut delta,
        &day_clk,
        "herb_tester",
        "bandage_wound",
        "",
    )
    .expect("bandaging works once restocked");
    assert!(delta.actors.get(&pid).unwrap().wound_part.is_none());
    assert_eq!(delta.actors.get(&pid).unwrap().copper, 21);
    assert_eq!(count_apothecary_herbs(&delta, herb_node), 0);
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn bridge_washout_cuts_rural_grain_and_drives_tavern_price_spike() {
    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let day_clk = solar_clock_at(epoch + 1_200);
    let tav_node = "oi:white_gull_harbor:rusty_anchor_tavern";
    let (_, keeper_id_opt) = find_town_tavern_and_keeper(&doc, &delta, 4616);
    let keeper_id = keeper_id_opt.expect("white gull tavern keeper");

    apply_setup_origin(
        &doc,
        &mut delta,
        &day_clk,
        "grain_tester",
        "麥袋瑪莎",
        "人類",
        "手拎乾糧袋",
        "smith_apprentice",
    )
    .expect("setup grain tester");
    let pid = delta.resolve_actor_id("grain_tester").expect("player id");
    {
        let p = delta.actors.get_mut(&pid).unwrap();
        p.copper = 30;
        delta.municipal_treasury_copper -= 30;
        p.mark_mutated();
    }
    delta.relocate_actor(&pid, tav_node, 4616, true);

    // 1. 充裕（30 份）-> 2 銅；吃緊（10 份）-> 3 銅；見底（3 份）-> 4 銅
    assert_eq!(quote_tavern_ration_copper(&doc, &delta, 4616, tav_node), 2);
    {
        let ik = delta.actors.get_mut(&keeper_id).unwrap();
        let cur = count_box_home_rations_only(ik, tav_node);
        assert!(cur >= 20);
        assert!(deduct_box_home_rations(ik, tav_node, cur - 10));
    }
    assert_eq!(quote_tavern_ration_copper(&doc, &delta, 4616, tav_node), 3);

    // 移開掌櫃與學徒，確認誠實竹簍物件報價與 buy_ration 實扣 1:1 吻合
    let staff: Vec<String> = delta
        .node_occupants
        .get(tav_node)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|id| {
            id != &pid
                && delta
                    .actors
                    .get(id)
                    .is_some_and(|a| a.role.contains("酒館掌櫃"))
        })
        .collect();
    for sid in &staff {
        delta.relocate_actor(sid, "oi:white_gull_harbor:plaza", 4616, true);
    }
    let view = build_view_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &day_clk);
    let basket_param = view["scene"]["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["object_id"] == "obj:ration_basket")
        .and_then(|o| o["options"][0]["param"].as_str().map(str::to_string))
        .expect("self_basket must be visible");
    assert_eq!(basket_param, "self_basket|3");
    apply_world_action(
        &doc,
        &mut delta,
        &day_clk,
        "grain_tester",
        "buy_ration",
        &basket_param,
    )
    .expect("honesty basket uses live price 3");
    assert_eq!(delta.actors.get(&pid).unwrap().copper, 27);

    // 2. 驗證貨運卸入大宗穀袋真實轉為酒館口糧
    {
        let ik = delta.actors.get_mut(&keeper_id).unwrap();
        let cur = count_box_home_rations_only(ik, tav_node);
        assert!(deduct_box_home_rations(ik, tav_node, cur - 3));
    }
    assert_eq!(quote_tavern_ration_copper(&doc, &delta, 4616, tav_node), 4);
    route_unloaded_freight_to_town_shops(
        &doc,
        &mut delta,
        4616,
        vec![("wheat_grain".to_string(), "飽滿黑麥穀袋".to_string(), 5)],
    );
    let after_stock = count_box_home_rations_only(delta.actors.get(&keeper_id).unwrap(), tav_node);
    assert_eq!(
        after_stock, 18,
        "5 wheat_grain sacks must bake 15 home_ration into tavern"
    );
    assert_eq!(quote_tavern_ration_copper(&doc, &delta, 4616, tav_node), 2);

    // 3. 驗證河橋沖毀後，對岸鄉野餘糧無法瞬移供應本鎮酒館
    let remote_rural = doc
        .cells
        .values()
        .find(|c| {
            c.kind == "rural_settlement" && nearest_core_town_cell_id(&doc, c.cell_id) == 4616
        })
        .expect("rural cell attached to white gull");
    let rcid = remote_rural.cell_id;
    assert!(can_deliver_surplus_to_town(&doc, &delta, rcid, 4616));
    let mut blocked_ids = Vec::new();
    for (cid, cl) in &mut delta.live_land.cells {
        if *cid != 4616 && cl.has_river_crossing {
            cl.bridge_integrity = 0;
            blocked_ids.push(*cid);
        }
    }
    assert!(!blocked_ids.is_empty());
    // 若沖毀該村自身或全區過河橋樑，跨河供糧鏈須真實中斷
    if let Some(cl) = delta.live_land.cells.get_mut(&rcid) {
        cl.has_river_crossing = true;
        cl.bridge_integrity = 0;
    }
    assert!(!can_deliver_surplus_to_town(&doc, &delta, rcid, 4616));
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn port_dock_barge_refills_at_most_once_per_day_and_delivers_coal_plus_iron() {
    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let day0_clk = solar_clock_at(epoch + 1_200);
    let dock_nid = "oi:white_gull_harbor:dock";
    let forge_node = "oi:white_gull_harbor:bal_forge";

    apply_setup_origin(
        &doc,
        &mut delta,
        &day0_clk,
        "dock_tester",
        "碼頭阿力",
        "人類",
        "粗麻短褂",
        "dock_hand",
    )
    .expect("setup dock tester");
    let pid = delta.resolve_actor_id("dock_tester").expect("player id");
    let dock_nd = doc.nodes.get(dock_nid).unwrap();

    let init_fuel = count_smithy_fuel(&delta, forge_node);
    let init_iron = count_shop_stock_by_slugs(&delta, forge_node, &["iron_ore"]);

    ensure_dock_cargo_piles_initialized(&doc, &mut delta, day0_clk.game_days);
    assert_eq!(
        delta.dock_cargo_piles.get(dock_nid).map_or(0, Vec::len),
        3,
        "port begins with 3 daily dock orders"
    );

    // 同日領走並送達全部 3 個貨箱
    for _ in 0..3 {
        delta.relocate_actor(&pid, dock_nid, 4616, true);
        start_claim_dock_haul(&doc, &mut delta, &day0_clk, &pid, dock_nd)
            .expect("claim dock haul succeeds");
        let target_shop = delta
            .actors
            .get(&pid)
            .and_then(|a| a.ongoing_activity.as_ref().map(|o| o.target_node.clone()))
            .unwrap();
        delta.relocate_actor(&pid, &target_shop, 4616, true);
        settle_dock_haul_delivery(&doc, &mut delta, &day0_clk, &pid)
            .expect("deliver dock crate succeeds");
    }

    assert_eq!(
        count_shop_stock_by_slugs(&delta, forge_node, &["iron_ore"]),
        init_iron + 3,
        "forge crate must deliver 3 iron_ore"
    );
    assert_eq!(
        count_smithy_fuel(&delta, forge_node),
        init_fuel + 3,
        "forge coal+iron crate must deliver 3 black_coal"
    );

    // 同日再領第 4 箱必須被拒（海駁每日限一班，不可同日無限憑空變貨箱！）
    delta.relocate_actor(&pid, dock_nid, 4616, true);
    assert!(
        start_claim_dock_haul(&doc, &mut delta, &day0_clk, &pid, dock_nd).is_err(),
        "port must not infinitely respawn dock crates on the same game day"
    );

    // 跨入下一遊戲日後新海駁靠岸才補新箱
    let day1_clk = solar_clock_at(epoch + 1_200 + 3_600);
    assert!(day1_clk.game_days > day0_clk.game_days);
    start_claim_dock_haul(&doc, &mut delta, &day1_clk, &pid, dock_nd)
        .expect("next-day barge brings fresh cargo orders");
    assert_eq!(delta.conservation_diff(&doc), 0);
}

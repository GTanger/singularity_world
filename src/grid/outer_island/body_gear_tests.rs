//! 第二刀回歸測試：全身多部位裝備物理效應（負重擴容／涉水省力／防反震護腕）、零門檻拾材綁製、百工自由造物與 4B 路名鎖定。

use crate::grid::outer_island::actions::apply_world_action;
use crate::grid::outer_island::body_gear_craft::{
    arms_absorbs_wrist_recoil, effective_overland_bulk_cap, feet_fatigue_reduction,
};
use crate::grid::outer_island::clock_and_lang::solar_clock_for_settled_bell;
use crate::grid::outer_island::engine::load_static_world_doc;
use crate::grid::outer_island::movement::{add_carry_item, apply_setup_origin};
use crate::grid::outer_island::sim_tick::seed_genesis_actors_once;
use crate::grid::outer_island::types::{CarryItem, IslandWorldDelta};

#[test]
fn zero_gate_raw_gather_field_bind_and_body_gear_physics_work_end_to_end() {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    let day_clock = solar_clock_for_settled_bell(120 * 12 + 5, 1_750_000_000);
    seed_genesis_actors_once(&doc, &mut delta, false);

    apply_setup_origin(
        &doc,
        &mut delta,
        &day_clock,
        "p_gear",
        "阿木",
        "人類",
        "粗麻衫",
        "smith_apprentice",
    )
    .expect("setup");

    // 找一個野外節點測試零門檻徒手拾取原材
    let wild_node = doc
        .nodes
        .values()
        .find(|n| !n.indoor && !n.dungeon && n.kind == "wild")
        .expect("wild node");
    delta.relocate_actor("pl:p_gear", &wild_node.id, wild_node.cell_id, true);

    let g_msg = apply_world_action(&doc, &mut delta, &day_clock, "p_gear", "gather_wild_raw", "")
        .expect("gather raw");
    assert!(g_msg.contains("河灘硬石"), "應取得天然原材: {g_msg}");

    // 補多一份枯枝與韌藤後，測試就地綁製護腕、涉水履與採集背架
    {
        let a = delta.actors.get_mut("pl:p_gear").expect("actor");
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "tough_vine".to_string(),
                name: "韌藤纖維".to_string(),
                count: 4,
                weight_units: 1,
                ..CarryItem::default()
            },
        );
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "dry_branch".to_string(),
                name: "枯枝木桿".to_string(),
                count: 3,
                weight_units: 1,
                ..CarryItem::default()
            },
        );
    }

    assert_eq!(effective_overland_bulk_cap(&delta.actors["pl:p_gear"]), 2);
    assert!(!arms_absorbs_wrist_recoil(&delta.actors["pl:p_gear"]));
    assert_eq!(feet_fatigue_reduction(&delta.actors["pl:p_gear"], true), 0);

    apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_gear",
        "field_bind_craft",
        "vine_bracers",
    )
    .expect("bind bracers");
    apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_gear",
        "field_bind_craft",
        "reed_boots",
    )
    .expect("bind boots");
    apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_gear",
        "field_bind_craft",
        "vine_packframe",
    )
    .expect("bind packframe");

    apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_gear",
        "equip_body_gear",
        "gear_vine_bracers",
    )
    .expect("equip arms");
    apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_gear",
        "equip_body_gear",
        "gear_reed_boots",
    )
    .expect("equip feet");
    apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_gear",
        "equip_body_gear",
        "gear_vine_packframe",
    )
    .expect("equip torso");

    let p = &delta.actors["pl:p_gear"];
    assert!(arms_absorbs_wrist_recoil(p), "粗藤護腕應啟動防手腕反震");
    assert!(feet_fatigue_reduction(p, true) >= 2, "編藤涉水履應減免涉水疲勞");
    assert_eq!(effective_overland_bulk_cap(p), 3, "藤編採集背架應擴增跨格負重上限至 3");

    // 卸下守恆驗證：原物退回行囊且負重恢復 2
    apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_gear",
        "unequip_body_gear",
        "torso",
    )
    .expect("unequip torso");
    assert_eq!(effective_overland_bulk_cap(&delta.actors["pl:p_gear"]), 2);
    assert!(
        delta.actors["pl:p_gear"]
            .backpack
            .iter()
            .any(|i| i.slug == "gear_vine_packframe" && i.count == 1),
        "卸下的背架必須守恆退回行囊"
    );
}

#[test]
fn workshop_custom_craft_scales_with_material_physics_and_conserves_copper() {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    let day_clock = solar_clock_for_settled_bell(120 * 12 + 5, 1_750_000_000);
    seed_genesis_actors_once(&doc, &mut delta, false);
    crate::grid::outer_island::live_commodities::ensure_shop_material_stocks_seeded(&doc, &mut delta);

    apply_setup_origin(
        &doc,
        &mut delta,
        &day_clock,
        "p_smith",
        "鐵工客",
        "人類",
        "皮圍裙",
        "smith_apprentice",
    )
    .expect("setup");

    let smithy_id = "oi:white_gull_harbor:bal_forge";
    delta.relocate_actor("pl:p_smith", smithy_id, 4616, true);
    // 確保在場有當值鐵匠並給予測試者 10 銅與 1 份巨蛛甲片
    let smith_aid = delta
        .actors
        .iter()
        .find(|(_, a)| a.alive && a.controller_id.is_none() && a.role.contains("鍛鐵"))
        .map(|(k, _)| k.clone())
        .expect("smith");
    delta.relocate_actor(&smith_aid, smithy_id, 4616, true);
    assert!(delta.transfer_copper(
        day_clock.game_days,
        day_clock.bell_index,
        "test_seed",
        "municipal_treasury",
        "pl:p_smith",
        10,
        "測試本金".to_string(),
    ));
    add_carry_item(
        &mut delta.actors.get_mut("pl:p_smith").expect("p").backpack,
        CarryItem {
            slug: "spider_chitin".to_string(),
            name: "吞石巨蛛甲片".to_string(),
            count: 1,
            weight_units: 2,
            ..CarryItem::default()
        },
    );

    let total_before: i64 = delta.municipal_treasury_copper
        + delta.silver_bank_reserve_copper
        + delta.actors.values().map(|a| a.copper).sum::<i64>();

    let msg = apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_smith",
        "craft_custom_gear",
        "cargo_frame|spider_chitin",
    )
    .expect("craft spider_chitin frame");
    assert!(msg.contains("蛛甲模組採集背架"), "應依形制×主材生成專屬名號: {msg}");

    let total_after: i64 = delta.municipal_treasury_copper
        + delta.silver_bank_reserve_copper
        + delta.actors.values().map(|a| a.copper).sum::<i64>();
    assert_eq!(total_before, total_after, "工坊自由打造必須嚴守銅幣守恆");

    apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_smith",
        "equip_body_gear",
        "craft_cargo_frame_spider_chitin",
    )
    .expect("equip chitin frame");
    assert_eq!(
        effective_overland_bulk_cap(&delta.actors["pl:p_smith"]),
        4,
        "高韌性巨蛛甲片背架應將跨格負重上限擴增至 4"
    );
}

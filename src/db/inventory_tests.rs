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
    let equip = r#"{"hold_r":"craft_wooden_tool_handle"}"#;
    assert_eq!(
        pick_entity_trade_offer(inv, equip).as_deref(),
        Some("wt-good-012")
    );
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
        InventoryEntry { item_id: "mat_reed_bundle".into(), qty: 133_530 },
        InventoryEntry { item_id: "manual_854b8035e7134512".into(), qty: 1 },
    ];
    assert!(normalize_npc_inventory_entries(&mut entries));
    assert_eq!((entries.len(), entries[0].item_id.as_str(), entries[0].qty), (1, "mat_reed_bundle", MAX_NPC_ITEM_STACK_QTY));
    for i in 1..=10 {
        entries.push(InventoryEntry { item_id: format!("mat_item_{i}"), qty: 50 });
    }
    assert!(normalize_npc_inventory_entries(&mut entries));
    assert!(entries.len() <= MAX_NPC_DISTINCT_ITEMS && entries.iter().all(|it| it.qty <= MAX_NPC_ITEM_STACK_QTY));
}

#[test]
fn test_peninsula_catalog_wt_cat_equipment_and_tool_slots_derived() {
    let knife = get_item_def("wt_cat_1021").expect("wt_cat_1021 should exist");
    assert_eq!(knife.slot, "hold_r");
    assert!(knife.atk_bonus > 0);

    let plane = get_item_def("wt_cat_0954").expect("wt_cat_0954 should exist");
    assert_eq!(plane.slot, "hold_r");
    assert!(plane.dex_bonus > 0);
}

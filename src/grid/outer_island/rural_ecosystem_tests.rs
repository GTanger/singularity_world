//! 銀鷗島 `rural_v5` 全島 88 城間地塊、35 基層聚落、實體車船、二階防螞蟻搬家負重與實物工序閉環驗收測試。

use super::actions::apply_world_action;
use super::causal_arcs::validate_biography_prose;
use super::clock_and_lang::solar_clock_at;
use super::engine::load_static_world_doc;
use super::explore_and_combat::apply_explore;
use super::movement::{add_carry_item, apply_move_step, apply_setup_origin, shortest_node_path};
use super::persist_pg::prepare_commit_batch;
use super::rural_migrate::apply_rural_v5_pg_upgrade;
use super::sim_tick::{seed_genesis_actors_once, tick_outer_island_world};
use super::types::{CarryItem, IslandWorldDelta, UnifiedMindBody};
use super::world_weave_bio::{settlement_name_zh, settlement_slug_of};

#[test]
fn all_88_cells_35_burgs_and_994_biographies_hold_strict_invariants() {
    let doc = load_static_world_doc();
    assert_eq!(doc.cells.len(), 93);
    assert_eq!(doc.transit_lines.len(), 4);
    assert!(doc.residents.len() >= 980);

    // 1. 88 個城間地塊描寫 100% 唯一，且 35 正典聚落分級（29 活聚落 + 5 封村遺址 + 1 黑煤塌陷坑）精確無誤
    let mut non_core_descs = std::collections::BTreeSet::new();
    let mut rural_live = 0usize;
    let mut plague_ruins = 0usize;
    let mut ruin_dng = 0usize;
    for c in doc.cells.values() {
        if c.kind != "settlement" {
            assert!(
                non_core_descs.insert(c.desc.clone()),
                "duplicate cell desc on cell {}: {}",
                c.cell_id,
                c.desc
            );
        }
        match c.kind.as_str() {
            "rural_settlement" => rural_live += 1,
            "plague_ruin" => plague_ruins += 1,
            "ruin_dungeon" => ruin_dng += 1,
            _ => {}
        }
    }
    assert_eq!(non_core_descs.len(), 88);
    assert_eq!(rural_live, 29);
    assert_eq!(plague_ruins, 5);
    assert_eq!(ruin_dng, 1);

    // 2. 幹道村口正交側枝＋網狀微拓撲（Room 4387da46）：東西橫貫步數落於 24..=65 步合理大島區間
    let ew_path = shortest_node_path(
        &doc,
        "oi:white_gull_harbor:dock",
        "oi:west_strait_ferry:dock",
    )
    .expect("ew path exists");
    let ew_steps = ew_path.len() - 1;
    assert!(
        (24..=65).contains(&ew_steps),
        "EW island spine out of 24..=65 range, steps={ew_steps}"
    );

    // 3. 五鎮二樓 1~6 號獨立客房與四港記籌手室內居所齊備
    for rm in 1..=6 {
        assert!(
            doc.nodes
                .contains_key(&format!("oi:white_gull_harbor:inn_room_{rm}"))
        );
    }
    for slug in [
        "white_gull_harbor",
        "west_strait_ferry",
        "gull_tail_port",
        "north_cape_beacon",
    ] {
        let th = format!("oi:{slug}:tally_house");
        assert!(doc.nodes.get(&th).is_some_and(|n| n.indoor));
    }

    // 4. 創世建檔後全島 994 名居民 100% 通過文學守門員，且鄉野戶長皆具五鎮親師/契作關係，銅幣守恆差額 = 0
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    assert_eq!(delta.conservation_diff(&doc), 0);

    let mut rural_heads_checked = 0usize;
    for a in delta.actors.values() {
        if a.controller_id.is_some() {
            continue;
        }
        let slug = settlement_slug_of(&a.household_id);
        let tzh = settlement_name_zh(&doc, a);
        validate_biography_prose(&a.display_name, slug, &tzh, &a.personal_bio_zh)
            .unwrap_or_else(|e| panic!("bio gate failed on {}: {e}", a.actor_id));
        if a.household_id.starts_with("hh:burg_") && a.actor_id.ends_with("_0") {
            rural_heads_checked += 1;
            assert!(
                !a.mind.ltm.social.is_empty(),
                "rural head {} must be woven into social network",
                a.actor_id
            );
        }
    }
    assert!(rural_heads_checked >= 80);
}

#[test]
fn plague_ruins_zero_monsters_tea_shed_zero_lock_and_apex_permanent_death() {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    let (epoch, _) = crate::game::game_time_cfg();
    let clk = solar_clock_at(epoch + 1_200);

    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "eco_tester",
        "林間客",
        "人類",
        "forest_hunter",
        "背著長弓",
    )
    .expect("setup ok");
    let aid = delta.resolve_controller("eco_tester").unwrap();
    let _ = delta.transfer_copper(
        0,
        3,
        "test_grant",
        "municipal_treasury",
        &aid,
        20,
        "測試盤纏".to_string(),
    );

    // 1. 南岸船疫封村遺址（4849 古加津舊祠堂）：跨多時段搜巡 0 刷怪，尋得三二六年未寄家書，並於達利莊隔欄遞交
    delta.relocate_actor(&aid, "oi:burg_4849:ruin_hall", 4849, true);
    for step in 0..8 {
        let c_step = solar_clock_at(epoch + 1_200 + step * 600);
        let _ = apply_explore(&doc, &mut delta, &c_step, "eco_tester").expect("ruin explore ok");
        assert!(
            delta.actors.get(&aid).unwrap().active_skirmish.is_none(),
            "sealed plague ruin must NEVER spawn monsters!"
        );
    }
    assert!(
        delta
            .actors
            .get(&aid)
            .unwrap()
            .backpack
            .iter()
            .any(|i| i.slug == "plague_family_letter")
    );

    delta.relocate_actor(&aid, "oi:burg_4777:yard", 4777, true);
    let barrier_msg =
        apply_world_action(&doc, &mut delta, &clk, "eco_tester", "survivor_barrier", "")
            .expect("deliver survivor letter across barrier");
    assert!(barrier_msg.contains("濃煎苦艾湯囊"));

    // 2. 官道避雨茶棚（0 秒熱茶不鎖人、消疲勞並回傳真實活情報）
    delta.actors.get_mut(&aid).unwrap().fatigue = 40;
    delta.relocate_actor(&aid, "oi:cell4432:tea_shed", 4432, true);
    let tea_msg = apply_world_action(&doc, &mut delta, &clk, "eco_tester", "drink_tea_shed", "")
        .expect("tea shed drink ok");
    assert!(tea_msg.contains("苦艾粗茶"));
    let p_after_tea = delta.actors.get(&aid).unwrap();
    assert_eq!(p_after_tea.fatigue, 22);
    assert!(p_after_tea.ongoing_activity.is_none());

    // 三尊唯一太古巨物（4070、4326、5079）全數掛載於真實島嶼荒野地塊
    let apex_cids: Vec<u32> = doc
        .cells
        .values()
        .filter(|c| c.apex_beast.is_some())
        .map(|c| c.cell_id)
        .collect();
    assert_eq!(
        apex_cids.len(),
        3,
        "all 3 apex beasts must exist on valid island cells"
    );

    // 3. 太古巨物（4070 淵潮覆甲巨鰲）：擊殺後永久寫入 slain_apex_beasts，再次搜巡絕不重生！
    delta.relocate_actor(&aid, "oi:cell4070:0", 4070, true);
    apply_explore(&doc, &mut delta, &clk, "eco_tester").expect("trigger apex");
    assert_eq!(
        delta
            .actors
            .get(&aid)
            .unwrap()
            .active_skirmish
            .as_ref()
            .map(|s| s.foe_slug.as_str()),
        Some("apex:abyssal_turtle_4070")
    );
    let _ = apply_world_action(&doc, &mut delta, &clk, "eco_tester", "skirmish_wrist", "");
    let slay_msg = apply_world_action(&doc, &mut delta, &clk, "eco_tester", "skirmish_heavy", "")
        .expect("slay apex");
    assert!(slay_msg.contains("再也不會重現人間"));
    assert!(delta.actors.get(&aid).unwrap().active_skirmish.is_none());
    assert!(delta.slain_apex_beasts.contains_key(&4070));

    let clk_next_day = solar_clock_at(epoch + 5_200);
    apply_explore(&doc, &mut delta, &clk_next_day, "eco_tester")
        .expect("re-explore after apex dead");
    assert!(
        delta.actors.get(&aid).unwrap().active_skirmish.is_none(),
        "slain apex beast must never respawn"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn bulk_anti_ant_carrying_embodied_vehicles_and_dock_haul_drop_conserve_copper() {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    let (epoch, _) = crate::game::game_time_cfg();
    let clk = solar_clock_at(epoch + 1_200);

    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "haul_tester",
        "扛箱客",
        "人類",
        "smith_apprentice",
        "肩寬臂粗",
    )
    .expect("setup ok");
    let aid = delta.resolve_controller("haul_tester").unwrap();
    let _ = delta.transfer_copper(
        0,
        3,
        "test_grant",
        "municipal_treasury",
        &aid,
        25,
        "測試車資".to_string(),
    );

    // 1. 碼頭記籌手領箱 -> 走到廣場中途放下（DroppedAt 落入地面行囊，零憑空消失）-> 領第二箱送達鍛鐵鋪領工資！
    apply_world_action(&doc, &mut delta, &clk, "haul_tester", "claim_dock_haul", "")
        .expect("claim dock crate");
    apply_move_step(
        &doc,
        &mut delta,
        &clk,
        "haul_tester",
        "oi:white_gull_harbor:market_avenue",
        1000,
    )
    .expect("intra-town walk to market_avenue with dock crate allowed");
    apply_move_step(
        &doc,
        &mut delta,
        &clk,
        "haul_tester",
        "oi:white_gull_harbor:plaza",
        1600,
    )
    .expect("intra-town walk with dock crate allowed");
    apply_world_action(&doc, &mut delta, &clk, "haul_tester", "cancel_activity", "")
        .expect("cancel drops crate at plaza");
    assert!(
        delta
            .dropped_loot_by_node
            .get("oi:white_gull_harbor:plaza")
            .is_some_and(|v| !v.is_empty()),
        "cancelled dock crate must remain physically on ground"
    );
    // 重新彎腰拾起落地的港務貨箱：自動還原送貨工序，絕不軟鎖！
    apply_world_action(
        &doc,
        &mut delta,
        &clk,
        "haul_tester",
        "pickup_dropped_bag",
        "",
    )
    .expect("re-pickup dropped dock crate");
    assert!(
        delta.actors.get(&aid).unwrap().ongoing_activity.is_some(),
        "picking up dropped dock crate must restore ongoing_activity"
    );
    apply_world_action(&doc, &mut delta, &clk, "haul_tester", "cancel_activity", "")
        .expect("re-drop dock crate cleanly");

    delta.relocate_actor(&aid, "oi:white_gull_harbor:dock", 4616, true);
    apply_world_action(&doc, &mut delta, &clk, "haul_tester", "claim_dock_haul", "")
        .expect("claim second dock crate");
    let target_shop = delta
        .actors
        .get(&aid)
        .unwrap()
        .ongoing_activity
        .as_ref()
        .unwrap()
        .target_node
        .clone();
    let path = super::movement::shortest_walk_path(
        &doc,
        "oi:white_gull_harbor:dock",
        &target_shop,
        &std::collections::BTreeSet::new(),
    )
    .unwrap();
    for (idx, step_nid) in path.iter().enumerate().skip(1) {
        apply_move_step(
            &doc,
            &mut delta,
            &clk,
            "haul_tester",
            step_nid,
            2000 + (idx as u64) * 500,
        )
        .expect("step to shop");
    }
    assert!(
        delta.actors.get(&aid).unwrap().ongoing_activity.is_none(),
        "arriving at shop auto-delivers crate"
    );

    // 2. 防螞蟻搬家：分堆攜帶 2 鐵礦 + 1 精煤（合計 3 件大宗商貨）禁止徒步跨出北城門，但可搭雙馬驛車！
    delta.relocate_actor(&aid, "oi:white_gull_harbor:north_gate", 4616, true);
    let out_wild = doc.nodes["oi:white_gull_harbor:north_gate"]
        .exits
        .iter()
        .find(|e| doc.nodes[&e.to].cell_id != 4616)
        .unwrap()
        .to
        .clone();
    {
        let a = delta.actors.get_mut(&aid).unwrap();
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "iron_ore".to_string(),
                name: "粗鐵礦石".to_string(),
                count: 2,
                weight_units: 4,
                ..CarryItem::default()
            },
        );
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "black_coal".to_string(),
                name: "北林精煤塊".to_string(),
                count: 1,
                weight_units: 4,
                ..CarryItem::default()
            },
        );
    }
    assert!(
        apply_move_step(&doc, &mut delta, &clk, "haul_tester", &out_wild, 9000).is_err(),
        "3 bulk items split across stacks must block overland walking"
    );

    // 同等規則約束 NPC（sim_tick.rs:480 旁路已封死）
    let npc_id = "res:white_gull_harbor:dock_porter";
    delta.relocate_actor(npc_id, "oi:white_gull_harbor:north_gate", 4616, true);
    {
        let npc = delta.actors.get_mut(npc_id).unwrap();
        add_carry_item(
            &mut npc.backpack,
            CarryItem {
                slug: "timber_log".to_string(),
                name: "灰松厚原木".to_string(),
                count: 3,
                weight_units: 4,
                ..CarryItem::default()
            },
        );
        npc.route_plan.clear();
        npc.route_plan.push_back(out_wild.clone());
    }
    tick_outer_island_world(&doc, &mut delta, &clk, 12_000);
    assert_eq!(
        delta.actors.get(npc_id).unwrap().node_id,
        "oi:white_gull_harbor:north_gate",
        "NPC with >2 bulk items must ALSO be blocked from overland walking"
    );

    // 改至市集廣場搭東西幹線雙馬驛車：背負 3 件重貨順利隨車推進並於下一站停靠下車！
    delta.relocate_actor(&aid, "oi:white_gull_harbor:plaza", 4616, true);
    apply_world_action(
        &doc,
        &mut delta,
        &clk,
        "haul_tester",
        "board_transit",
        "land_ew_express",
    )
    .expect("board ew carriage with bulk cargo");
    apply_world_action(
        &doc,
        &mut delta,
        &clk,
        "haul_tester",
        "transit_alight_next",
        "",
    )
    .expect("request alight at next stop");
    for step in 0..8 {
        tick_outer_island_world(&doc, &mut delta, &clk, 20_000 + step * 2_500);
        if delta.actors.get(&aid).unwrap().active_transit.is_none() {
            break;
        }
    }
    assert!(delta.actors.get(&aid).unwrap().active_transit.is_none());
    assert_ne!(
        delta.actors.get(&aid).unwrap().node_id,
        "oi:white_gull_harbor:plaza"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn rural_v5_pg_upgrade_preserves_existing_674_bios_players_and_120805_copper() {
    let doc = load_static_world_doc();
    // 模擬升級前線上資料庫狀態：僅含前 674 名五鎮居民＋1 名真人玩家，銀號準備金為舊值 40,000（總額仍為 120,805！）
    let mut pre_delta = IslandWorldDelta {
        persist_seq: 22_329,
        genesis_seeded: true,
        genesis_total_copper: 120_805,
        municipal_treasury_copper: 25_000,
        silver_bank_reserve_copper: 40_000,
        ..IslandWorldDelta::default()
    };
    for seed in doc.residents.iter().take(674) {
        let a = UnifiedMindBody {
            actor_id: seed.id.clone(),
            soul_seed: seed.soul_seed,
            alive: true,
            setup_done: true,
            display_name: seed.name.clone(),
            race: seed.race.clone(),
            role: seed.role.clone(),
            household_id: seed.household_id.clone(),
            home_node: seed.home_node.clone(),
            work_node: seed.work_node.clone(),
            node_id: seed.work_node.clone(),
            copper: seed.copper,
            bio_core_zh: format!("【精修保留測試】{}在白鷗港的既有列傳一字不改。", seed.name),
            personal_bio_zh: format!("【精修保留測試】{}在白鷗港的既有列傳一字不改。", seed.name),
            ..UnifiedMindBody::default()
        };
        pre_delta.upsert_actor(a);
    }
    let (epoch, _) = crate::game::game_time_cfg();
    let clk = solar_clock_at(epoch + 1_200);
    apply_setup_origin(
        &doc,
        &mut pre_delta,
        &clk,
        "live_human_player",
        "真實旅人",
        "精靈",
        "herb_gatherer",
        "斗篷染露",
    )
    .expect("live player exists");
    assert_eq!(pre_delta.conservation_diff(&doc), 0);

    let spawned =
        apply_rural_v5_pg_upgrade(&doc, &mut pre_delta, clk.game_days, clk.bell_index, false);
    assert_eq!(spawned, doc.residents.len() - 674);
    assert_eq!(pre_delta.actors.len(), doc.residents.len() + 1);
    assert_eq!(pre_delta.conservation_diff(&doc), 0);

    // 驗證既有五鎮 NPC 精修列傳與真人玩家 100% 原樣保留，且提交快照檢查通過
    let first_id = &doc.residents[0].id;
    assert!(
        pre_delta
            .actors
            .get(first_id)
            .unwrap()
            .personal_bio_zh
            .starts_with("【精修保留測試】")
    );
    assert!(pre_delta.resolve_controller("live_human_player").is_some());
    prepare_commit_batch(&doc, &pre_delta, false).expect("commit batch conservation check passes");
}

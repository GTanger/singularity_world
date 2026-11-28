//! 東南外海大島（`Feature 6`）與全大陸通用單一 `UnifiedMindBody` 核心模組與單元測試。

pub mod actions;
pub mod actor_map;
pub mod appraisal;
mod atlas_live_pulse;
pub mod atlas_observatory;
pub mod body_gear_craft;
#[cfg(test)]
mod body_gear_tests;
pub mod causal_arc_catalog;
pub mod causal_arcs;
pub mod cell_maps;
pub mod cell_resources;
pub mod chronicle_export;
pub mod clock_and_lang;
pub mod desire_effects;
pub mod desire_sim;
pub mod dialogue_consequence;
pub mod dialogue_llm;
#[cfg(test)]
mod diegetic_surface_tests;
#[cfg(test)]
mod emotion_mind_tests;
pub mod engine;
pub mod explore_and_combat;
pub mod gazetteer;
pub mod gazetteer_wars;
pub mod harbor_history;
pub mod harbor_lineage;
pub mod household_economy;
pub mod island_substrate;
pub mod island_substrate_pack;
pub mod island_substrate_paths;
pub mod island_substrate_rural;
#[cfg(test)]
mod island_substrate_tests;
pub mod island_substrate_towns;
pub mod live_agency;
#[cfg(test)]
mod live_agency_tests;
pub mod live_commodities;
#[cfg(test)]
mod live_commodities_tests;
pub mod live_land;
#[cfg(test)]
mod live_land_tests;
pub mod live_land_tiles;
pub mod live_lifecycle;
pub mod live_shifts;
pub mod live_tide;
#[cfg(test)]
mod live_waves_tests;
pub mod live_wilds;
pub mod living_chronicler;
pub mod living_chronicler_deeds;
pub mod living_chronicler_gate;
#[cfg(test)]
mod living_chronicler_tests;
pub mod living_chronicler_worker;
pub mod living_presence;
#[cfg(test)]
mod living_presence_tests;
#[cfg(test)]
mod living_player_audit_tests;
#[cfg(test)]
mod living_player_tests;
#[cfg(test)]
mod living_water_tests;
pub mod map_memory;
pub mod map_memory_pack;
#[cfg(test)]
mod map_memory_tests;
pub mod map_walk;
pub mod map_walk_plan;
pub mod martial_art_and_manuals;
#[cfg(test)]
mod martial_art_tests;
pub mod mesh_expedition;
pub mod mind;
pub mod movement;
pub mod offline_autonomy;
pub mod ongoing_activity;
pub mod pacts_and_peers;
pub mod peer_interactions;
pub mod persist_pg;
pub mod persist_schema;
pub mod persist_v1_migrate;
pub mod persona;
pub mod phase_and_vessels;
pub mod place_registry;
#[cfg(test)]
mod rural_ecosystem_tests;
pub mod rural_migrate;
pub mod scene_objects;
pub mod scene_sockets;
pub mod shop_leases;
pub mod sim_tick;
pub mod slow_think;
pub mod surface_cues;
pub mod transit_and_cargo;
pub mod types;
pub mod unified_kernel;
pub mod views;
pub mod world_weave;
pub mod world_weave_bio;

pub use engine::{IslandEngine, global_island, load_static_world_doc};

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::actions::apply_world_action;
    use super::clock_and_lang::{mask_speech_by_proficiency, solar_clock_at};
    use super::engine::load_static_world_doc;
    use super::explore_and_combat::{apply_explore, trigger_non_lethal_defeat};
    use super::movement::{apply_move_step, apply_setup_origin, shortest_node_path};
    use super::persist_pg::{ack_commit_batch, prepare_commit_batch};
    use super::persist_v1_migrate::{migrate_v1_player_json, migrate_v1_resident_json};
    use super::persona::derive_innate_persona;
    use super::sim_tick::{
        default_tool_for_role, seed_genesis_actors_once, spawn_dynamic_actors,
        tick_outer_island_world,
    };
    use super::types::{ContractRecord, IslandWorldDelta, NewActorSpawnSpec, ResidentSeedDef};
    use super::views::{build_filtered_map_json, build_view_json};

    fn fresh_delta() -> (super::types::WorldStaticDoc, IslandWorldDelta) {
        let doc = load_static_world_doc();
        let delta = IslandWorldDelta {
            genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
            municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
            silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
            ..IslandWorldDelta::default()
        };
        assert_eq!(delta.conservation_diff(&doc), 0);
        (doc, delta)
    }

    #[test]
    fn static_island_topology_and_wall_blocking_invariants_hold() {
        let (doc, _) = fresh_delta();
        assert_eq!(doc.cells.len(), 93);
        assert!(doc.residents.len() >= 600);
        assert!(doc.households.len() >= 200);

        let path = shortest_node_path(
            &doc,
            "oi:white_gull_harbor:dock",
            "oi:west_strait_ferry:dock",
        )
        .expect("east-to-west route must exist");
        assert!(
            (12..=35).contains(&(path.len() - 1)),
            "unexpected cross-island steps: {}",
            path.len() - 1
        );

        let w_cell_node = doc.nodes.get("oi:cell4528:0").expect("cell 4528 exists");
        for ex in &w_cell_node.exits {
            let dst = doc.nodes.get(&ex.to).expect("dst exists");
            assert_ne!(
                dst.cell_id, 4616,
                "cell 4528 must not pierce White Gull Harbor un-gated western city wall"
            );
        }
    }

    #[test]
    fn language_masking_cuts_at_source_across_three_tiers() {
        let secret = "北林斷橋石縫下有試煉坑暗門與古符文原石";
        let (m0, ok0) = mask_speech_by_proficiency("紅鬚托爾", "dwarven", 0, secret);
        assert!(!ok0);
        assert!(!m0.contains("試煉坑"));
        assert!(!m0.contains("原石"));

        let (m30, ok30) = mask_speech_by_proficiency("紅鬚托爾", "dwarven", 35, secret);
        assert!(!ok30);
        assert!(m30.contains("北林"));
        assert!(!m30.contains("石縫下有"));

        let (m85, ok85) = mask_speech_by_proficiency("紅鬚托爾", "dwarven", 85, secret);
        assert!(ok85);
        assert!(m85.contains(secret));
    }

    #[test]
    fn double_entry_copper_conservation_and_defeat_loot_cycle() {
        let (doc, mut delta) = fresh_delta();
        let clock = solar_clock_at(1_772_118_486);
        seed_genesis_actors_once(&doc, &mut delta, false);

        apply_setup_origin(
            &doc,
            &mut delta,
            &clock,
            "tester_a",
            "羅蘭",
            "人類",
            "肩披灰布斗篷",
            "smith_apprentice",
        )
        .expect("setup ok");
        assert_eq!(delta.conservation_diff(&doc), 0);
        assert_eq!(
            delta.actor_by_id_or_controller("tester_a").unwrap().copper,
            0
        );

        // 登岸完成後重呼 setup 不可竄改種族或洗掉語言
        apply_setup_origin(
            &doc,
            &mut delta,
            &clock,
            "tester_a",
            "羅蘭改名",
            "矮人",
            "換了新披風",
            "forest_hunter",
        )
        .expect("re-setup cosmetic only");
        let a_ref = delta.actor_by_id_or_controller("tester_a").unwrap();
        assert_eq!(a_ref.race, "人類");
        assert_eq!(a_ref.languages.get("dwarven").copied().unwrap_or(0), 0);

        apply_world_action(&doc, &mut delta, &clock, "tester_a", "work_dock", "")
            .expect("dock work ok");
        assert_eq!(
            delta.actor_by_id_or_controller("tester_a").unwrap().copper,
            4
        );
        assert_eq!(delta.conservation_diff(&doc), 0);

        for (idx, dst) in [
            "oi:white_gull_harbor:market_avenue",
            "oi:white_gull_harbor:plaza",
            "oi:white_gull_harbor:white_gull_bridge",
            "oi:white_gull_harbor:river_tavern_st",
            "oi:white_gull_harbor:rusty_anchor_tavern",
        ]
        .iter()
        .enumerate()
        {
            apply_move_step(
                &doc,
                &mut delta,
                &clock,
                "tester_a",
                dst,
                1000 + (idx as u64) * 500,
            )
            .expect("step to tavern");
        }

        // 未付宿費前上二樓不得白用木箱
        apply_move_step(
            &doc,
            &mut delta,
            &clock,
            "tester_a",
            "oi:white_gull_harbor:inn_upstairs",
            4000,
        )
        .expect("upstairs");
        assert!(
            apply_world_action(
                &doc,
                &mut delta,
                &clock,
                "tester_a",
                "store_item",
                "rye_bread"
            )
            .is_err()
        );
        apply_move_step(
            &doc,
            &mut delta,
            &clock,
            "tester_a",
            "oi:white_gull_harbor:rusty_anchor_tavern",
            5000,
        )
        .expect("downstairs");

        apply_world_action(&doc, &mut delta, &clock, "tester_a", "redeem_voucher", "")
            .expect("redeem voucher");
        apply_world_action(&doc, &mut delta, &clock, "tester_a", "buy_ration", "")
            .expect("buy ration");
        assert_eq!(
            delta.actor_by_id_or_controller("tester_a").unwrap().copper,
            2
        );
        assert_eq!(delta.conservation_diff(&doc), 0);

        trigger_non_lethal_defeat(
            &doc,
            &mut delta,
            &clock,
            "tester_a",
            "oi:cell4529:0",
            "grey_ear_goblin",
            "灰耳哥布林斥候",
        )
        .expect("defeat ok");
        let a_after = delta.actor_by_id_or_controller("tester_a").unwrap();
        assert_eq!(a_after.copper, 0);
        assert_eq!(a_after.bound_weapon.name, "港務舊鍛錘");
        assert!(delta.dropped_loot_by_node.contains_key("oi:cell4529:0"));
        assert_eq!(delta.conservation_diff(&doc), 0);

        apply_setup_origin(
            &doc,
            &mut delta,
            &clock,
            "tester_b",
            "薇拉",
            "精靈",
            "背負長弓",
            "forest_hunter",
        )
        .expect("setup b");
        delta.relocate_actor("tester_b", "oi:cell4529:0", 4529, false);
        apply_world_action(
            &doc,
            &mut delta,
            &clock,
            "tester_b",
            "pickup_dropped_bag",
            "",
        )
        .expect("pickup ok");
        let b_ref = delta.actor_by_id_or_controller("tester_b").unwrap();
        assert_eq!(b_ref.copper, 2);
        assert_eq!(delta.pickup_ledger.len(), 1);
        assert_eq!(delta.conservation_diff(&doc), 0);

        // 確認拾遺物品回傳給前端時絕不洩漏原失主的登入帳號 ID
        let view_b = build_view_json(&doc, &delta, b_ref, &clock).to_string();
        assert!(!view_b.contains("tester_a"));
    }

    #[test]
    fn spider_blocks_altar_and_dungeon_fog_hides_unvisited_rooms() {
        let (doc, mut delta) = fresh_delta();
        let clock = solar_clock_at(1_770_000_000);
        apply_setup_origin(
            &doc,
            &mut delta,
            &clock,
            "dng_runner",
            "艾倫",
            "人類",
            "",
            "smith_apprentice",
        )
        .expect("setup");

        delta.relocate_actor("dng_runner", "oi:dng_pit:lair", 4617, false);

        // 坑道局部圖不得洩漏未踏足的凝石台 ID 與名稱
        let runner = delta.actor_by_id_or_controller("dng_runner").unwrap();
        let dng_map = build_filtered_map_json(&doc, &delta, runner, &clock).to_string();
        assert!(!dng_map.contains("oi:dng_pit:altar"));
        assert!(!dng_map.contains("地脈凝石台"));

        // 未擊敗巨蛛直接闖向凝石台會被巨蛛攔截進入對峙，且對峙期間禁止買票或打鐵
        apply_move_step(
            &doc,
            &mut delta,
            &clock,
            "dng_runner",
            "oi:dng_pit:altar",
            9000,
        )
        .expect("ambushed");
        let runner_after = delta.actor_by_id_or_controller("dng_runner").unwrap();
        assert_eq!(runner_after.node_id, "oi:dng_pit:lair");
        assert!(runner_after.active_skirmish.is_some());
        assert!(
            apply_world_action(
                &doc,
                &mut delta,
                &clock,
                "dng_runner",
                "board_carriage_west",
                ""
            )
            .is_err()
        );
    }

    #[test]
    fn packet_fog_and_no_meta_jargon_in_views() {
        let (doc, mut delta) = fresh_delta();
        let clock = solar_clock_at(1_770_000_000);
        apply_setup_origin(
            &doc,
            &mut delta,
            &clock,
            "viewer1",
            "艾倫",
            "人類",
            "",
            "smith_apprentice",
        )
        .expect("setup");

        // 初抵僅踏足白鷗港（4616），隨後邁入 4617 探索
        delta.relocate_actor("viewer1", "oi:cell4617:0", 4617, false);
        let first = apply_explore(&doc, &mut delta, &clock, "viewer1").expect("explore 1");
        let second = apply_explore(&doc, &mut delta, &clock, "viewer1").expect("explore 2");
        assert_ne!(first, second);
        assert!(second.contains("這個時段"));

        let viewer = delta.actor_by_id_or_controller("viewer1").unwrap();
        let map_json = build_filtered_map_json(&doc, &delta, viewer, &clock);
        let visited_arr = map_json["visited_cells"].as_array().unwrap();
        assert_eq!(visited_arr.len(), 2);
        let outline_arr = map_json["outline_cells"].as_array().unwrap();
        assert!(!outline_arr.is_empty());
        assert!(outline_arr[0].get("title").is_none());

        let view_str = build_view_json(&doc, &delta, viewer, &clock).to_string();
        for forbidden in [
            "去數值化",
            "無職業",
            "零現銀",
            "防分身",
            "獨占態",
            "高維混沌",
            "富態化",
            "定形紋",
            "soul_seed",
            "STM",
            "LTM",
            "CAPS",
            "好感度",
        ] {
            assert!(
                !view_str.contains(forbidden),
                "leaked forbidden meta term in view JSON: {forbidden}"
            );
        }
    }

    #[test]
    fn all_674_residents_and_players_share_single_unified_mind_body_and_unique_soul_seed() {
        let (doc, mut delta) = fresh_delta();
        let clock = solar_clock_at(1_770_000_000);
        seed_genesis_actors_once(&doc, &mut delta, false);
        apply_setup_origin(
            &doc,
            &mut delta,
            &clock,
            "p_unique_1",
            "葉卅",
            "人類",
            "青衫行旅",
            "smith_apprentice",
        )
        .expect("setup p1");
        apply_setup_origin(
            &doc,
            &mut delta,
            &clock,
            "p_unique_2",
            "薇拉",
            "精靈",
            "長弓獵手",
            "forest_hunter",
        )
        .expect("setup p2");

        assert_eq!(delta.actors.len(), doc.residents.len() + 2);
        assert_eq!(delta.conservation_diff(&doc), 0);

        // H1 回歸斷言：創世居民必須 100% 保留編譯期 63-bit 正規化 `soul_seed`（與 PG `BIGINT` 完全一致），絕不因雙重配發被重抽洗掉！
        for r in &doc.residents {
            let expected_seed = (r.soul_seed & ((1u64 << 63) - 1)).max(1);
            assert_eq!(
                delta.actors[&r.id].soul_seed, expected_seed,
                "genesis resident {} lost canonical soul_seed",
                r.id
            );
        }

        let mut seen_seeds = BTreeSet::new();
        for a in delta.actors.values() {
            assert!(a.soul_seed > 0);
            assert!(
                seen_seeds.insert(a.soul_seed),
                "duplicate UnifiedMindBody soul_seed: {}",
                a.soul_seed
            );
            let innate = derive_innate_persona(a.soul_seed);
            let sum_v: u32 = innate.base_values.iter().map(|&v| u32::from(v)).sum();
            assert_eq!(sum_v, 240, "innate values must conserve budget 240");
        }
    }

    #[test]
    fn unified_actors_walk_step_by_step_and_settle_double_entry_meals() {
        let (doc, mut delta) = fresh_delta();
        seed_genesis_actors_once(&doc, &mut delta, true);
        assert_eq!(delta.conservation_diff(&doc), 0);

        let day_clock = solar_clock_at(1_770_020_000);
        let mut t_ms = 10_000u64;
        let mut any_routed = false;
        for step in 0..8 {
            let mut clk = day_clock.clone();
            clk.bell_index = (2 + step) % 6;
            clk.is_night = false;
            tick_outer_island_world(&doc, &mut delta, &clk, t_ms);
            if delta.actors.values().any(|a| !a.route_plan.is_empty()) {
                any_routed = true;
            }
            assert_eq!(
                delta.conservation_diff(&doc),
                0,
                "copper conservation violated at step {step}"
            );
            t_ms += 10_000;
        }
        assert!(
            any_routed,
            "autonomous UnifiedMindBody must physically walk multi-step routes"
        );

        let mut night_clk = day_clock;
        night_clk.is_night = true;
        night_clk.bell_index = 11;
        night_clk.game_days += 1;
        for _ in 0..5 {
            tick_outer_island_world(&doc, &mut delta, &night_clk, t_ms);
            t_ms += 10_000;
        }
        assert_eq!(delta.conservation_diff(&doc), 0);
        assert!(
            delta
                .actors
                .values()
                .any(|a| a.mind.stm.last_sleep_day >= night_clk.game_days),
            "actors at home at night must run sleep consolidation"
        );
    }

    #[test]
    fn language_masked_sensory_and_three_gate_permanent_character_shift() {
        use super::mind::{CognitiveMind, EventPublicity, EvidenceTier};
        use super::persona::CharacterDriftState;

        let innate = derive_innate_persona(987_654_321);
        let mut drift = CharacterDriftState::default();
        let mut mind = CognitiveMind::default();
        let mut human_langs = BTreeMap::new();
        human_langs.insert("harbor_trade".to_string(), 90);

        let accepted = mind.ingest_sensory_event(
            &human_langs,
            EventPublicity::TableWhisper,
            Some("dwarven"),
            false,
            "ev:dwarven_secret",
            10,
            3,
            "oi:white_gull_harbor:bal_forge",
            "res:dwarf_1",
            "鐵砧巴爾",
            EvidenceTier::TavernHearsay,
            80,
            50,
            innate.taboo_axis,
            "北林斷橋石縫下藏著古符文原石",
        );
        assert!(accepted);
        let stored = &mind.stm.day_episodes.back().unwrap().summary_zh;
        assert!(!stored.contains("古符文原石"));

        let taboo = innate.taboo_axis;
        let _ = mind.ingest_sensory_event(
            &human_langs,
            EventPublicity::TableWhisper,
            Some("harbor_trade"),
            true,
            "ev:betrayal_anchor_1",
            1,
            5,
            "oi:white_gull_harbor:plaza",
            "traitor_x",
            "背信者",
            EvidenceTier::DirectWitness,
            92,
            90,
            taboo,
            "結拜夥計捲走全部過冬救命銅幣潛逃",
        );
        mind.run_sleep_consolidation(&innate, &mut drift, 1, true);
        assert!(drift.permanent_shifts.is_empty());
        mind.run_sleep_consolidation(&innate, &mut drift, 2, true);
        assert!(drift.permanent_shifts.is_empty());
        mind.run_sleep_consolidation(&innate, &mut drift, 3, true);
        assert_eq!(drift.permanent_shifts.len(), 1);
        assert_eq!(
            drift.permanent_shifts[0].anchor_event_id,
            "ev:betrayal_anchor_1"
        );
        assert!(!mind.ltm.turning_points.is_empty());
    }

    #[test]
    fn pinned_shopkeeper_and_acquaintances_never_vanish_under_crowd_overflow() {
        let (doc, mut delta) = fresh_delta();
        let clock = solar_clock_at(1_770_020_000);
        seed_genesis_actors_once(&doc, &mut delta, false);
        apply_setup_origin(
            &doc,
            &mut delta,
            &clock,
            "crowd_tester",
            "葉卅",
            "人類",
            "",
            "smith_apprentice",
        )
        .expect("setup");

        let tavern_id = "oi:white_gull_harbor:rusty_anchor_tavern";
        delta.relocate_actor("crowd_tester", tavern_id, 4616, true);
        let passerby_ids: Vec<String> = delta
            .actors
            .keys()
            .filter(|id| {
                id.as_str() != "res:white_gull_harbor:shop_1_0" && id.as_str() != "pl:crowd_tester"
            })
            .take(15)
            .cloned()
            .collect();
        for pid in passerby_ids {
            delta.relocate_actor(&pid, tavern_id, 4616, true);
        }
        delta.relocate_actor("res:white_gull_harbor:shop_1_0", tavern_id, 4616, true);

        let viewer = delta.actor_by_id_or_controller("crowd_tester").unwrap();
        let v = build_view_json(&doc, &delta, viewer, &clock);
        let primary = v["scene"]["npcs"].as_array().unwrap();
        let overflow = v["scene"]["crowd_overflow"].as_array().unwrap();
        assert_eq!(primary.len(), 6);
        assert!(overflow.len() >= 10);
        assert!(
            primary.iter().any(|n| n["name"] == "瑪莎"),
            "pinned tavern keeper must stay in primary slots even in a crowded tavern"
        );
    }

    #[test]
    fn direct_copper_mutation_and_concurrent_settled_contract_commit_cleanly() {
        let (doc, mut delta) = fresh_delta();
        let mut clock = solar_clock_at(1_770_020_000);
        seed_genesis_actors_once(&doc, &mut delta, false);

        // 先完成創世全量落盤快照確認
        let b0 = prepare_commit_batch(&doc, &delta, true).expect("genesis batch");
        ack_commit_batch(&mut delta, &b0);

        let debtor_id = "res:white_gull_harbor:dock_porter".to_string();
        let creditor_id = "res:white_gull_harbor:shop_1_0".to_string();
        let orig_copper = delta.actors[&debtor_id].copper;

        // 1. 經複式帳本撥款 +2 銅幣並完成落盤
        assert!(delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "test_wage",
            "municipal_treasury",
            &debtor_id,
            2,
            "測試工錢".to_string(),
        ));
        let b1 = prepare_commit_batch(&doc, &delta, false).expect("batch 1");
        assert!(
            b1.actor_rows
                .iter()
                .any(|r| r.actor_id == debtor_id && r.copper == orig_copper + 2)
        );
        ack_commit_batch(&mut delta, &b1);
        assert!(!delta.actors[&debtor_id].dirty);
        assert_eq!(delta.actors[&debtor_id].flushed_copper, orig_copper + 2);

        // 2. 結清 2 枚飯帳回到原始餘額（`orig + 2 -> orig`），確認第二次提交批次必定包含該列，且併發 settled 契約不被誤刪！
        delta.contracts.insert(
            "tab_c1_test".to_string(),
            ContractRecord {
                contract_id: "tab_c1_test".to_string(),
                kind: "meal_tab".to_string(),
                creditor_id: creditor_id.clone(),
                debtor_id: debtor_id.clone(),
                amount_copper: 2,
                due_day: clock.game_days,
                status: "open".to_string(),
                note_zh: "測試飯帳".to_string(),
            },
        );
        clock.game_days += 1;
        tick_outer_island_world(&doc, &mut delta, &clock, 50_000);
        assert_eq!(delta.conservation_diff(&doc), 0);

        let b2 = prepare_commit_batch(&doc, &delta, false).expect("batch 2");
        assert!(
            b2.actor_rows.iter().any(|r| r.actor_id == debtor_id),
            "reverted copper balance must still flush to overwrite previous PG balance"
        );

        delta.contracts.insert(
            "tab_concurrent_settled".to_string(),
            ContractRecord {
                contract_id: "tab_concurrent_settled".to_string(),
                kind: "meal_tab".to_string(),
                creditor_id,
                debtor_id: debtor_id.clone(),
                amount_copper: 2,
                due_day: clock.game_days,
                status: "settled".to_string(),
                note_zh: "併發結清飯帳".to_string(),
            },
        );
        ack_commit_batch(&mut delta, &b2);
        assert!(!delta.contracts.contains_key("tab_c1_test"));
        assert!(
            delta.contracts.contains_key("tab_concurrent_settled"),
            "concurrently settled contract must stay until its own DELETE batch commits"
        );
    }

    #[test]
    fn dynamic_hot_spawn_1500_actors_in_memory_preserves_o1_index_uniqueness_and_conservation() {
        // 純記憶體壓測（絕不寫入線上 PostgreSQL）：驗證隨時動態新增 1,500 名無靜態種子人物（合計 > 2,170 人）100% 即插即活！
        let (doc, mut delta) = fresh_delta();
        let mut clock = solar_clock_at(1_772_118_486);
        seed_genesis_actors_once(&doc, &mut delta, false);
        apply_setup_origin(
            &doc,
            &mut delta,
            &clock,
            "scale_viewer",
            "葉卅",
            "人類",
            "青衫旅人",
            "fallen_peddler",
        )
        .expect("setup viewer");

        let base_count = delta.actors.len();
        let existing_seed = delta.actors.values().next().unwrap().soul_seed;
        let mut specs = Vec::with_capacity(1_500);
        for idx in 0..1_500usize {
            let race = match idx % 4 {
                0 => "人類",
                1 => "矮人",
                2 => "精靈",
                _ => "獸人",
            };
            let role = if idx % 3 == 0 {
                "跨海行商"
            } else if idx % 3 == 1 {
                "渡海石工"
            } else {
                "北境流民"
            };
            let mut langs = BTreeMap::new();
            langs.insert("harbor_trade".to_string(), 80);
            let start_node = if idx % 2 == 0 {
                "oi:west_strait_ferry:dock"
            } else {
                "oi:white_gull_harbor:dock"
            };
            specs.push(NewActorSpawnSpec {
                actor_id: format!("dyn:migrant_{idx:04}"),
                controller_id: None,
                // 刻意在前 10 名傳入既有碰撞 soul_seed，驗證 $O(1)$ 去重分配器絕不撞號
                preferred_soul_seed: if idx < 10 { Some(existing_seed) } else { None },
                display_name: format!("渡海客{}", idx + 1),
                race: race.to_string(),
                role: role.to_string(),
                appearance: "背負行囊的渡海來客".to_string(),
                household_id: format!("hh:migrant_{}", idx / 4),
                home_node: start_node.to_string(),
                work_node: "oi:white_gull_harbor:plaza".to_string(),
                start_node: start_node.to_string(),
                initial_copper_from_mainland: 15,
                bound_weapon: default_tool_for_role(role, "渡海客"),
                languages: langs,
            });
        }

        let created =
            spawn_dynamic_actors(&doc, &mut delta, specs, clock.game_days, clock.bell_index);
        assert_eq!(created.len(), 1_500);
        assert_eq!(delta.actors.len(), base_count + 1_500);
        assert_eq!(
            delta.conservation_diff(&doc),
            0,
            "1500 mainland migrants bringing copper via mainland_clearing must keep conservation_diff == 0"
        );

        let mut unique_seeds = BTreeSet::new();
        for (aid, a) in &delta.actors {
            assert!(
                unique_seeds.insert(a.soul_seed),
                "soul_seed collision at {aid}"
            );
            assert!(
                delta
                    .node_occupants
                    .get(&a.node_id)
                    .is_some_and(|set| set.contains(aid)),
                "O(1) node_occupants index out of sync for {aid}"
            );
        }

        // 推進兩個響鐘：確認這 1,500 名不在 `doc.residents` 內的動態人物照樣走路、記地圖、上畫面、進提交批次！
        clock.bell_index = (clock.bell_index + 1) % 12;
        tick_outer_island_world(&doc, &mut delta, &clock, 80_000);
        assert_eq!(delta.conservation_diff(&doc), 0);

        let viewer = delta.actor_by_id_or_controller("scale_viewer").unwrap();
        let dock_view = build_view_json(&doc, &delta, viewer, &clock);
        let total_shown = dock_view["scene"]["npcs"].as_array().unwrap().len()
            + dock_view["scene"]["crowd_overflow"]
                .as_array()
                .unwrap()
                .len();
        assert!(
            total_shown > 500,
            "dynamic actors at White Gull Harbor dock must be visible via O(1) node_occupants without doc.residents"
        );

        let batch = prepare_commit_batch(&doc, &delta, true).expect("bulk batch");
        assert!(batch.actor_rows.len() >= 2_175);
        ack_commit_batch(&mut delta, &batch);
        assert!(delta.actors.dirty_keys().is_empty());

        // 驗證 Copy-on-Write + O(D) 髒集追蹤：2,175 人世界裡單名玩家做 1 個動作，clone + 落盤僅輸出被動到的 1~2 人！
        let mut working = delta.clone();
        apply_world_action(
            &doc,
            &mut working,
            &clock,
            "pl:scale_viewer",
            "work_dock",
            "",
        )
        .expect("dock work");
        let small_batch = prepare_commit_batch(&doc, &working, false).expect("small cow batch");
        assert!(
            (1..=2).contains(&small_batch.actor_rows.len()),
            "O(D) dirty commit must only serialize touched actors, got {}",
            small_batch.actor_rows.len()
        );
    }

    #[test]
    fn v1_dual_track_json_migrates_losslessly_into_unified_mind_body() {
        let (doc, mut delta) = fresh_delta();
        let seed_by_id: BTreeMap<&str, &ResidentSeedDef> =
            doc.residents.iter().map(|r| (r.id.as_str(), r)).collect();

        let first_seed = &doc.residents[0];
        let v1_player_raw = format!(
            r#"{{
                "player_id": "葉卅",
                "soul_seed": 7248483681745583722,
                "rev": 5,
                "display_name": "葉卅",
                "race": "人類",
                "appearance": "披著海風舊斗篷、神色沉穩",
                "origin_pack": "fallen_peddler",
                "setup_done": true,
                "node_id": "oi:white_gull_harbor:plaza",
                "fatigue": 44,
                "visited_cells": [4616, 4529],
                "known_state_cells": [4616],
                "visited_nodes": ["oi:white_gull_harbor:dock", "oi:white_gull_harbor:plaza"],
                "acquaintances": ["{}"]
            }}"#,
            first_seed.id
        );
        let migrated_pl = migrate_v1_player_json(
            &doc,
            &seed_by_id,
            "pl:葉卅",
            7_248_483_681_745_583_722,
            5,
            12,
            &v1_player_raw,
        )
        .expect("migrate v1 player");
        assert_eq!(migrated_pl.actor_id, "pl:葉卅");
        assert_eq!(migrated_pl.controller_id.as_deref(), Some("葉卅"));
        assert_eq!(migrated_pl.copper, 12);
        assert!(migrated_pl.mind.ltm.known_cells.contains(&4529));
        assert!(
            migrated_pl
                .mind
                .ltm
                .known_nodes
                .contains("oi:white_gull_harbor:dock")
        );
        assert!(migrated_pl.is_acquainted_with(&first_seed.id));
        delta.upsert_actor(migrated_pl);

        // C3 回歸測試：模擬稀疏 v1 `oi_actors`（僅第 0 名居民有寫入列，其餘 673 名居民未入表），確認補種後 674 名居民全員齊備且守恆差額 == 0！
        let r0 = &doc.residents[0];
        let v1_res_raw = format!(
            r#"{{"id":"{}","soul_seed":{},"rev":3,"alive":true,"node_id":"{}"}}"#,
            r0.id, r0.soul_seed, r0.work_node
        );
        let migrated_res = migrate_v1_resident_json(
            &doc,
            &seed_by_id,
            &r0.id,
            r0.soul_seed as i64,
            3,
            -12,
            &v1_res_raw,
        )
        .expect("migrate v1 resident");
        assert_eq!(migrated_res.copper, r0.copper - 12);
        delta.upsert_actor(migrated_res);

        delta.genesis_seeded = false;
        seed_genesis_actors_once(&doc, &mut delta, false);
        delta.rebuild_indexes();
        assert_eq!(delta.actors.len(), doc.residents.len() + 1);
        assert_eq!(
            delta.conservation_diff(&doc),
            0,
            "sparse v1->v2 migration must fill missing genesis residents and preserve conservation_diff == 0"
        );
    }
}

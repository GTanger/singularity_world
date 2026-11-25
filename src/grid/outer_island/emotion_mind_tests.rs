use std::collections::BTreeSet;

use super::appraisal::{ConcernCause, GoalStrategyKind, PrimaryEmotion};
use super::chronicle_export::build_chronicle_actor_json;
use super::clock_and_lang::{SolarClockSnapshot, solar_clock_at};
use super::desire_sim::{BellDecisionIndex, evaluate_personalized_desire};
use super::engine::load_static_world_doc;
use super::household_economy::{find_town_tavern_and_keeper, try_restock_household_larder};
use super::mind::{CognitiveMind, tie_flags};
use super::movement::{add_carry_item, apply_setup_origin};
use super::persist_pg::{ack_commit_batch, prepare_commit_batch};
use super::persona::{ValueAxis, derive_innate_persona};
use super::sim_tick::{seed_genesis_actors_once, tick_outer_island_world};
use super::slow_think::parse_slow_think_chinese_output;
use super::surface_cues::enrich_dialogue_with_graduated_disclosure;
use super::types::{CarryItem, IslandWorldDelta, UnifiedMindBody};
use super::views::build_view_json;

fn fresh_world() -> (super::types::WorldStaticDoc, IslandWorldDelta) {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    assert_eq!(delta.conservation_diff(&doc), 0);
    (doc, delta)
}

fn test_clock_for(day: i64, bell: u8) -> SolarClockSnapshot {
    let hour = u32::from(bell) * 2;
    SolarClockSnapshot {
        real_unix: 1_770_000_000 + day * 3_600 + i64::from(bell) * 300,
        game_days: day,
        hour,
        bell_index: bell,
        calendar_label: format!("太陽曆 329 年・初春・{} 日", day + 1),
        bell_label: format!("第{bell}鐘"),
        six_phase_key: format!("d{day}:p{}", hour / 4),
        is_night: !(3..=9).contains(&bell),
    }
}

#[test]
fn c1_all_five_settlements_resolve_ground_floor_shop_tavern_and_exact_innkeeper() {
    let (doc, delta) = fresh_world();
    for cid in [4616_u32, 4613, 4321, 4525, 4073] {
        let (tav_opt, ik_opt) = find_town_tavern_and_keeper(&doc, &delta, cid);
        let tav_id = tav_opt.expect("tavern node must exist");
        let ik_id = ik_opt.expect("innkeeper must exist");
        let nd = doc.nodes.get(&tav_id).expect("node exists");
        assert_eq!(
            nd.kind, "shop",
            "C1: must never hit inn_upstairs (inn_room)"
        );
        assert_eq!(
            delta.actors[&ik_id].role, "酒館掌櫃",
            "C1: must never hit 車行掌櫃"
        );
    }
}

#[test]
fn same_event_produces_divergent_emotion_and_action_across_personas() {
    let (doc, mut delta) = fresh_world();
    let mut bold_id = String::new();
    let mut wary_id = String::new();
    let mut rival_id = String::new();
    for (aid, a) in &delta.actors {
        let inn = derive_innate_persona(a.soul_seed);
        if bold_id.is_empty()
            && inn.temperament.approach_avoid >= 24
            && inn.temperament.ignition <= -10
        {
            bold_id = aid.clone();
        } else if wary_id.is_empty() && inn.temperament.approach_avoid <= -24 {
            wary_id = aid.clone();
        } else if rival_id.is_empty()
            && aid != &bold_id
            && aid != &wary_id
            && !bold_id.is_empty()
            && !wary_id.is_empty()
            && a.household_id != delta.actors[&bold_id].household_id
            && a.household_id != delta.actors[&wary_id].household_id
            && a.work_node != "oi:white_gull_harbor:plaza"
        {
            rival_id = aid.clone();
        }
    }
    assert!(!bold_id.is_empty() && !wary_id.is_empty() && !rival_id.is_empty());

    let shared_node = "oi:white_gull_harbor:plaza";
    delta.relocate_actor(&bold_id, shared_node, 4616, true);
    delta.relocate_actor(&wary_id, shared_node, 4616, true);
    delta.relocate_actor(&rival_id, shared_node, 4616, true);
    // 確保兩人知曉白鷗港相鄰街道供繞行（L1 嚴格驗證 known_nodes）
    if let Some(plaza_nd) = doc.nodes.get(shared_node) {
        for ex in &plaza_nd.exits {
            delta
                .actors
                .get_mut(&wary_id)
                .expect("wary")
                .mind
                .ltm
                .known_nodes
                .insert(ex.to.clone());
        }
    }

    for subj in [&bold_id, &wary_id] {
        let a = delta.actors.get_mut(subj).expect("actor exists");
        for _ in 0..2 {
            a.mind.record_pact_or_trade_evidence(
                &rival_id,
                "宿怨對手",
                10,
                false,
                false,
                "早年分帳毀諾結怨",
            );
        }
        let inn = derive_innate_persona(a.soul_seed);
        assert!(a.mind.appraise_event(
            &inn,
            ConcernCause::RivalEncounter,
            Some(&rival_id),
            None,
            52,
            120,
        ));
    }

    let bold_a = &delta.actors[&bold_id];
    let wary_a = &delta.actors[&wary_id];
    assert_eq!(
        bold_a.mind.top_concern().map(|c| c.emotion),
        Some(PrimaryEmotion::Anger)
    );
    assert_eq!(
        wary_a.mind.top_concern().map(|c| c.emotion),
        Some(PrimaryEmotion::Fear)
    );

    let idx = BellDecisionIndex::build(&delta);
    let (_, ik_opt) = find_town_tavern_and_keeper(&doc, &delta, 4616);
    let bold_dec = evaluate_personalized_desire(
        &doc,
        &delta,
        &idx,
        bold_a,
        &derive_innate_persona(bold_a.soul_seed),
        Some("oi:white_gull_harbor:rusty_anchor_tavern"),
        ik_opt.as_deref(),
        5,
        120,
    );
    let wary_dec = evaluate_personalized_desire(
        &doc,
        &delta,
        &idx,
        wary_a,
        &derive_innate_persona(wary_a.soul_seed),
        Some("oi:white_gull_harbor:rusty_anchor_tavern"),
        ik_opt.as_deref(),
        5,
        120,
    );
    assert_eq!(bold_dec.kind, "confront_chase");
    assert_eq!(wary_dec.kind, "detour_avoid");
}

#[test]
fn grief_and_betrayal_resist_mundane_wash_and_same_day_ratchet_is_capped() {
    let inn = derive_innate_persona(99_887_766);
    let mut mind = CognitiveMind::default();
    let _ = mind.fold_social_memory_from_episode(
        "kin_seed",
        1,
        "kin_bro",
        "親兄長",
        "白鷗港",
        tie_flags::BLOOD_KIN,
        "親兄長",
        true,
        false,
        75,
        -40,
        "分家決裂",
        "",
    );

    // M1 驗證：同一遊戲日（bell 12..18）重複觸發同因事件不會無限墊高強度
    assert!(mind.appraise_event(
        &inn,
        ConcernCause::KinshipRift,
        Some("kin_bro"),
        None,
        55,
        12
    ));
    let first_int = mind.top_concern().expect("concern").intensity;
    for b in 13..20 {
        let _ = mind.appraise_event(
            &inn,
            ConcernCause::KinshipRift,
            Some("kin_bro"),
            None,
            55,
            b,
        );
    }
    assert_eq!(
        mind.top_concern().expect("concern").intensity,
        first_int,
        "M1: same-day repeat must not ratchet intensity up"
    );

    assert!(mind.appraise_event(&inn, ConcernCause::LivelihoodShortfall, None, None, 58, 20));
    let grief_before = mind.stm.affect.grief;
    let fear_before = mind.stm.affect.fear;

    assert!(mind.appraise_event(
        &inn,
        ConcernCause::CraftBreakthrough,
        None,
        Some(ValueAxis::TangibleGain),
        52,
        21,
    ));
    assert!(mind.stm.affect.fear < fear_before);
    assert_eq!(mind.stm.affect.grief, grief_before);

    for _ in 0..60 {
        mind.decay_concerns_and_sync_affect(&inn);
    }
    assert_eq!(mind.stm.focus_concerns.iter().flatten().count(), 0);
}

#[test]
fn h2_ration_ledger_never_mints_free_rations_from_rye_bread_mismatch() {
    let (doc, mut delta) = fresh_world();
    let clk = test_clock_for(5, 5);
    let (tav_opt, ik_opt) = find_town_tavern_and_keeper(&doc, &delta, 4616);
    let tav = tav_opt.unwrap();
    let ik = ik_opt.unwrap();

    // 清空全島產糧戶與掌櫃之 home_ration，僅在掌櫃箱放 rye_bread × 5
    for a in delta.actors.values_mut() {
        a.inn_lockers.clear();
        a.backpack.clear();
    }
    add_carry_item(
        delta
            .actors
            .get_mut(&ik)
            .unwrap()
            .inn_lockers
            .entry(tav.clone())
            .or_default(),
        CarryItem {
            slug: "rye_bread".to_string(),
            name: "黑麥乾糧".to_string(),
            count: 5,
            weight_units: 1,
            ..CarryItem::default()
        },
    );

    let buyer_id = delta
        .actors
        .iter()
        .find(|(id, a)| !id.starts_with("pl:") && *id != &ik && a.copper >= 10)
        .map(|(id, _)| id.clone())
        .unwrap();
    assert!(
        !try_restock_household_larder(&doc, &mut delta, &clk, &buyer_id, Some(&tav), Some(&ik)),
        "H2: must refuse restock when seller has 0 home_ration"
    );
}

#[test]
fn slow_think_chinese_parser_and_strategy_bias_work_without_ascii_json() {
    let parsed = parse_slow_think_chinese_output(
        "定見：米缸見底不能坐困愁城，明日一早去海灘拾木換錢。｜方針：採集貼補",
        GoalStrategyKind::HoneCraft,
    )
    .expect("clean chinese line parses");
    assert_eq!(parsed.1, GoalStrategyKind::ForageSupplement);
    assert!(
        parse_slow_think_chinese_output(
            "定見：我要提升 HP 與 NPC 好感度｜方針：採集貼補",
            GoalStrategyKind::HoneCraft,
        )
        .is_none()
    );
}

#[test]
fn multi_day_45_days_economy_conserves_copper_protects_offline_player_and_avoids_famine() {
    let (doc, mut delta) = fresh_world();
    let init_clk = test_clock_for(1, 4);
    apply_setup_origin(
        &doc,
        &mut delta,
        &init_clk,
        "offline_hunter",
        "山林客",
        "人類",
        "皮甲",
        "forest_hunter",
    )
    .expect("setup offline hunter");
    let bread_before: u32 = delta.actors["pl:offline_hunter"]
        .backpack
        .iter()
        .filter(|i| i.slug == "rye_bread")
        .map(|i| i.count)
        .sum();
    assert!(bread_before >= 1);

    // 連續跑滿 120 個完整遊戲日（1,440 響鐘 = 2,880 ticks），完整覆蓋 R1（D45）與 R2（D120 公庫斜率、掌櫃囤銅、麥酒心事洗白與同屋理論鎖死）！
    for day in 2..122_i64 {
        for bell in 0..12_u8 {
            let clk = test_clock_for(day, bell);
            tick_outer_island_world(&doc, &mut delta, &clk, (clk.real_unix as u64) * 1000);
            tick_outer_island_world(
                &doc,
                &mut delta,
                &clk,
                (clk.real_unix as u64) * 1000 + 5_000,
            );
            assert_eq!(
                delta.conservation_diff(&doc),
                0,
                "copper conservation violated at day={day}, bell={bell}"
            );
        }
    }

    // H1 驗證：離線 120 日之玩家背囊黑麥乾糧 1 塊都不能少！
    let bread_after: u32 = delta.actors["pl:offline_hunter"]
        .backpack
        .iter()
        .filter(|i| i.slug == "rye_bread")
        .map(|i| i.count)
        .sum();
    assert_eq!(
        bread_after, bread_before,
        "H1: offline player backpack rye_bread must never be auto-eaten"
    );

    let npcs: Vec<&UnifiedMindBody> = delta
        .actors
        .values()
        .filter(|a| a.controller_id.is_none() && a.alive)
        .collect();
    let starving_90 = npcs
        .iter()
        .filter(|a| a.mind.stm.affect.hunger >= 90)
        .count();
    assert!(
        starving_90 * 100 / npcs.len() < 10,
        "C1: starving hunger>=90 ratio must stay <10% at Day 120, got {starving_90}/{}",
        npcs.len()
    );
    assert!(
        delta.municipal_treasury_copper >= 21_500,
        "R2-M2: municipal treasury must remain solvent & stable after 120 days, got {}",
        delta.municipal_treasury_copper
    );
    let max_npc_copper = npcs.iter().map(|a| a.copper).max().unwrap_or(0);
    assert!(
        max_npc_copper < 6_000,
        "R2-M2: no single NPC/keeper may hoard >=5% (6,040) of island copper at Day 120, got max={max_npc_copper}"
    );
    let pact_honored_top = npcs
        .iter()
        .filter(|a| {
            a.mind
                .top_concern()
                .is_some_and(|c| c.cause == ConcernCause::PactHonored && c.target_id.is_some())
        })
        .count();
    assert!(
        pact_honored_top * 100 / npcs.len() < 20,
        "R2-M1: tavern ale must not flood PactHonored with keeper into >20% top concerns, got {pact_honored_top}/{}",
        npcs.len()
    );
    let unsold_foragers = npcs
        .iter()
        .filter(|a| {
            a.backpack.iter().any(|i| {
                matches!(i.slug.as_str(), "driftwood" | "herb_bundle" | "iron_ore") && i.count > 0
            })
        })
        .count();
    assert!(
        unsold_foragers * 100 / npcs.len() < 15,
        "R4-H1: harbor craft turnover must prevent unsold foraged backlog (<15%), got {unsold_foragers}/{}",
        npcs.len()
    );
    let written_off_cnt = delta
        .contracts
        .values()
        .filter(|c| c.status == "written_off")
        .count();
    assert!(
        written_off_cnt <= 150,
        "R4-M1: written_off credit blacklist must prevent repeat free-ration tabs, got {written_off_cnt}"
    );

    let has_restock_or_meal_ledger = delta
        .pending_ledger
        .iter()
        .any(|l| l.kind == "larder_restock" || l.kind == "npc_meal");
    let has_stall_levy = delta
        .pending_ledger
        .iter()
        .any(|l| l.kind == "municipal_stall_levy");
    assert!(has_restock_or_meal_ledger && has_stall_levy);

    let mut kind_set: BTreeSet<String> = BTreeSet::new();
    let mut total_intents = 0usize;
    let mut non_work_intents = 0usize;
    for a in &npcs {
        assert!(
            !a.explored_windows
                .iter()
                .any(|w| w.starts_with("npc_forage:")),
            "L3: explored_windows must not accumulate npc_forage keys"
        );
        for it in &a.mind.stm.intentions {
            total_intents += 1;
            kind_set.insert(it.kind.clone());
            if it.kind != "walk_work" && it.kind != "go_home_sleep" {
                non_work_intents += 1;
            }
        }
    }
    assert!(
        kind_set.len() >= 5 && non_work_intents * 100 / total_intents.max(1) >= 15,
        "expected diverse System 1 intentions at Day 45, got kinds={kind_set:?}, non_work={non_work_intents}/{total_intents}"
    );
}

#[test]
fn anti_mind_reading_spam_resistance_and_chronicle_player_masking_hold() {
    let (doc, mut delta) = fresh_world();
    let clk = test_clock_for(20, 5);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "secret_player_99",
        "真實玩家真名",
        "人類",
        "灰斗篷",
        "deck_sailor",
    )
    .expect("setup ok");

    let (target_id, warm_kin_id) = delta
        .actors
        .iter()
        .find_map(|(aid, a)| {
            if aid.starts_with("pl:") {
                return None;
            }
            let wk = a.mind.ltm.social.values().find(|s| {
                s.warmth >= 40 && s.familiarity >= 60 && delta.actors.contains_key(&s.target_id)
            })?;
            Some((aid.clone(), wk.target_id.clone()))
        })
        .expect("npc with warm confidant exists");

    let dock = "oi:white_gull_harbor:dock";
    delta.relocate_actor(&target_id, dock, 4616, true);
    delta.relocate_actor("pl:secret_player_99", dock, 4616, true);

    {
        let npc = delta.actors.get_mut(&target_id).expect("npc");
        let inn = derive_innate_persona(npc.soul_seed);
        assert!(npc.mind.appraise_event(
            &inn,
            ConcernCause::DebtDefaulted,
            Some("pl:secret_player_99"),
            None,
            72,
            245,
        ));
        // M5 驗證：同一日連洗 20 句對話不會刷爆熟稔度突破私密度門檻
        for _ in 0..20 {
            npc.mind.record_talk_exchange(
                "pl:secret_player_99",
                "真實玩家真名",
                clk.game_days,
                "日高四響",
                "最近還好嗎",
                "還行",
                false,
            );
        }
    }

    let chr = build_chronicle_actor_json(&doc, &delta, &target_id).expect("chronicle json");
    let chr_str = chr.to_string();
    assert!(
        !chr_str.contains("真實玩家真名") && chr_str.contains("外來旅人"),
        "M4: /chronicle must mask player display_name to 外來旅人, got: {chr_str}"
    );

    let target_ref = &delta.actors[&target_id];
    let inn = derive_innate_persona(target_ref.soul_seed);
    let mut sv_stranger = inn.arbitrate_dialogue_stance(
        &target_ref.drift,
        &target_ref.role,
        true,
        0,
        0,
        0,
        0,
        "最近還好嗎",
    );
    let mut mem_stranger = Vec::new();
    enrich_dialogue_with_graduated_disclosure(
        &doc,
        &delta,
        &delta.actors["pl:secret_player_99"],
        target_ref,
        &inn,
        "rumor",
        "最近還好嗎",
        &mut sv_stranger,
        &mut mem_stranger,
    );
    assert!(
        mem_stranger.is_empty(),
        "M5: same-day talk spam must not bypass privacy gate"
    );

    let warm_friend = &delta.actors[&warm_kin_id];
    let mut sv_kin = inn.arbitrate_dialogue_stance(
        &target_ref.drift,
        &target_ref.role,
        true,
        2,
        0,
        0,
        0,
        "最近還好嗎",
    );
    let mut mem_kin = Vec::new();
    enrich_dialogue_with_graduated_disclosure(
        &doc,
        &delta,
        warm_friend,
        target_ref,
        &inn,
        "rumor",
        "最近還好嗎",
        &mut sv_kin,
        &mut mem_kin,
    );
    assert!(mem_kin.iter().any(|m| m.contains("心裡懸著這筆債")));

    let _ = build_view_json(&doc, &delta, &delta.actors["pl:secret_player_99"], &clk);
    tick_outer_island_world(&doc, &mut delta, &clk, (clk.real_unix as u64) * 1000);
    assert!(delta.actors["pl:secret_player_99"].route_plan.is_empty());
}

#[test]
fn controller_isolation_ledger_anti_shadow_and_cold_archive_invariants() {
    let (doc, mut delta) = fresh_world();
    let clock = solar_clock_at(1_770_020_000);

    let masha_id = "res:white_gull_harbor:shop_1_0";
    let masha_orig_copper = delta.actors[masha_id].copper;
    assert!(delta.resolve_controller(masha_id).is_none());
    apply_setup_origin(
        &doc,
        &mut delta,
        &clock,
        masha_id,
        "冒名客",
        "人類",
        "",
        "deck_sailor",
    )
    .expect("isolated actor created under pl: prefix");
    assert_eq!(
        delta.resolve_controller(masha_id).as_deref(),
        Some("pl:res:white_gull_harbor:shop_1_0")
    );
    assert_eq!(delta.actors[masha_id].display_name, "瑪莎");
    assert_eq!(delta.actors[masha_id].copper, masha_orig_copper);

    apply_setup_origin(
        &doc,
        &mut delta,
        &clock,
        "municipal_treasury",
        "假公庫",
        "人類",
        "",
        "smith_apprentice",
    )
    .expect("setup");
    let treasury_before = delta.municipal_treasury_copper;
    assert!(delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "test_tax",
        masha_id,
        "municipal_treasury",
        5,
        "測試公庫規費".to_string(),
    ));
    assert_eq!(delta.municipal_treasury_copper, treasury_before + 5);
    assert_eq!(delta.actors["pl:municipal_treasury"].copper, 0);
    assert_eq!(delta.conservation_diff(&doc), 0);

    {
        let a = delta
            .actors
            .get_mut("pl:municipal_treasury")
            .expect("actor exists");
        for idx in 0..18 {
            a.mind.push_turning_point(
                &format!("tp:{idx}"),
                idx,
                &format!("第 {idx} 章歷練定見"),
                "hone_craft",
            );
        }
        a.mark_mutated();
        assert_eq!(a.mind.ltm.turning_points.len(), 16);
        assert_eq!(a.mind.ltm.cold_outbox.len(), 2);
    }
    let batch = prepare_commit_batch(&doc, &delta, false).expect("batch ok");
    assert_eq!(batch.tp_archives.len(), 2);
    ack_commit_batch(&mut delta, &batch);
    assert!(
        delta.actors["pl:municipal_treasury"]
            .mind
            .ltm
            .cold_outbox
            .is_empty()
    );
}

#[test]
fn r2_h1_m3_m5_cohabiting_rivals_recall_privacy_and_forage_strategy_hold() {
    let (doc, mut delta) = fresh_world();

    // 1. R2-H1：同屋嫌隙家人在 3 日內絕不釘死在屋裡每響互吼，雙方白天照常出門上工！
    let pair_opt = delta.actors.iter().find_map(|(aid, a)| {
        if aid.starts_with("pl:") || a.home_node == a.work_node {
            return None;
        }
        let co_id = a
            .mind
            .ltm
            .social
            .values()
            .find(|s| s.cohabiting && s.warmth <= -22 && delta.actors.contains_key(&s.target_id))?
            .target_id
            .clone();
        Some((aid.clone(), co_id))
    });
    let (h_a, h_b) = pair_opt.expect("estranged cohabiting pair exists in world_weave");
    let home_nd = delta.actors[&h_a].home_node.clone();
    let home_cell = doc.nodes.get(&home_nd).map_or(4616, |n| n.cell_id);
    delta.relocate_actor(&h_a, &home_nd, home_cell, true);
    delta.relocate_actor(&h_b, &home_nd, home_cell, true);

    let work_nd = delta.actors[&h_a].work_node.clone();
    let mut work_bells_present = 0usize;
    let mut mutual_confronts = 0usize;
    let mut slow_popped = 0usize;
    for day in 10..14_i64 {
        for bell in 0..12_u8 {
            let clk = test_clock_for(day, bell);
            for sub in 0..8_u64 {
                tick_outer_island_world(
                    &doc,
                    &mut delta,
                    &clk,
                    (clk.real_unix as u64) * 1000 + sub * 2_000,
                );
            }
            if (4..=7).contains(&bell) && delta.actors[&h_a].node_id == work_nd {
                work_bells_present += 1;
            }
            if bell == 9 {
                mutual_confronts += delta.actors[&h_a]
                    .mind
                    .stm
                    .day_episodes
                    .iter()
                    .filter(|e| {
                        e.event_id.contains("confront_scene")
                            && (e.actor_id == h_a || e.actor_id == h_b)
                    })
                    .count();
            }
            if let Some(cand) =
                super::slow_think::pop_top_slow_think_candidate(&doc, &mut delta, day)
            {
                slow_popped += 1;
                let owes = delta.contracts.values().any(|c| {
                    c.debtor_id == cand.actor_id && (c.status == "open" || c.status == "defaulted")
                });
                if let Some(act) = delta.actors.get_mut(&cand.actor_id) {
                    let wear = act.bound_weapon.wear_stage;
                    if wear == 0 {
                        assert_ne!(
                            cand.fallback_strategy,
                            GoalStrategyKind::HoneCraft,
                            "R3-M2: must never assign unachievable HoneCraft when wear_stage == 0"
                        );
                    }
                    act.mind.apply_goal_strategy_with_snapshot(
                        cand.fallback_strategy,
                        cand.focus_target_id.clone(),
                        day,
                        3,
                        wear,
                        owes,
                    );
                }
            }
        }
    }
    assert!(
        work_bells_present >= 8,
        "R3-H1: estranged cohabiting resident must attend work_node (no detour livelock), got {work_bells_present}/16"
    );
    assert!(
        mutual_confronts <= 1,
        "R2-H1: mutual confront_scene between cohabiting rivals must be capped, got {mutual_confronts}"
    );
    assert!(
        slow_popped <= 32,
        "R3-M2: slow-think queue must not flood from bystander noise, got {slow_popped} over 4 days"
    );

    // 2. R2-M3：經 `recall_relevant_for_utterance` 正式路徑撈出之私密欠債情節，對生面孔必被閘門攔截並切入 GuardedEvade！
    let setup_clk = test_clock_for(15, 5);
    apply_setup_origin(
        &doc,
        &mut delta,
        &setup_clk,
        "stranger_probe",
        "過路生客",
        "人類",
        "布衣",
        "deck_sailor",
    )
    .expect("setup stranger");
    {
        let npc = delta.actors.get_mut(&h_a).expect("npc");
        npc.mind.ingest_historical_episode(
            "priv:debt:99",
            14,
            &home_nd,
            &h_b,
            "債主",
            super::mind::EvidenceTier::DirectWitness,
            95,
            100,
            80,
            3,
            "自從親歷「手頭吃緊欠下三枚銅幣飯債被寬限兩日」，心裡一直發愁。".to_string(),
        );
        let inn = derive_innate_persona(npc.soul_seed);
        assert!(npc.mind.appraise_event(
            &inn,
            ConcernCause::DebtDefaulted,
            Some(&h_b),
            None,
            75,
            180,
        ));
    }
    let npc_ref = &delta.actors[&h_a];
    let inn = derive_innate_persona(npc_ref.soul_seed);
    let mut recalled =
        npc_ref
            .mind
            .recall_relevant_for_utterance("pl:stranger_probe", "最近還好嗎", "rumor", 15);
    let mut sv = inn.arbitrate_dialogue_stance(
        &npc_ref.drift,
        &npc_ref.role,
        false,
        0,
        0,
        0,
        0,
        "最近還好嗎",
    );
    enrich_dialogue_with_graduated_disclosure(
        &doc,
        &delta,
        &delta.actors["pl:stranger_probe"],
        npc_ref,
        &inn,
        "rumor",
        "最近還好嗎",
        &mut sv,
        &mut recalled,
    );
    assert!(
        !recalled
            .iter()
            .any(|m| m.contains("欠") || m.contains("債")),
        "R2-M3: stranger recall must never leak private debt episodes, got {recalled:?}"
    );
    assert_eq!(sv.stance, super::persona::DialogueHardStance::GuardedEvade);

    // 3. R2-M5：`ForageSupplement` 未實際執行採集前絕不假達成，執行採集後方可結案！
    let mut m = CognitiveMind::default();
    m.stm.last_forage_window = 40;
    m.apply_goal_strategy_with_snapshot(GoalStrategyKind::ForageSupplement, None, 10, 4, 0, false);
    m.tick_goal_strategy_expiry(11, false, 8, 20, 0, &inn, 132);
    assert!(
        m.stm.goal_strategy.is_some(),
        "R2-M5: ForageSupplement must not resolve without actual forage action"
    );
    m.stm.last_forage_window = 45;
    m.tick_goal_strategy_expiry(11, false, 8, 20, 0, &inn, 133);
    assert!(
        m.stm.goal_strategy.is_none(),
        "R2-M5: ForageSupplement resolves once forage occurred and larder/copper recovered"
    );

    // 4. R3-L1：`written_off` 黑冊欠款仍屬 `has_unsettled_or_written_off_debt == true`，絕不誤判 ClearDebt 達成！
    m.apply_goal_strategy_with_snapshot(GoalStrategyKind::ClearDebt, None, 12, 4, 0, true);
    m.tick_goal_strategy_expiry(13, true, 8, 20, 0, &inn, 156);
    assert!(
        m.stm.goal_strategy.is_some(),
        "R3-L1: ClearDebt must never fire PactHonored when debt is written_off/unpaid"
    );

    // 5. R5-M2：貧困家戶「賒糧 → 逾期催討一回 → 再逾限轉黑冊 → 拒賒攔截」完整狀態機實證！
    let (tav_opt, ik_opt) = find_town_tavern_and_keeper(&doc, &delta, home_cell);
    let tav = tav_opt.expect("tavern");
    let ik = ik_opt.expect("keeper");
    let broke_c = delta.actors[&h_a].copper;
    if broke_c > 0 {
        assert!(delta.transfer_copper(
            20,
            4,
            "test_drain",
            &h_a,
            "municipal_treasury",
            broke_c,
            "清空銅幣測試貧戶賒糧".to_string()
        ));
    }
    let hh_key = super::household_economy::household_bucket_key(&delta.actors[&h_a]);
    for act in delta.actors.values_mut() {
        if super::household_economy::household_bucket_key(act) == hh_key {
            act.inn_lockers.clear();
            act.backpack.clear();
        }
    }
    delta.contracts.retain(|_, c| c.debtor_id != h_a);
    let c20 = test_clock_for(20, 5);
    assert!(
        try_restock_household_larder(&doc, &mut delta, &c20, &h_a, Some(&tav), Some(&ik)),
        "R5-M2: broke empty-larder household opens 1st larder_tab"
    );
    assert!(
        delta
            .contracts
            .values()
            .any(|c| c.debtor_id == h_a && c.status == "open")
    );
    super::desire_effects::settle_due_contracts(&doc, &mut delta, &test_clock_for(23, 5));
    assert!(delta.contracts.values().any(|c| c.debtor_id == h_a
        && c.status == "defaulted"
        && c.note_zh.contains("〔催討一回〕")));
    super::desire_effects::settle_due_contracts(&doc, &mut delta, &test_clock_for(25, 5));
    assert!(
        delta
            .contracts
            .values()
            .any(|c| c.debtor_id == h_a && c.status == "written_off")
    );
    for act in delta.actors.values_mut() {
        if super::household_economy::household_bucket_key(act) == hh_key {
            act.inn_lockers.clear();
            act.backpack.clear();
        }
    }
    assert!(
        !try_restock_household_larder(
            &doc,
            &mut delta,
            &test_clock_for(26, 5),
            &h_a,
            Some(&tav),
            Some(&ik)
        ),
        "R5-M2: written_off debtor must be strictly rejected from opening repeat credit tabs"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

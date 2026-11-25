//! 審辯室 Room `7eee7478-1493-4faf-b7e3-12de460402e3`「活人、真人玩家同構（`NPC ≡ 真人玩家`）」六大支柱整合與長程守恆驗證測試。

use super::actions::apply_world_action;
use super::clock_and_lang::SolarClockSnapshot;
use super::engine::load_static_world_doc;
use super::explore_and_combat::{apply_explore, resolve_skirmish_turn, trigger_non_lethal_defeat};
use super::movement::{add_carry_item, apply_setup_origin};
use super::pacts_and_peers::{
    accept_escrow_pact, apply_peer_assist_skirmish, apply_peer_consensual_spar,
    apply_peer_direct_speech, apply_peer_item_trade, fulfill_escrow_pact, propose_escrow_pact,
    settle_expired_escrow_pacts_as_world_events,
};
use super::phase_and_vessels::{
    ensure_chaos_rune_pool_seeded, equip_living_vessel, forge_living_vessel,
};
use super::sim_tick::{seed_genesis_actors_once, tick_outer_island_world};
use super::types::{CarryItem, IslandWorldDelta, SkirmishState};
use super::unified_kernel::build_new_homestead;

pub(super) fn seeded_world() -> (super::types::WorldStaticDoc, IslandWorldDelta) {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    ensure_chaos_rune_pool_seeded(&mut delta);
    assert_eq!(delta.conservation_diff(&doc), 0);
    (doc, delta)
}

pub(super) fn test_clock_for(day: i64, bell: u8) -> SolarClockSnapshot {
    let hour = u32::from(bell) * 2;
    SolarClockSnapshot {
        real_unix: 1_800_000_000 + day * 3_600 + i64::from(bell) * 300,
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
fn martial_phase_separates_private_skill_from_witnessed_renown_and_exposes_false_boast() {
    let (doc, mut delta) = seeded_world();
    let clock = test_clock_for(10, 5);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clock,
        "hero_solo",
        "孤影客",
        "人類",
        "",
        "forest_hunter",
    )
    .unwrap();
    apply_setup_origin(
        &doc,
        &mut delta,
        &clock,
        "wit_peer",
        "見證鐵漢",
        "獸人",
        "",
        "deck_sailor",
    )
    .unwrap();

    // 1. 獨自在無人山徑連勝兩種不同對手 -> 獲得私有武相（物理生效），但因現場零目擊者，街頭公認名號必須為 None！
    let solo_node = "oi:cell4529:0";
    delta.relocate_actor("pl:hero_solo", solo_node, 4529, true);
    if let Some(occ) = delta.node_occupants.get(solo_node).cloned() {
        for oid in occ {
            if oid != "pl:hero_solo" {
                delta.relocate_actor(&oid, "oi:white_gull_harbor:dock", 4616, true);
            }
        }
    }

    delta
        .actors
        .get_mut("pl:hero_solo")
        .unwrap()
        .active_skirmish = Some(SkirmishState {
        foe_slug: "grey_ear_goblin".to_string(),
        foe_name: "灰耳哥布林斥候".to_string(),
        can_speak_goblin: false,
        foe_stagger: 0,
        player_guard: true,
        round_idx: 1,
    });
    resolve_skirmish_turn(&doc, &mut delta, &clock, "pl:hero_solo", "skirmish_heavy")
        .expect("win 1");

    delta
        .actors
        .get_mut("pl:hero_solo")
        .unwrap()
        .active_skirmish = Some(SkirmishState {
        foe_slug: "cave_lizard".to_string(),
        foe_name: "嗜石岩蜥".to_string(),
        can_speak_goblin: false,
        foe_stagger: 1,
        player_guard: true,
        round_idx: 1,
    });
    resolve_skirmish_turn(&doc, &mut delta, &clock, "pl:hero_solo", "skirmish_heavy")
        .expect("win 2");

    {
        let h = delta.actors.get("pl:hero_solo").unwrap();
        assert!(
            h.mind.ltm.mastery.private_martial_phase.is_some(),
            "cross-context victories must unlock private physical martial phase"
        );
        assert!(
            h.mind.ltm.mastery.witnessed_martial_phase.is_none(),
            "unwitnessed victory must NOT grant street renown"
        );
    }

    // 2. 回到有目擊者的碼頭當眾誇口「我斬了巨蛛」 -> 當場被在場者拆穿！
    delta.relocate_actor("pl:hero_solo", "oi:white_gull_harbor:dock", 4616, true);
    delta.relocate_actor("pl:wit_peer", "oi:white_gull_harbor:dock", 4616, true);
    let wit_tok = delta
        .actors
        .get("pl:wit_peer")
        .unwrap()
        .public_actor_token();
    let (_masked, callout) = apply_peer_direct_speech(
        &doc,
        &mut delta,
        &clock,
        "pl:hero_solo",
        &wit_tok,
        "昨天在坑底我斬了巨蛛，天下無敵！",
    )
    .expect("speech ok");
    assert!(
        callout.is_some_and(|c| c.contains("拆穿")),
        "unwitnessed boast must be called out by co-present witness"
    );

    // 3. 真人雙段切磋確認：先邀約（不強制扣對方疲勞），對方拱手應戰後正式過招見證武相！
    let solo_tok = delta
        .actors
        .get("pl:hero_solo")
        .unwrap()
        .public_actor_token();
    let fat_before = delta.actors.get("pl:wit_peer").unwrap().fatigue;
    let inv_msg = apply_peer_consensual_spar(&doc, &mut delta, &clock, "pl:hero_solo", &wit_tok)
        .expect("spar invite ok");
    assert!(inv_msg.contains("邀約"));
    assert_eq!(delta.actors.get("pl:wit_peer").unwrap().fatigue, fat_before);
    let _ = apply_peer_consensual_spar(&doc, &mut delta, &clock, "pl:wit_peer", &solo_tok)
        .expect("spar accept ok");
    let either_witnessed = delta
        .actors
        .get("pl:hero_solo")
        .unwrap()
        .mind
        .ltm
        .mastery
        .witnessed_martial_phase
        .is_some()
        || delta
            .actors
            .get("pl:wit_peer")
            .unwrap()
            .mind
            .ltm
            .mastery
            .martial_bouts
            >= 1;
    assert!(either_witnessed);
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn primordial_spider_never_respawns_and_chaos_rune_pool_is_strictly_bounded() {
    let (doc, mut delta) = seeded_world();
    let clock_d10 = test_clock_for(10, 5);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clock_d10,
        "delver",
        "探窟者",
        "矮人",
        "",
        "smith_apprentice",
    )
    .expect("setup");

    let lair = "oi:dng_pit:lair";
    let altar = "oi:dng_pit:altar";
    delta.relocate_actor("pl:delver", lair, 4246, true);
    let msg1 = apply_explore(&doc, &mut delta, &clock_d10, "pl:delver").expect("lair 1");
    assert!(msg1.contains("吞石巨蛛"));

    // 沉肩護腕後重擊斬殺元祖吞石巨蛛
    resolve_skirmish_turn(&doc, &mut delta, &clock_d10, "pl:delver", "skirmish_guard").unwrap();
    resolve_skirmish_turn(&doc, &mut delta, &clock_d10, "pl:delver", "skirmish_heavy").unwrap();
    assert!(delta.first_spider_slain.is_some());
    assert!(
        !delta.actors["pl:delver"]
            .known_rumors
            .contains("pit_cleared"),
        "R2-H3: clearing lair must NOT mark pit_cleared before visiting altar"
    );

    // 撬取一塊原石 -> chaos_rune_pool 由 8 減為 7，且此刻始標記 pit_cleared
    let pool_before = delta.chaos_rune_pool;
    delta.relocate_actor("pl:delver", altar, 4246, true);
    let pry_msg = apply_explore(&doc, &mut delta, &clock_d10, "pl:delver").expect("pry");
    assert!(pry_msg.contains("古符文原石"));
    assert_eq!(delta.chaos_rune_pool, pool_before - 1);
    assert!(
        delta.actors["pl:delver"]
            .known_rumors
            .contains("pit_cleared")
    );

    // 過 4 天（舊版 +3 天早就重刷了！）：巢穴必須仍為空巢！過 9 天後則為「空巢易主（嗜石岩蜥）」，元祖巨蛛永不復活！
    let clock_d14 = test_clock_for(14, 5);
    delta.relocate_actor("pl:delver", lair, 4246, true);
    let msg_d14 = apply_explore(&doc, &mut delta, &clock_d14, "pl:delver").expect("d14 empty");
    assert!(
        msg_d14.contains("伏誅")
            && delta
                .actors
                .get("pl:delver")
                .unwrap()
                .active_skirmish
                .is_none()
    );

    let clock_d20 = test_clock_for(20, 5);
    let move_ambush = super::movement::apply_move_step(
        &doc,
        &mut delta,
        &clock_d20,
        "pl:delver",
        altar,
        2_000_000,
    )
    .expect("d20 move ambush");
    assert!(
        move_ambush.contains("空巢易主") && move_ambush.contains("嗜石岩蜥"),
        "C2: moving lair->altar after 8 days must spawn cave_lizard, never stone_spider: {move_ambush}"
    );
    resolve_skirmish_turn(&doc, &mut delta, &clock_d20, "pl:delver", "skirmish_guard").unwrap();
    let liz_win =
        resolve_skirmish_turn(&doc, &mut delta, &clock_d20, "pl:delver", "skirmish_heavy").unwrap();
    assert!(
        liz_win.contains("嗜石岩蜥厚鱗皮") && !liz_win.contains("吞石巨蛛厚甲殼"),
        "C2: cave_lizard must drop lizard_scale, not spider_carapace"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn living_vessel_forging_peer_trade_homestead_building_and_escrow_pact_conserve_copper() {
    let (doc, mut delta) = seeded_world();
    let clock = test_clock_for(12, 5);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clock,
        "smith_p",
        "鐵手阿成",
        "矮人",
        "",
        "smith_apprentice",
    )
    .unwrap();
    apply_setup_origin(
        &doc,
        &mut delta,
        &clock,
        "runner_p",
        "快腳阿風",
        "人類",
        "",
        "fallen_peddler",
    )
    .unwrap();
    for (to, amt) in [("pl:smith_p", 160), ("pl:runner_p", 40)] {
        assert!(delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "test_seed",
            "municipal_treasury",
            to,
            amt,
            "測試撥款".to_string()
        ));
    }

    // 1. 鐵匠鋪開爐鍛打唯一落款活器物，並與同場真人旅人亮物報價交割、換握上手
    let forge_node = doc
        .nodes
        .iter()
        .find(|(_, n)| n.name.contains("鍛鐵"))
        .map(|(k, _)| k.clone())
        .expect("forge node");
    delta.relocate_actor("pl:smith_p", &forge_node, 4616, true);
    delta.relocate_actor("pl:runner_p", &forge_node, 4616, true);
    {
        let sm = delta.actors.get_mut("pl:smith_p").unwrap();
        add_carry_item(
            &mut sm.backpack,
            CarryItem {
                slug: "iron_ore".to_string(),
                name: "粗鐵礦石".to_string(),
                count: 2,
                weight_units: 3,
                ..CarryItem::default()
            },
        );
        add_carry_item(
            &mut sm.backpack,
            CarryItem {
                slug: "spider_carapace".to_string(),
                name: "吞石巨蛛厚甲殼".to_string(),
                count: 1,
                weight_units: 3,
                ..CarryItem::default()
            },
        );
    }
    let f_msg = forge_living_vessel(&doc, &mut delta, &clock, "pl:smith_p").expect("forge ok");
    assert!(f_msg.contains("鍛・甲脊獵刃"));

    let v_slug = delta
        .actors
        .get("pl:smith_p")
        .unwrap()
        .backpack
        .iter()
        .find(|i| i.vessel_meta.is_some())
        .unwrap()
        .slug
        .clone();
    let runner_tok = delta
        .actors
        .get("pl:runner_p")
        .unwrap()
        .public_actor_token();
    apply_peer_item_trade(
        &mut delta,
        &clock,
        "pl:smith_p",
        "offer_to_peer",
        &format!("{runner_tok}|{v_slug}|18"),
    )
    .expect("offer ok");
    apply_peer_item_trade(&mut delta, &clock, "pl:runner_p", "accept_peer_offer", "")
        .expect("accept offer ok");
    equip_living_vessel(&mut delta, "pl:runner_p", &v_slug).expect("equip ok");
    assert!(
        delta
            .actors
            .get("pl:runner_p")
            .unwrap()
            .bound_weapon
            .name
            .contains("鐵手阿成鍛・甲脊獵刃")
    );
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 2. 銀號保人託管契：禁止環形轉包、正常履約結清、以及逾期掛牌退還押金（零無痕系統重置）
    let rec_npc = "res:west_strait_ferry:ferry_row_2_0";
    propose_escrow_pact(
        &doc,
        &mut delta,
        &clock,
        "pl:smith_p",
        "courier_errand",
        rec_npc,
        6,
    )
    .expect("propose courier");
    assert_eq!(delta.contract_escrow_copper, 6);
    assert_eq!(delta.conservation_diff(&doc), 0);

    let pact_id = delta
        .contracts
        .keys()
        .find(|k| k.starts_with("pact:courier_errand:"))
        .cloned()
        .unwrap();
    accept_escrow_pact(&doc, &mut delta, &clock, "pl:runner_p", &pact_id).expect("accept courier");

    // 測試環形轉包攔截：runner_p 正欠 smith_p 未結託付時，smith_p 反向接 runner_p 的託付必須被拒！
    propose_escrow_pact(
        &doc,
        &mut delta,
        &clock,
        "pl:runner_p",
        "supply_commission",
        "iron_ore",
        5,
    )
    .expect("propose rev");
    let rev_pact_id = delta
        .contracts
        .keys()
        .find(|k| k.starts_with("pact:supply_commission:"))
        .cloned()
        .unwrap();
    let cyc_err = accept_escrow_pact(&doc, &mut delta, &clock, "pl:smith_p", &rev_pact_id)
        .expect_err("circular subcontract must be rejected");
    assert!(cyc_err.contains("反向未結託付"));

    // runner_p 走到西峽渡口當面送交家書領賞
    let rec_node = delta.actors.get(rec_npc).unwrap().node_id.clone();
    delta.relocate_actor("pl:runner_p", &rec_node, 4800, true);
    fulfill_escrow_pact(&doc, &mut delta, &clock, "pl:runner_p", &pact_id)
        .expect("fulfill courier");
    assert_eq!(delta.contract_escrow_copper, 5); // 僅剩 rev_pact_id 的 5 銅

    // 推進 6 天使 rev_pact_id 逾期 -> 銀號自動退還 5 銅，contract_escrow 歸零，守恆恆等 0
    let clock_late = test_clock_for(18, 5);
    settle_expired_escrow_pacts_as_world_events(&doc, &mut delta, &clock_late);
    assert_eq!(delta.contract_escrow_copper, 0);
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 3. 營造自立門戶石木新宅（破除 6 棟空屋硬頂）＋ 同場拔刀助戰 vs 袖手旁觀
    delta.relocate_actor("pl:smith_p", "oi:white_gull_harbor:plaza", 4616, true);
    {
        let sm = delta.actors.get_mut("pl:smith_p").unwrap();
        add_carry_item(
            &mut sm.backpack,
            CarryItem {
                slug: "driftwood".to_string(),
                name: "異國刻紋漂流木".to_string(),
                count: 2,
                weight_units: 2,
                ..CarryItem::default()
            },
        );
        add_carry_item(
            &mut sm.backpack,
            CarryItem {
                slug: "iron_ore".to_string(),
                name: "粗鐵礦石".to_string(),
                count: 1,
                weight_units: 3,
                ..CarryItem::default()
            },
        );
    }
    build_new_homestead(&doc, &mut delta, &clock_late, "pl:smith_p").expect("homestead built");
    let built_node = delta
        .actors
        .get("pl:smith_p")
        .unwrap()
        .owned_house_node
        .clone()
        .expect("owned_house_node set");
    assert_ne!(
        built_node, "oi:white_gull_harbor:plaza",
        "M1: homestead must anchor to residential alley, not open plaza"
    );
    assert!(
        delta
            .actors
            .get("pl:smith_p")
            .unwrap()
            .household_id
            .starts_with("hh:white_gull_harbor:homestead:"),
        "M1: household_id must preserve settlement slug"
    );
    assert_eq!(
        delta
            .actors
            .get("pl:smith_p")
            .unwrap()
            .backpack
            .iter()
            .filter(|i| i.slug == "driftwood")
            .count(),
        0,
        "H1: all 2 driftwoods must be consumed"
    );
    apply_world_action(&doc, &mut delta, &clock_late, "pl:smith_p", "rest_bed", "")
        .expect("can sleep in newly built homestead");

    // 測試拔刀助戰與見死不救
    delta.relocate_actor("pl:runner_p", &built_node, 4616, true);
    delta.actors.get_mut("pl:smith_p").unwrap().active_skirmish = Some(SkirmishState {
        foe_slug: "grey_ear_goblin".to_string(),
        foe_name: "流竄灰耳斥候".to_string(),
        can_speak_goblin: false,
        foe_stagger: 0,
        player_guard: false,
        round_idx: 1,
    });
    let sm_tok = delta.actors.get("pl:smith_p").unwrap().public_actor_token();
    apply_peer_assist_skirmish(&doc, &mut delta, &clock_late, "pl:runner_p", &sm_tok)
        .expect("assist ok");
    assert!(
        delta
            .actors
            .get("pl:smith_p")
            .unwrap()
            .active_skirmish
            .is_none()
    );

    // 見死不救測試
    let warm_before = delta
        .actors
        .get("pl:smith_p")
        .unwrap()
        .mind
        .ltm
        .social
        .get("pl:runner_p")
        .map_or(0, |s| s.warmth);
    trigger_non_lethal_defeat(
        &mut delta,
        &clock_late,
        "pl:smith_p",
        &built_node,
        "嗜石岩蜥",
    )
    .expect("defeat ok");
    let warm_after = delta
        .actors
        .get("pl:smith_p")
        .unwrap()
        .mind
        .ltm
        .social
        .get("pl:runner_p")
        .map_or(0, |s| s.warmth);
    assert!(warm_after < warm_before, "idle bystander must lose warmth");
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn sixty_day_autonomous_island_emerges_living_player_behaviors_without_breaking_economy() {
    let (doc, mut delta) = seeded_world();

    // 連續推進 60 遊戲日（720 響鐘，每響鐘 2 拍加速驗證）
    for day in 1..=60 {
        for bell in 0..12_u8 {
            let clk1 = test_clock_for(day, bell);
            let ms1 = (clk1.real_unix as u64) * 1000;
            tick_outer_island_world(&doc, &mut delta, &clk1, ms1);
            let mut clk2 = clk1.clone();
            clk2.real_unix += 120;
            tick_outer_island_world(&doc, &mut delta, &clk2, ms1 + 120_000);
        }
        if day % 5 == 0 {
            assert_eq!(
                delta.conservation_diff(&doc),
                0,
                "copper conservation violated at day {day}"
            );
        }
    }

    assert_eq!(delta.conservation_diff(&doc), 0);
    let active_escrow_sum: i64 = delta
        .contracts
        .values()
        .filter(|c| c.status == "escrowed" || c.status == "in_progress")
        .map(|c| c.amount_copper)
        .sum();
    assert_eq!(
        delta.contract_escrow_copper, active_escrow_sum,
        "escrow ledger balance must equal sum of active escrow contracts"
    );

    let forged_or_traded_vessels: usize = delta
        .actors
        .values()
        .map(|a| {
            usize::from(
                a.bound_weapon.name.contains("鍛・") || a.bound_weapon.name.contains("製・"),
            ) + a
                .backpack
                .iter()
                .filter(|i| i.vessel_meta.is_some())
                .count()
        })
        .sum();
    let practiced_actors = delta
        .actors
        .values()
        .filter(|a| a.mind.ltm.mastery.martial_bouts > 0 || a.mind.ltm.mastery.craft_works > 0)
        .count();
    let pacts_created = delta
        .contracts
        .keys()
        .filter(|k| k.starts_with("pact:"))
        .count();
    let starving = delta
        .actors
        .values()
        .filter(|a| a.controller_id.is_none() && a.mind.stm.affect.hunger >= 92)
        .count();

    assert!(forged_or_traded_vessels >= 1 && practiced_actors >= 3 && pacts_created >= 1);
    assert!(
        starving <= 15,
        "town livelihood supply chain must not collapse (starving={starving})"
    );
}

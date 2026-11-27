//! 審辯室 Room `7eee7478` R1～R3 無狀態盲審迴歸與跨響鐘 `tick_outer_island_world` 整合驗證測試。

use super::living_player_tests::{seeded_world, test_clock_for};
use super::movement::{add_carry_item, apply_setup_origin};
use super::pacts_and_peers::{
    accept_escrow_pact, apply_peer_item_trade, fulfill_escrow_pact, propose_escrow_pact,
    settle_expired_escrow_pacts_as_world_events,
};
use super::phase_and_vessels::{equip_living_vessel, forge_living_vessel};
use super::sim_tick::tick_outer_island_world;
use super::types::CarryItem;
use super::unified_kernel::build_new_homestead;

#[test]
fn r1_blind_audit_regressions_privacy_offline_guard_and_expedition_bfs() {
    let (doc, mut delta) = seeded_world();
    let clock = test_clock_for(15, 5);

    // C1: 動態解析斷橋暗門（oi:cell4529:0）＋完整門戶與地城尋路
    let (bridge_nid, hid) =
        super::movement::find_hidden_entrance_for_rumor(&doc, "pit_rumor").unwrap();
    assert_eq!(bridge_nid, "oi:cell4529:0");
    let herb_nid = doc
        .nodes
        .iter()
        .find(|(_, n)| n.name.contains("草藥"))
        .map(|(k, _)| k.clone())
        .unwrap();
    let mut unlocked = std::collections::BTreeSet::new();
    assert!(
        super::movement::shortest_expedition_path(
            &doc,
            "oi:white_gull_harbor:dock",
            &herb_nid,
            &unlocked
        )
        .is_some()
    );
    assert!(
        super::movement::shortest_expedition_path(&doc, bridge_nid, &hid.target, &unlocked)
            .is_none()
    );
    unlocked.insert(bridge_nid.to_string());
    assert!(
        super::movement::shortest_expedition_path(&doc, bridge_nid, "oi:dng_pit:lair", &unlocked)
            .is_some()
    );

    // C3 & H9: 離線真人玩家不可被強購背囊；四款起手兵刃換下後皆自動記名禁售
    for (pid, nm, rc, og) in [
        ("p_sleep", "初用名甲", "精靈", "forest_hunter"),
        ("p_sleep", "新名號乙", "精靈", "forest_hunter"),
        ("p_looter", "路過買客", "人類", "deck_sailor"),
    ] {
        apply_setup_origin(&doc, &mut delta, &clock, pid, nm, rc, "", og).unwrap();
    }
    for (to, amt) in [("pl:p_looter", 50), ("pl:p_sleep", 30)] {
        assert!(delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "seed",
            "municipal_treasury",
            to,
            amt,
            "測試".to_string()
        ));
    }

    let sleep_tok = delta.actors.get("pl:p_sleep").unwrap().public_actor_token();
    delta.actors.get_mut("pl:p_sleep").unwrap().last_active_unix = 0; // 離線逾數年
    let c3_err = apply_peer_item_trade(
        &mut delta,
        &clock,
        "pl:p_looter",
        "buy_from_peer",
        &format!("{sleep_tok}|rye_bread"),
    )
    .expect_err("C3: offline human player backpack must NEVER be force-bought");
    assert!(c3_err.contains("同道旅人"));

    // C4 & H9: 玩家鍛器賣予 NPC 並換握 -> 換下之舊鹿角獵刀具記名防洗鐵，且 `/api/chronicle/actor` 遮罩玩家新舊名與帳號！
    let forge_nid = doc
        .nodes
        .iter()
        .find(|(_, n)| n.name.contains("鍛鐵"))
        .map(|(k, _)| k.clone())
        .unwrap();
    delta.relocate_actor("pl:p_sleep", &forge_nid, 4616, true);
    {
        let p = delta.actors.get_mut("pl:p_sleep").unwrap();
        add_carry_item(
            &mut p.backpack,
            CarryItem {
                slug: "iron_ore".to_string(),
                name: "粗鐵礦石".to_string(),
                count: 2,
                weight_units: 3,
                ..CarryItem::default()
            },
        );
    }
    forge_living_vessel(&doc, &mut delta, &clock, "pl:p_sleep").unwrap();
    let v_slug = delta.actors["pl:p_sleep"]
        .backpack
        .iter()
        .find(|i| i.vessel_meta.is_some())
        .unwrap()
        .slug
        .clone();
    equip_living_vessel(&mut delta, "pl:p_sleep", &v_slug).unwrap();
    let old_knife = delta.actors["pl:p_sleep"]
        .backpack
        .iter()
        .find(|i| i.slug == "old_hunting_knife")
        .expect("old starter knife in bag");
    assert!(
        old_knife.bound_owner.is_some(),
        "H9: starter hunting knife must retain bound_owner"
    );

    // 再鍛一把賣給在場 NPC 並換握，驗證 `/api/chronicle/actor` 零洩漏玩家真名／舊名
    {
        let p = delta.actors.get_mut("pl:p_sleep").unwrap();
        p.fatigue = 0;
        add_carry_item(
            &mut p.backpack,
            CarryItem {
                slug: "driftwood".to_string(),
                name: "異國刻紋漂流木".to_string(),
                count: 2,
                weight_units: 2,
                ..CarryItem::default()
            },
        );
    }
    forge_living_vessel(&doc, &mut delta, &clock, "pl:p_sleep").unwrap();
    let v2_slug = delta.actors["pl:p_sleep"]
        .backpack
        .iter()
        .find(|i| i.vessel_meta.is_some() && i.bound_owner.is_none())
        .unwrap()
        .slug
        .clone();
    let npc_id = delta
        .actors
        .iter()
        .find(|(k, a)| k.starts_with("res:white_gull_harbor:") && a.copper >= 30)
        .map(|(k, _)| k.clone())
        .expect("wealthy harbor resident");
    delta.relocate_actor(&npc_id, &forge_nid, 4616, true);
    apply_peer_item_trade(
        &mut delta,
        &clock,
        "pl:p_sleep",
        "offer_to_peer",
        &format!("{npc_id}|{v2_slug}|12"),
    )
    .unwrap();
    equip_living_vessel(&mut delta, &npc_id, &v2_slug).unwrap();
    let chron_json = super::chronicle_export::build_chronicle_actor_json(&doc, &delta, &npc_id)
        .expect("chronicle json")
        .to_string();
    assert!(
        !chron_json.contains("新名號乙")
            && !chron_json.contains("初用名甲")
            && !chron_json.contains("p_sleep"),
        "C4: chronicle actor JSON must mask player current name, former alias, and controller_id: {chron_json}"
    );

    // H2 & M4: 不可抗力（發包人謝幕）退押金不冤判承接人 breached，且 prepare_commit_batch 嚴格核對託管總額
    propose_escrow_pact(&doc, &mut delta, &clock, "pl:p_looter", "shop_cover", "", 6).unwrap();
    let cid = delta
        .contracts
        .keys()
        .find(|k| k.starts_with("pact:shop_cover:"))
        .cloned()
        .unwrap();
    delta.relocate_actor(&npc_id, "oi:white_gull_harbor:dock", 4616, true);
    accept_escrow_pact(&doc, &mut delta, &clock, &npc_id, &cid).unwrap();
    assert!(super::persist_pg::prepare_commit_batch(&doc, &delta, false).is_ok());

    delta.actors.get_mut("pl:p_looter").unwrap().alive = false;
    settle_expired_escrow_pacts_as_world_events(&doc, &mut delta, &clock);
    assert_eq!(
        delta.contracts[&cid].status, "settled",
        "H2: creditor death force-majeure must NOT frame contractor as breached"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
    assert!(super::persist_pg::prepare_commit_batch(&doc, &delta, false).is_ok());

    // R2-L2: 前綴縮短改名（「初用名甲長」->「初用名」）降序替換零殘留尾字
    apply_setup_origin(
        &doc,
        &mut delta,
        &clock,
        "p_pre",
        "初用名甲長",
        "人類",
        "",
        "deck_sailor",
    )
    .unwrap();
    apply_setup_origin(
        &doc,
        &mut delta,
        &clock,
        "p_pre",
        "初用名",
        "人類",
        "",
        "deck_sailor",
    )
    .unwrap();
    let masked = super::slow_think::mask_player_names_in_text(&delta, "初用名甲長鍛・重刃");
    assert_eq!(masked, "外來旅人鍛・重刃");

    // R2-H2: 待售空屋 (`house_vacant`) 禁止以 120 銅繞過 360 銅購屋契，且廣場起屋錨定獨立 `alley` 絕不侵入 NPC `h1`
    assert!(delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "seed_home",
        "municipal_treasury",
        "pl:p_pre",
        150,
        "起屋測試".to_string()
    ));
    let vac_nid = doc
        .nodes
        .iter()
        .find(|(_, n)| n.kind == "house_vacant")
        .map(|(k, _)| k.clone())
        .unwrap();
    delta.relocate_actor("pl:p_pre", &vac_nid, 4616, true);
    {
        let p = delta.actors.get_mut("pl:p_pre").unwrap();
        for (sl, nm, cnt, wu) in [
            ("driftwood", "漂流木", 2u32, 2u16),
            ("iron_ore", "鐵礦", 1, 3),
        ] {
            add_carry_item(
                &mut p.backpack,
                CarryItem {
                    slug: sl.to_string(),
                    name: nm.to_string(),
                    count: cnt,
                    weight_units: wu,
                    ..CarryItem::default()
                },
            );
        }
    }
    assert!(build_new_homestead(&doc, &mut delta, &clock, "pl:p_pre").is_err());
    delta.relocate_actor("pl:p_pre", "oi:white_gull_harbor:plaza", 4616, true);
    build_new_homestead(&doc, &mut delta, &clock, "pl:p_pre").unwrap();
    let owned_lot = delta.actors["pl:p_pre"].owned_house_node.clone().unwrap();
    assert_eq!(doc.nodes[&owned_lot].kind, "alley");

    // R2-H1 & R2-L2: 已結案契約不殘留死按鈕，且 `CommitBatch` (extras_json + state_json) 反序列化往返無損
    delta.relocate_actor(&npc_id, "oi:white_gull_harbor:dock", 4616, true);
    let view_val = super::views::build_view_json(&doc, &delta, &delta.actors[&npc_id], &clock);
    assert!(!view_val["scene"]["actions"].to_string().contains(&cid));

    delta.first_spider_slain = Some(("pl:p_sleep".to_string(), "新名號乙".to_string(), 3));
    {
        let sp = delta.actors.get_mut("pl:p_sleep").unwrap();
        sp.mind.ltm.mastery.witnessed_boss_slayer = true;
        sp.mind.stm.pending_spar_from = Some(("pl:p_pre".to_string(), "初用名".to_string(), 9));
        sp.mark_mutated();
    }
    let batch = super::persist_pg::prepare_commit_batch(&doc, &delta, true).expect("full batch");
    let ext: super::persist_pg::WorldExtrasBlob = serde_json::from_str(&batch.extras_json).unwrap();
    assert_eq!(ext.first_spider_slain.as_ref().unwrap().0, "pl:p_sleep");
    assert_eq!(ext.chaos_rune_pool, delta.chaos_rune_pool);
    assert!(!ext.built_houses.is_empty());
    let sleep_row = batch
        .actor_rows
        .iter()
        .find(|r| r.actor_id == "pl:p_sleep")
        .unwrap();
    let restored: super::types::UnifiedMindBody =
        serde_json::from_str(&sleep_row.state_json).unwrap();
    assert!(
        restored
            .mind
            .ltm
            .mastery
            .past_aliases
            .iter()
            .any(|s| s == "初用名甲")
    );
    assert!(restored.mind.ltm.mastery.witnessed_boss_slayer);
    assert_eq!(restored.mind.stm.pending_spar_from.unwrap().0, "pl:p_pre");
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn r3_tick_loop_npc_pit_altar_pry_and_escort_watch_hold_at_goal() {
    let (doc, mut delta) = seeded_world();
    let c_warm = test_clock_for(4, 9);
    tick_outer_island_world(&doc, &mut delta, &c_warm, 10_000);
    let c0 = test_clock_for(5, 3);
    apply_setup_origin(
        &doc,
        &mut delta,
        &c0,
        "guard_p",
        "護行客",
        "人類",
        "",
        "forest_hunter",
    )
    .unwrap();

    // 1. 選一名敢闖試煉坑之白鷗港居民，置於斷橋並賦予 `pit_rumor` 與穩固鐵紋，透過真實 `tick_outer_island_world` 驗證斬蛛後必深入 `altar` 撬得 `rune_stone`！
    let delver_npc = delta
        .actors
        .iter()
        .find(|(k, a)| {
            k.starts_with("res:white_gull_harbor:")
                && super::persona::derive_innate_persona(a.soul_seed.max(1))
                    .temperament
                    .approach_avoid
                    >= 18
        })
        .map(|(k, _)| k.clone())
        .expect("bold harbor npc");
    let (bridge_nid, _) =
        super::movement::find_hidden_entrance_for_rumor(&doc, "pit_rumor").unwrap();
    delta.relocate_actor(&delver_npc, bridge_nid, 4529, true);
    {
        let d = delta.actors.get_mut(&delver_npc).unwrap();
        d.known_rumors.insert("pit_rumor".to_string());
        d.fatigue = 0;
        d.bound_weapon.rune_mark = Some("穩固鐵紋".to_string());
        d.route_plan.clear();
        d.mark_mutated();
    }

    // 2. 另選一名白鷗港居民於廣場發起 `escort_watch`（往西峽渡口廣場），由玩家接單，驗證跨響鐘抵達後持續駐足不逃回！
    let issuer_npc = delta
        .actors
        .keys()
        .find(|k| {
            k.starts_with("res:white_gull_harbor:")
                && **k != delver_npc
                && !delta.contracts.values().any(|c| c.creditor_id == **k)
        })
        .cloned()
        .unwrap();
    delta.relocate_actor(&issuer_npc, "oi:white_gull_harbor:plaza", 4616, true);
    delta.relocate_actor("pl:guard_p", "oi:white_gull_harbor:plaza", 4616, true);
    assert!(delta.transfer_copper(
        5,
        3,
        "seed_esc",
        "municipal_treasury",
        &issuer_npc,
        30,
        "護行測試".into()
    ));
    propose_escrow_pact(&doc, &mut delta, &c0, &issuer_npc, "escort_watch", "", 6).unwrap();
    let esc_cid = delta
        .contracts
        .iter()
        .find(|(_, c)| c.kind == "escort_watch" && c.creditor_id == issuer_npc)
        .map(|(k, _)| k.clone())
        .unwrap();
    accept_escrow_pact(&doc, &mut delta, &c0, "pl:guard_p", &esc_cid).unwrap();

    let mut ever_npc_pried_rune = false;
    for day in 5..=8 {
        for bell in 3..=9 {
            let clk = test_clock_for(day, bell);
            for sub in 0..3u64 {
                tick_outer_island_world(
                    &doc,
                    &mut delta,
                    &clk,
                    50_000 + (day as u64) * 999 + u64::from(bell) * 30 + sub,
                );
                if delta.actors.values().any(|a| {
                    a.controller_id.is_none()
                        && a.backpack
                            .iter()
                            .any(|i| i.slug == "rune_stone" && i.count > 0)
                }) {
                    ever_npc_pried_rune = true;
                }
            }
        }
    }
    assert!(
        ever_npc_pried_rune
            && delta.first_spider_slain.is_some()
            && delta
                .harvested_altars_until_day
                .contains_key("oi:dng_pit:altar")
            && !doc.nodes[&delta.actors[&delver_npc].node_id].dungeon,
        "R3-H1/R3-M1/R2-H3: autonomous NPC must slay spider, enter altar to pry rune_stone, and exit cleanly without empty-altar loop: pried={ever_npc_pried_rune}, slain={:?}, harvested={:?}, delver_node={}, route={:?}",
        delta.first_spider_slain,
        delta.harvested_altars_until_day,
        delta.actors[&delver_npc].node_id,
        delta.actors[&delver_npc].route_plan
    );
    assert_eq!(
        delta.actors[&issuer_npc].node_id, "oi:west_strait_ferry:plaza",
        "R3-H1/R2-M1: escort_watch issuer must travel to destination plaza and hold across bells"
    );
    delta.relocate_actor("pl:guard_p", "oi:west_strait_ferry:plaza", 4437, true);
    let end_clk = test_clock_for(7, 9);
    fulfill_escrow_pact(&doc, &mut delta, &end_clk, "pl:guard_p", &esc_cid).unwrap();
    assert_eq!(delta.contracts[&esc_cid].status, "settled");
    assert_eq!(delta.conservation_diff(&doc), 0);
}

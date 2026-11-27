//! 第一刀「人物真活、現場群戲插話、對話世界後果、離線託管方針與經歷回顧」回歸單元測試。

use crate::grid::outer_island::actions::apply_world_action;
use crate::grid::outer_island::clock_and_lang::solar_clock_for_settled_bell;
use crate::grid::outer_island::dialogue_consequence::apply_dialogue_world_consequence;
use crate::grid::outer_island::engine::load_static_world_doc;
use crate::grid::outer_island::living_presence::{
    build_live_scene_happenings, build_living_npc_cue,
};
use crate::grid::outer_island::movement::apply_setup_origin;
use crate::grid::outer_island::offline_autonomy::{
    set_player_offline_policy, tick_offline_player_day_autonomy,
};
use crate::grid::outer_island::persona::derive_innate_persona;
use crate::grid::outer_island::sim_tick::seed_genesis_actors_once;
use crate::grid::outer_island::types::{ContractRecord, IslandWorldDelta};

#[test]
fn living_presence_cues_happenings_and_vignette_intervene_mutate_real_world() {
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
        "p_live",
        "林遠行",
        "人類",
        "披舊斗篷",
        "smith_apprentice",
    )
    .expect("setup");

    // 把兩名真實居民移到市集廣場並立一筆真實逾期債契
    let plaza_id = "oi:white_gull_harbor:plaza";
    let npcs: Vec<String> = delta
        .actors
        .iter()
        .filter(|(_, a)| a.alive && a.controller_id.is_none())
        .take(2)
        .map(|(k, _)| k.clone())
        .collect();
    assert_eq!(npcs.len(), 2);
    let a1 = npcs[0].clone();
    let a2 = npcs[1].clone();
    delta.relocate_actor("pl:p_live", plaza_id, 4616, true);
    delta.relocate_actor(&a1, plaza_id, 4616, true);
    delta.relocate_actor(&a2, plaza_id, 4616, true);

    delta.contracts.insert(
        "test:debt:1".to_string(),
        ContractRecord {
            contract_id: "test:debt:1".to_string(),
            kind: "meal_tab".to_string(),
            creditor_id: a1.clone(),
            debtor_id: a2.clone(),
            amount_copper: 9,
            due_day: 118,
            status: "defaulted".to_string(),
            note_zh: "酒館賒帳".to_string(),
        },
    );

    let viewer = delta.actors.get("pl:p_live").expect("viewer");
    let npc1 = delta.actors.get(&a1).expect("npc1");
    let inn1 = derive_innate_persona(npc1.soul_seed.max(1));
    let cue1 = build_living_npc_cue(&doc, &delta, &day_clock, viewer, npc1, &inn1);
    assert!(
        cue1.contains(&delta.actors[&a2].display_name),
        "同場債主外顯舉止應真實鎖定欠債人，而非套用固定貼紙: {cue1}"
    );

    let plaza_node = doc.nodes.get(plaza_id).expect("plaza");
    let haps = build_live_scene_happenings(&doc, &delta, viewer, plaza_node, &day_clock);
    assert!(
        haps.iter().any(|h| h["kind"] == "npc_duo_tension"),
        "同場真實欠債應浮現可插話的現場對峙活戲: {haps:?}"
    );

    let tok1 = delta.actors[&a1].public_actor_token();
    let tok2 = delta.actors[&a2].public_actor_token();
    let msg = apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_live",
        "intervene_vignette",
        &format!("mediate|{tok1}|{tok2}"),
    )
    .expect("mediate ok");
    assert!(msg.contains("說和"), "調停應產生真實敘事回饋: {msg}");
    assert!(
        delta.actors[&a1]
            .mind
            .ltm
            .social
            .get("pl:p_live")
            .is_some_and(|s| s.acquainted && s.warmth > 0),
        "插話調停應真實寫入雙方結識與好感"
    );
}

#[test]
fn dialogue_world_consequence_transfers_route_knowledge_and_mutates_npc_plan() {
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
        "p_talk",
        "阿明",
        "人類",
        "麻布衣",
        "smith_apprentice",
    )
    .expect("setup");

    let plaza_id = "oi:white_gull_harbor:plaza";
    let npc_id = delta
        .actors
        .iter()
        .find(|(_, a)| a.alive && a.controller_id.is_none())
        .map(|(k, _)| k.clone())
        .expect("npc");
    delta.relocate_actor("pl:p_talk", plaza_id, 4616, true);
    delta.relocate_actor(&npc_id, plaza_id, 4616, true);

    // 讓 NPC 認識北城門，而玩家尚未踏足
    {
        let npc = delta.actors.get_mut(&npc_id).expect("npc mut");
        npc.mind
            .ltm
            .known_nodes
            .insert("oi:white_gull_harbor:north_gate".to_string());
        npc.mind
            .ltm
            .known_nodes
            .insert("oi:white_gull_harbor:smithy_st".to_string());
        npc.mind
            .ltm
            .known_nodes
            .insert("oi:white_gull_harbor:craft_south_st".to_string());
        npc.mind.ltm.known_nodes.insert(plaza_id.to_string());
        let s = npc
            .mind
            .ltm
            .social
            .entry("pl:p_talk".to_string())
            .or_default();
        s.acquainted = true;
        s.warmth = 24;
    }
    delta
        .actors
        .get_mut("pl:p_talk")
        .expect("player")
        .mind
        .ltm
        .known_nodes
        .remove("oi:white_gull_harbor:north_gate");

    let ask_res = apply_dialogue_world_consequence(
        &doc,
        &mut delta,
        &day_clock,
        "pl:p_talk",
        &npc_id,
        plaza_id,
        "請問北城門怎麼走？",
    );
    assert!(ask_res.is_some(), "問路應產生具體世界後果");
    assert!(
        delta.actors["pl:p_talk"]
            .mind
            .ltm
            .known_nodes
            .contains("oi:white_gull_harbor:north_gate"),
        "對方指路應真實將地標寫入玩家腦海地圖"
    );

    let invite_res = apply_dialogue_world_consequence(
        &doc,
        &mut delta,
        &day_clock,
        "pl:p_talk",
        &npc_id,
        plaza_id,
        "熟人一場，一起去北城門走一趟吧",
    );
    assert!(invite_res.is_some(), "熟人邀約應產生具體後果");
    assert!(
        !delta.actors[&npc_id].route_plan.is_empty(),
        "對方答應邀約後應真實改寫 route_plan 開始動身"
    );
}

#[test]
fn offline_autonomy_conserves_copper_and_records_non_numeric_diary() {
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
        "p_off",
        "遠遊客",
        "人類",
        "舊斗篷",
        "smith_apprentice",
    )
    .expect("setup");

    let total_before: i64 = delta.municipal_treasury_copper
        + delta.silver_bank_reserve_copper
        + delta.actors.values().map(|a| a.copper).sum::<i64>();

    set_player_offline_policy(&mut delta, &day_clock, "p_off", "work_livelihood").expect("policy");
    // 模擬離線超過活躍保護視窗
    delta.actors.get_mut("pl:p_off").expect("p").last_active_unix = day_clock.real_unix - 3600;
    delta.relocate_actor("pl:p_off", "oi:white_gull_harbor:dock", 4616, true);

    tick_offline_player_day_autonomy(&doc, &mut delta, &day_clock, "pl:p_off", 120 * 12 + 5);

    let total_after: i64 = delta.municipal_treasury_copper
        + delta.silver_bank_reserve_copper
        + delta.actors.values().map(|a| a.copper).sum::<i64>();
    assert_eq!(total_before, total_after, "離線幫工必須嚴格保持全島銅幣守恆");

    let p = &delta.actors["pl:p_off"];
    assert!(
        !p.offline_diary.is_empty() && p.offline_recap_unread,
        "離線自主行動後應寫入去數值化經歷日誌與未讀回顧旗標"
    );
}

//! 第三刀回歸測試：個人 4 武相＋4 藝相自然生成、《功法手札》著譜與同法不同相研讀、全自動田忌賽馬連續交鋒。

use crate::grid::outer_island::actions::apply_world_action;
use crate::grid::outer_island::clock_and_lang::solar_clock_for_settled_bell;
use crate::grid::outer_island::engine::load_static_world_doc;
use crate::grid::outer_island::martial_art_and_manuals::{
    build_eight_phases_view_json, derive_actor_eight_phases,
};
use crate::grid::outer_island::movement::apply_setup_origin;
use crate::grid::outer_island::sim_tick::seed_genesis_actors_once;
use crate::grid::outer_island::types::{IslandWorldDelta, SkirmishState};

#[test]
fn eight_phases_auto_generate_and_manuals_preserve_same_method_different_phase() {
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
        "p_alpha",
        "阿剛",
        "人類",
        "舊布衣",
        "smith_apprentice",
    )
    .expect("setup alpha");
    apply_setup_origin(
        &doc,
        &mut delta,
        &day_clock,
        "p_beta",
        "阿敏",
        "精靈",
        "輕獵裝",
        "forest_hunter",
    )
    .expect("setup beta");

    // 驗證兩名不同骨相角色各自自然生成 4 武相 ＋ 4 藝相
    let (m_a, c_a) = derive_actor_eight_phases(&delta.actors["pl:p_alpha"]);
    let (m_b, c_b) = derive_actor_eight_phases(&delta.actors["pl:p_beta"]);
    assert_eq!(m_a.len(), 4, "應自然生成 4 式臨陣武相");
    assert_eq!(c_a.len(), 4, "應自然生成 4 式百工藝相");
    assert_eq!(m_b.len(), 4);
    assert_eq!(c_b.len(), 4);

    let view_json = build_eight_phases_view_json(&delta.actors["pl:p_alpha"]);
    assert_eq!(view_json["martial"].as_array().map(Vec::len), Some(4));
    assert_eq!(view_json["craft"].as_array().map(Vec::len), Some(4));

    // 阿剛著譜寫出實體《功法手札》
    delta
        .actors
        .get_mut("pl:p_alpha")
        .expect("alpha")
        .bound_weapon
        .grip_familiarity = 14;
    let w_msg = apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_alpha",
        "write_martial_manual",
        "midgame",
    )
    .expect("write manual");
    assert!(w_msg.contains("手札"), "應生成實體手札: {w_msg}");

    let book_item = delta.actors["pl:p_alpha"]
        .backpack
        .iter()
        .find(|i| i.slug.starts_with("manual_"))
        .cloned()
        .expect("manual item in backpack");

    // 把實體手札轉交給阿敏研讀（不銷毀書冊且寫入 studied_manuals）
    delta
        .actors
        .get_mut("pl:p_beta")
        .expect("beta")
        .backpack
        .push(book_item.clone());
    let s_msg = apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_beta",
        "study_martial_manual",
        &book_item.slug,
    )
    .expect("study manual");
    assert!(s_msg.contains("同法") || s_msg.contains("骨相"), "研讀應呈現自身骨相消化: {s_msg}");
    assert!(
        delta.actors["pl:p_beta"]
            .mind
            .ltm
            .mastery
            .studied_manuals
            .contains(&book_item.name),
        "研讀後應記錄於 studied_manuals"
    );
    assert!(
        delta.actors["pl:p_beta"]
            .backpack
            .iter()
            .any(|i| i.slug == book_item.slug && i.count == 1),
        "實體功法手札研讀後不可無故消失"
    );
}

#[test]
fn auto_tianji_clash_runs_continuous_three_phase_prose_and_settles_skirmish() {
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
        "p_duel",
        "林試刃",
        "人類",
        "粗麻衫",
        "smith_apprentice",
    )
    .expect("setup duel");

    let wild = doc
        .nodes
        .values()
        .find(|n| !n.indoor && !n.dungeon && n.kind == "wild")
        .expect("wild");
    delta.relocate_actor("pl:p_duel", &wild.id, wild.cell_id, true);
    delta.actors.get_mut("pl:p_duel").expect("p").active_skirmish = Some(SkirmishState {
        foe_slug: "grey_ear_goblin".to_string(),
        foe_name: "灰耳斥候".to_string(),
        can_speak_goblin: false,
        foe_stagger: 0,
        player_guard: false,
        round_idx: 1,
    });

    let clash_msg = apply_world_action(
        &doc,
        &mut delta,
        &day_clock,
        "p_duel",
        "skirmish_auto_clash",
        "yield_then_strike",
    )
    .expect("auto clash");
    assert!(
        clash_msg.contains("後發制人") && !clash_msg.contains("第1回合"),
        "應輸出全自動連續田忌演繹文字且零回合數: {clash_msg}"
    );
    // 若進換氣空檔再補一次策略即收尾
    if delta.actors["pl:p_duel"].active_skirmish.is_some() {
        let fin = apply_world_action(
            &doc,
            &mut delta,
            &day_clock,
            "p_duel",
            "skirmish_auto_clash",
            "seam_harvest",
        )
        .expect("follow-up clash");
        assert!(!fin.is_empty());
    }
    assert!(
        delta.actors["pl:p_duel"].active_skirmish.is_none(),
        "尋常灰耳斥候應於全自動連續交鋒內完成結算，不得陷入回合勞作"
    );
}

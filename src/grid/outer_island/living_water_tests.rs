//! 「1 真實小時 ＝ 1 遊戲日（`scale = 24`）」連續活水時間流、48 階晨昏天光、錯峰作息、
//! 120 秒在線存活租約（防搶身體與防偷花抵宿券）、潮汐有限班次與局部方向目擊記憶單元測試。

use super::actions::apply_world_action;
use super::clock_and_lang::solar_clock_at;
use super::engine::load_static_world_doc;
use super::live_tide::{
    derive_tide_clock_details, is_actor_online_protected, relocate_actor_with_local_witness,
    revoke_client_lease, touch_client_lease, wake_player_on_explicit_input,
};
use super::movement::apply_setup_origin;
use super::sim_tick::{seed_genesis_actors_once, tick_outer_island_world};
use super::types::IslandWorldDelta;
use super::views::build_view_json;

fn fresh_seeded_world(is_night: bool) -> (super::types::WorldStaticDoc, IslandWorldDelta) {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, is_night);
    assert_eq!(delta.conservation_diff(&doc), 0);
    (doc, delta)
}

#[test]
fn one_real_hour_equals_one_game_day_and_48_tide_slots_progress_smoothly() {
    let (epoch, scale) = crate::game::game_time_cfg();
    assert!((scale - 24.0).abs() < f64::EPSILON, "scale must be 24.0");

    let c0 = solar_clock_at(epoch);
    let t0 = derive_tide_clock_details(&c0);
    assert_eq!((t0.hour, t0.minute, t0.tide_slot), (0, 0, 0));

    // 75 真實秒 = 30 遊戲分鐘 = 推進 1 格 48 階天光候
    let c_half_hr = solar_clock_at(epoch + 75);
    let t_half = derive_tide_clock_details(&c_half_hr);
    assert_eq!((t_half.hour, t_half.minute, t_half.tide_slot), (0, 30, 1));

    // 150 真實秒 = 1 遊戲小時
    let c_one_hr = solar_clock_at(epoch + 150);
    let t_one = derive_tide_clock_details(&c_one_hr);
    assert_eq!((t_one.hour, t_one.minute, t_one.tide_slot), (1, 0, 2));

    // 3600 真實秒（1 真實小時）= 完整 1 遊戲日（24 遊戲小時）
    let c_next_day = solar_clock_at(epoch + 3_600);
    assert_eq!(c_next_day.game_days, c0.game_days + 1);
    assert_eq!(c_next_day.hour, 0);

    // 驗證 2.5s（2500ms）步進搭配 `last_stagger_minute` 水位，即使啟動相位落在 `.100s` 亦 100% 走齊奇偶 120 槽！
    let (doc, mut delta) = fresh_seeded_world(true);
    let base_ms = (epoch as u64) * 1_000 + 450_100; // 03:00 + 100ms 相位
    for i in 0..120u64 {
        let now_ms = base_ms + i * 2_500;
        let clk = solar_clock_at((now_ms / 1_000) as i64);
        super::live_tide::step_staggered_minute_routines(&doc, &mut delta, &clk, now_ms);
    }
    let expected_end_min = (((((base_ms + 119 * 2_500) as f64 / 1_000.0) - epoch as f64) * 24.0)
        / 60.0)
        .floor() as i64;
    assert_eq!(delta.last_stagger_minute, expected_end_min);
}

#[test]
fn porters_have_indoor_homes_and_dock_work_is_gated_to_daytime_cargo_tides() {
    let (epoch, _) = crate::game::game_time_cfg();
    // 殘夜二鐘 03:00（epoch + 450 真實秒）
    let night_clk = solar_clock_at(epoch + 450);
    assert!(night_clk.is_night);
    assert_eq!(night_clk.bell_label, "殘夜二鐘");

    let (doc, mut delta) = fresh_seeded_world(true);

    // 1. 驗證 4 鎮碼頭苦力全數擁有真實室內住家（不再把露天碼頭當床），且未佔用任何待售空屋
    for pid in [
        "res:white_gull_harbor:dock_porter",
        "res:west_strait_ferry:dock_porter",
        "res:gull_tail_port:dock_porter",
        "res:north_cape_beacon:dock_porter",
    ] {
        let p = delta.actors.get(pid).expect("porter exists");
        let home_nd = doc
            .nodes
            .get(&p.home_node)
            .expect("porter home node exists");
        assert!(
            home_nd.indoor && home_nd.kind != "house_vacant",
            "porter {pid} must live in a real non-vacant indoor home, got {} ({})",
            p.home_node,
            home_nd.kind
        );
    }

    apply_setup_origin(
        &doc,
        &mut delta,
        &night_clk,
        "night_tester",
        "林風",
        "人類",
        "披著舊斗篷",
        "smith_apprentice",
    )
    .expect("setup at night");

    // 2. 殘夜二鐘登岸：敘事不得出現「鐵砧敲擊聲」「爐火正旺」或破折號「——」，且不得出現扛貨按鈕
    let pl = delta.actor_by_controller("night_tester").unwrap();
    let night_view = build_view_json(&doc, &delta, pl, &night_clk);
    let breaths_str = night_view["scene"]["breaths"].to_string();
    assert!(!breaths_str.contains("鐵砧"));
    assert!(!breaths_str.contains("爐火正旺"));
    assert!(!breaths_str.contains("——"));
    let acts = night_view["scene"]["actions"].as_array().unwrap();
    assert!(
        !acts.iter().any(|a| a["id"] == "work_dock"),
        "work_dock button must be hidden at 3 AM"
    );
    assert!(
        apply_world_action(
            &doc,
            &mut delta,
            &night_clk,
            "night_tester",
            "work_dock",
            ""
        )
        .is_err(),
        "work_dock action must be rejected at 3 AM"
    );

    // 3. 推進至早潮（晨鐘三響 08:00 = epoch + 1200 真實秒）：可搬卸當班船貨 1 次，同鐘響連點被拒並收起按鈕
    let morning_clk = solar_clock_at(epoch + 1_200);
    assert!(!morning_clk.is_night);
    let pl_day = delta.actor_by_controller("night_tester").unwrap();
    let day_view_before = build_view_json(&doc, &delta, pl_day, &morning_clk);
    assert!(
        day_view_before["scene"]["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"] == "work_dock")
    );

    apply_world_action(
        &doc,
        &mut delta,
        &morning_clk,
        "night_tester",
        "work_dock",
        "",
    )
    .expect("first shift in morning cargo tide succeeds");
    assert_eq!(delta.actor_by_controller("night_tester").unwrap().copper, 4);
    assert_eq!(delta.conservation_diff(&doc), 0);

    assert!(
        apply_world_action(
            &doc,
            &mut delta,
            &morning_clk,
            "night_tester",
            "work_dock",
            "",
        )
        .is_err(),
        "second spam click in same bell must be rejected"
    );
    let pl_after = delta.actor_by_controller("night_tester").unwrap();
    let day_view_after = build_view_json(&doc, &delta, pl_after, &morning_clk);
    assert!(
        !day_view_after["scene"]["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"] == "work_dock"),
        "work_dock button must disappear once this tide shift is completed"
    );
}

#[test]
fn online_lease_prevents_hijack_and_offline_shelter_preserves_voucher_and_copper() {
    let (epoch, _) = crate::game::game_time_cfg();
    let night_clk = solar_clock_at(epoch + 450);
    let (doc, mut delta) = fresh_seeded_world(false);

    apply_setup_origin(
        &doc,
        &mut delta,
        &night_clk,
        "lease_tester",
        "葉測",
        "人類",
        "青衫旅人",
        "fallen_peddler",
    )
    .expect("setup");
    let _ = delta.transfer_copper(
        night_clk.game_days,
        night_clk.bell_index,
        "test_seed",
        "municipal_treasury",
        "pl:lease_tester",
        11,
        "測試初始銅幣".to_string(),
    );

    // 1. 視窗存活租約有效期間（120s 內）：背景 tick 絕不可搶走玩家身體移動半步！
    let _ = touch_client_lease("lease_tester", night_clk.real_unix, true);
    assert!(is_actor_online_protected(
        delta.actor_by_controller("lease_tester").unwrap(),
        night_clk.real_unix + 90
    ));
    let mut clk_90s = solar_clock_at(night_clk.real_unix + 90);
    clk_90s.is_night = true;
    tick_outer_island_world(&doc, &mut delta, &clk_90s, 90_000);
    let p_online = delta.actor_by_controller("lease_tester").unwrap();
    assert_eq!(p_online.node_id, "oi:white_gull_harbor:dock");
    assert!(p_online.route_plan.is_empty());

    // 2. 玩家主動離線（撤銷租約墓碑）後：下一秒即刻解除在線保護（零 1800s 假在線鎖死），跨夜自動步入酒館一樓壁爐避寒且零耗費！
    revoke_client_lease("lease_tester");
    assert!(
        !is_actor_online_protected(
            delta.actor_by_controller("lease_tester").unwrap(),
            night_clk.real_unix + 1
        ),
        "revoke_client_lease must immediately clear online protection without falling back to 1800s"
    );
    let cur_unix = night_clk.real_unix + 150;
    for step in 0..4 {
        let u = cur_unix + step * 150;
        let c = solar_clock_at(u);
        tick_outer_island_world(&doc, &mut delta, &c, (u as u64) * 1_000);
    }
    let p_off = delta.actor_by_controller("lease_tester").unwrap();
    assert_eq!(
        p_off.node_id, "oi:white_gull_harbor:rusty_anchor_tavern",
        "offline player at dock must walk safely to ground-floor tavern hearth"
    );
    assert_eq!(p_off.copper, 11, "offline shelter must spend 0 copper");
    let voucher_cnt: u32 = p_off
        .backpack
        .iter()
        .filter(|i| i.slug == "lodging_voucher")
        .map(|i| i.count)
        .sum();
    assert_eq!(
        voucher_cnt, 1,
        "offline shelter must NEVER auto-redeem player's lodging_voucher"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 3. 若玩家擁有白鷗港房產卻在西峽渡口離線，絕不可被跨鎮拖穿荒野偷開迷霧
    if let Some(p_mut) = delta.actors.get_mut("pl:lease_tester") {
        p_mut.owned_house_node = Some("oi:white_gull_harbor:sail_ln:h5".to_string());
    }
    delta.relocate_actor("pl:lease_tester", "oi:west_strait_ferry:dock", 4221, true);
    let target_west = super::live_tide::resolve_offline_player_night_target(
        &doc,
        &delta,
        delta.actor_by_controller("lease_tester").unwrap(),
    );
    assert!(
        target_west
            .as_ref()
            .is_some_and(|(_, dst, _)| dst.starts_with("oi:west_strait_ferry:")),
        "offline player in West Strait Ferry must shelter locally, never cross wilderness to White Gull Harbor: {target_west:?}"
    );

    // 3. 玩家重返操作瞬間清空殘留自主路徑
    wake_player_on_explicit_input(&mut delta, "lease_tester", cur_unix + 600);
    assert!(
        delta
            .actor_by_controller("lease_tester")
            .unwrap()
            .route_plan
            .is_empty()
    );
}

#[test]
fn local_witness_records_exit_direction_without_xray_and_multi_bell_catchup_conserves_copper() {
    let (epoch, _) = crate::game::game_time_cfg();
    let day_clk = solar_clock_at(epoch + 1_200);
    let (doc, mut delta) = fresh_seeded_world(false);

    apply_setup_origin(
        &doc,
        &mut delta,
        &day_clk,
        "witness_viewer",
        "觀潮客",
        "人類",
        "旅人",
        "forest_hunter",
    )
    .expect("setup");

    // 將觀測者與碼頭苦力同置於市集廣場，苦力先往酒館街離開，再轉入鏽錨酒館（觀測者視線外）
    delta.relocate_actor(
        "pl:witness_viewer",
        "oi:white_gull_harbor:plaza",
        4616,
        true,
    );
    delta.relocate_actor(
        "res:white_gull_harbor:dock_porter",
        "oi:white_gull_harbor:plaza",
        4616,
        true,
    );

    relocate_actor_with_local_witness(
        &doc,
        &mut delta,
        &day_clk,
        "res:white_gull_harbor:dock_porter",
        "oi:white_gull_harbor:plaza",
        "oi:white_gull_harbor:tavern_st",
        4616,
        true,
    );
    relocate_actor_with_local_witness(
        &doc,
        &mut delta,
        &day_clk,
        "res:white_gull_harbor:dock_porter",
        "oi:white_gull_harbor:tavern_st",
        "oi:white_gull_harbor:rusty_anchor_tavern",
        4616,
        true,
    );

    let viewer = delta.actor_by_controller("witness_viewer").unwrap();
    let eps: Vec<String> = viewer
        .mind
        .stm
        .day_episodes
        .iter()
        .map(|e| e.summary_zh.clone())
        .collect();
    assert!(
        eps.iter().any(|s| s.contains("驛站街方向離開")),
        "co-located observer must record exit direction, got {eps:?}"
    );
    assert!(
        !eps.iter().any(|s| s.contains("鏽錨酒館")),
        "observer at plaza must NOT have omniscient X-ray of second hop into tavern, got {eps:?}"
    );

    // 模擬跨 5 個響鐘追趕（涵蓋第 5 響與第 8 響生計稅收結算），驗證複式簿記 0 差額
    let c_first = solar_clock_at(epoch + 1_200); // 08:00 (bell 4)
    tick_outer_island_world(&doc, &mut delta, &c_first, 10_000);
    let c_jump = solar_clock_at(epoch + 2_700); // 18:00 (bell 9, jumped across bells 5..=9)
    tick_outer_island_world(&doc, &mut delta, &c_jump, 20_000);
    assert_eq!(
        delta.last_settled_bell,
        c_jump.game_days * 12 + i64::from(c_jump.bell_index)
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

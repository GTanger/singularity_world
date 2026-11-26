//! 第一刀驗收測試：四季水文沖橋攔車＋公庫付薪購木修橋、林木墾田退化、空屋傾頹修復、動態疫氣解封與持久化往返。

use std::fs;
use std::path::Path;

use super::actions::apply_world_action;
use super::clock_and_lang::{SolarClockSnapshot, solar_clock_for_settled_bell};
use super::engine::load_static_world_doc;
use super::island_substrate::{
    BLDG_RUIN_WALL, BLDG_VACANT_ROOF, ROAD_BRIDGE_OR_PIER, TERRAIN_FARM,
};
use super::island_substrate_pack::{
    build_full_revealed_substrate_json, build_viewer_substrate_json,
};
use super::live_land::{
    ensure_live_land_seeded, house_is_ruined, is_bridge_washed_out, is_quarantine_restricted,
    tick_daily_live_land,
};

fn decode_rle_b64(b64: &str) -> Vec<u8> {
    let val_of = |c: u8| -> u32 {
        match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    };
    let bytes = b64.as_bytes();
    let mut raw = Vec::new();
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = val_of(bytes[i]);
        let b = val_of(bytes[i + 1]);
        let c = if bytes[i + 2] == b'=' {
            0
        } else {
            val_of(bytes[i + 2])
        };
        let d = if bytes[i + 3] == b'=' {
            0
        } else {
            val_of(bytes[i + 3])
        };
        let n = (a << 18) | (b << 12) | (c << 6) | d;
        raw.push(((n >> 16) & 255) as u8);
        if bytes[i + 2] != b'=' {
            raw.push(((n >> 8) & 255) as u8);
        }
        if bytes[i + 3] != b'=' {
            raw.push((n & 255) as u8);
        }
        i += 4;
    }
    let mut out = Vec::new();
    for pair in raw.chunks_exact(2) {
        let cnt = usize::from(pair[0]);
        out.resize(out.len() + cnt, pair[1]);
    }
    out
}
use super::movement::{add_carry_item, apply_move_step, apply_setup_origin};
use super::persist_pg::prepare_commit_batch;
use super::persist_schema::WorldExtrasBlob;
use super::sim_tick::seed_genesis_actors_once;
use super::transit_and_cargo::{
    ensure_transit_vehicles_initialized, tick_embodied_transit_vehicles,
};
use super::types::{CarryItem, IslandWorldDelta};

fn winter_clock_at_day(day: i64, bell: u8) -> SolarClockSnapshot {
    solar_clock_for_settled_bell(day * 12 + i64::from(bell), 1_760_000_000 + day * 3600)
}

#[test]
fn dynamic_miasma_replaces_hardcoded_list_and_wormwood_lifts_quarantine() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let transit_src =
        fs::read_to_string(Path::new(manifest).join("src/grid/outer_island/transit_and_cargo.rs"))
            .expect("read transit_and_cargo.rs");
    assert!(
        !transit_src.contains("SOUTH_PLAGUE_CIDS"),
        "hardcoded SOUTH_PLAGUE_CIDS must be replaced by dynamic live_land miasma!"
    );

    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    // 寒冬第 341 天（與線下寒冬 69 日相同無洪時相）
    let clk = winter_clock_at_day(341, 4);
    ensure_live_land_seeded(&doc, &mut delta, clk.game_days);

    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "miasma_tester",
        "燻艾行者",
        "人類",
        "神情冷靜",
        "smith_apprentice",
    )
    .expect("setup ok");
    let aid = delta.resolve_controller("miasma_tester").unwrap();
    delta.relocate_actor(&aid, "oi:cell4695:skirt_b", 4695, true);
    {
        let a = delta.actors.get_mut(&aid).unwrap();
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "rye_bread".to_string(),
                name: "黑麥乾糧".to_string(),
                count: 1,
                weight_units: 1,
                ..CarryItem::default()
            },
        );
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "quarantine_wormwood".to_string(),
                name: "南岸清冽苦艾束".to_string(),
                count: 2,
                weight_units: 1,
                ..CarryItem::default()
            },
        );
    }

    assert!(is_quarantine_restricted(&doc, &delta, 4776));
    assert!(
        apply_move_step(
            &doc,
            &mut delta,
            &clk,
            "miasma_tester",
            "oi:cell4776:skirt_a",
            5000
        )
        .is_err(),
        "carrying bread into miasma >= 30 quarantine cell 4776 must be blocked"
    );

    // 站入 #4776 焚燒 2 束苦艾把 miasma 從 55 降至 15（< 30）
    delta.relocate_actor(&aid, "oi:cell4776:skirt_a", 4776, true);
    apply_world_action(
        &doc,
        &mut delta,
        &clk,
        "miasma_tester",
        "burn_wormwood_miasma",
        "",
    )
    .expect("first wormwood burn");
    apply_world_action(
        &doc,
        &mut delta,
        &clk,
        "miasma_tester",
        "burn_wormwood_miasma",
        "",
    )
    .expect("second wormwood burn");
    assert!(
        !is_quarantine_restricted(&doc, &delta, 4776),
        "miasma < 30 must dynamically lift quarantine restriction"
    );
    delta.relocate_actor(&aid, "oi:cell4695:skirt_b", 4695, true);
    apply_move_step(
        &doc,
        &mut delta,
        &clk,
        "miasma_tester",
        "oi:cell4776:skirt_a",
        7000,
    )
    .expect("clearing miasma lifts quarantine food restriction");
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn spring_flood_washes_out_bridge_blocks_carriage_and_paid_timber_repair_restores() {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    ensure_transit_vehicles_initialized(&doc, &mut delta);
    let viewer_id = doc.residents[0].id.clone();

    // 初春第 33 日起水勢升漲；刻意將鎮庫降為 0 以驗證無錢不會憑空自動修橋
    let start_clk = winter_clock_at_day(33, 4);
    ensure_live_land_seeded(&doc, &mut delta, start_clk.game_days);
    let treasury_saved = delta.municipal_treasury_copper;
    delta.silver_bank_reserve_copper += treasury_saved;
    delta.municipal_treasury_copper = 0;

    for d in 34..=42 {
        let clk = winter_clock_at_day(d, 4);
        tick_daily_live_land(&doc, &mut delta, &clk);
    }

    assert!(
        is_bridge_washed_out(&delta, 4527) && is_bridge_washed_out(&delta, 4850),
        "#4527 and #4850 river crossings must wash out during spring flood crest without money to repair"
    );
    assert!(
        delta
            .live_land
            .cells
            .get(&4527)
            .is_some_and(|c| c.bridge_down_days >= 2),
        "bridge must remain down when treasury has 0 copper (no magic repair)"
    );

    // 驗證 /world_atlas 全開即時底盤立刻反映 #4850 斷橋與淹水擴張；而仍在白鷗港的旅人若未曾踏足 #4850 則保持迷霧（不隔空衛星偷看）
    let sub_ref = super::island_substrate::compiled_island_substrate(&doc);
    let atlas_broken =
        build_full_revealed_substrate_json(&doc, &delta, &winter_clock_at_day(42, 4));
    let atlas_broken_ov = decode_rle_b64(atlas_broken["overlay_rle_b64"].as_str().unwrap());
    let atlas_broken_tr = decode_rle_b64(atlas_broken["terrain_rle_b64"].as_str().unwrap());
    let atlas_broken_bridge_tiles = sub_ref
        .cell_owner
        .iter()
        .enumerate()
        .filter(|&(pos, &own)| {
            own != u16::MAX
                && sub_ref.cell_ids[own as usize] == 4850
                && sub_ref.base_terrain[pos] == super::island_substrate::TERRAIN_RIVER
                && (atlas_broken_ov[pos] & 0x07) == ROAD_BRIDGE_OR_PIER
        })
        .count();
    assert_eq!(
        atlas_broken_bridge_tiles, 0,
        "washed-out river bridge on #4850 must vanish on full world atlas"
    );
    let atlas_river_4850 = sub_ref
        .cell_owner
        .iter()
        .enumerate()
        .filter(|&(pos, &own)| {
            own != u16::MAX
                && sub_ref.cell_ids[own as usize] == 4850
                && atlas_broken_tr[pos] == super::island_substrate::TERRAIN_RIVER
        })
        .count();
    let base_river_tiles_4850 = sub_ref
        .cell_owner
        .iter()
        .enumerate()
        .filter(|&(pos, &own)| {
            own != u16::MAX
                && sub_ref.cell_ids[own as usize] == 4850
                && sub_ref.base_terrain[pos] == super::island_substrate::TERRAIN_RIVER
        })
        .count();
    assert!(
        atlas_river_4850 > base_river_tiles_4850,
        "flooded river tiles on #4850 must expand cleanly on full atlas ({atlas_river_4850} > {base_river_tiles_4850})"
    );

    // 旅人親臨 #4850 見證現場：個人輿圖立刻見證斷橋消失與淹水擴張！
    delta.relocate_actor(&viewer_id, "oi:cell4850:0", 4850, true);
    let broken_sub = build_viewer_substrate_json(
        &doc,
        &delta,
        delta.actors.get(&viewer_id).unwrap(),
        &winter_clock_at_day(42, 4),
    );
    let broken_ov = decode_rle_b64(broken_sub["overlay_rle_b64"].as_str().unwrap());
    let broken_tr = decode_rle_b64(broken_sub["terrain_rle_b64"].as_str().unwrap());
    let broken_bridge_tiles = sub_ref
        .cell_owner
        .iter()
        .enumerate()
        .filter(|&(pos, &own)| {
            own != u16::MAX
                && sub_ref.cell_ids[own as usize] == 4850
                && sub_ref.base_terrain[pos] == super::island_substrate::TERRAIN_RIVER
                && (broken_ov[pos] & 0x07) == ROAD_BRIDGE_OR_PIER
        })
        .count();
    assert_eq!(
        broken_bridge_tiles, 0,
        "washed-out river bridge on #4850 must vanish after viewer arrives on site"
    );
    let river_tiles_4850 = sub_ref
        .cell_owner
        .iter()
        .enumerate()
        .filter(|&(pos, &own)| {
            own != u16::MAX
                && sub_ref.cell_ids[own as usize] == 4850
                && broken_tr[pos] == super::island_substrate::TERRAIN_RIVER
        })
        .count();
    assert!(
        river_tiles_4850 > base_river_tiles_4850,
        "flooded river tiles on #4850 must expand cleanly when witnessed on site ({river_tiles_4850} > {base_river_tiles_4850})"
    );

    // 驗證東西幹線雙馬驛車行經 #4527 前（idx 4 -> 5）遭洪橋擋駕並原地折返，不穿斷橋！
    let ev_idx = delta
        .transit_vehicles
        .iter()
        .position(|v| v.line_id == "land_ew_express")
        .expect("ew express exists");
    delta.transit_vehicles[ev_idx].route_idx = 4; // #4432
    delta.transit_vehicles[ev_idx].forward = true; // next stop 5 = oi:cell4527:0
    tick_embodied_transit_vehicles(&doc, &mut delta, &winter_clock_at_day(42, 4));
    assert_eq!(
        delta.transit_vehicles[ev_idx].route_idx, 4,
        "carriage must NOT enter washed-out #4527"
    );
    assert!(
        !delta.transit_vehicles[ev_idx].forward,
        "carriage must turn around when facing washed-out bridge"
    );

    // 歸還鎮庫銅幣，由一名開拓者砍木並親手修復 #4527 與 #4850 木橋
    delta.silver_bank_reserve_copper -= treasury_saved;
    delta.municipal_treasury_copper = treasury_saved;
    let clk42 = winter_clock_at_day(42, 4);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk42,
        "bridge_fixer",
        "修橋工",
        "矮人",
        "手提大斧",
        "smith_apprentice",
    )
    .expect("setup ok");
    let fixer_id = delta.resolve_controller("bridge_fixer").unwrap();
    delta.relocate_actor(&fixer_id, "oi:cell4527:0", 4527, true);
    apply_world_action(
        &doc,
        &mut delta,
        &clk42,
        "bridge_fixer",
        "fell_timber_clear",
        "",
    )
    .expect("fell timber ok");
    let repair_msg = apply_world_action(
        &doc,
        &mut delta,
        &clk42,
        "bridge_fixer",
        "repair_river_bridge",
        "4527",
    )
    .expect("repair bridge 4527 ok");
    assert!(repair_msg.contains("恢復"));
    assert!(!is_bridge_washed_out(&delta, 4527));

    delta.actors.get_mut(&fixer_id).unwrap().fatigue = 0;
    delta.relocate_actor(&fixer_id, "oi:cell4850:0", 4850, true);
    apply_world_action(
        &doc,
        &mut delta,
        &clk42,
        "bridge_fixer",
        "fell_timber_clear",
        "",
    )
    .expect("fell timber for 4850 ok");
    apply_world_action(
        &doc,
        &mut delta,
        &clk42,
        "bridge_fixer",
        "repair_river_bridge",
        "4850",
    )
    .expect("repair bridge 4850 ok");
    assert!(!is_bridge_washed_out(&delta, 4850));
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 修橋後地圖復現 #4850 橋樑，且篷車順利通過 #4527！
    let fixed_sub =
        build_viewer_substrate_json(&doc, &delta, delta.actors.get(&viewer_id).unwrap(), &clk42);
    let fixed_ov = decode_rle_b64(fixed_sub["overlay_rle_b64"].as_str().unwrap());
    let fixed_bridge_tiles = sub_ref
        .cell_owner
        .iter()
        .enumerate()
        .filter(|&(pos, &own)| {
            own != u16::MAX
                && sub_ref.cell_ids[own as usize] == 4850
                && sub_ref.base_terrain[pos] == super::island_substrate::TERRAIN_RIVER
                && (fixed_ov[pos] & 0x07) == ROAD_BRIDGE_OR_PIER
        })
        .count();
    assert!(
        fixed_bridge_tiles > 0,
        "repaired river bridge on #4850 must reappear on live map overlay"
    );

    delta.transit_vehicles[ev_idx].route_idx = 4;
    delta.transit_vehicles[ev_idx].forward = true;
    tick_embodied_transit_vehicles(&doc, &mut delta, &clk42);
    assert_eq!(
        delta.transit_vehicles[ev_idx].route_idx, 5,
        "carriage must resume crossing #4527 once bridge is repaired"
    );
}

#[test]
fn farmland_reclaim_fallow_regression_vacant_ruin_and_commit_roundtrip() {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    let clk0 = winter_clock_at_day(280, 4);
    ensure_live_land_seeded(&doc, &mut delta, clk0.game_days);

    apply_setup_origin(
        &doc,
        &mut delta,
        &clk0,
        "settler_qa",
        "開荒客",
        "人類",
        "腰插鐮鋤",
        "smith_apprentice",
    )
    .expect("setup ok");
    let aid = delta.resolve_controller("settler_qa").unwrap();
    let target_cell = 4432u32;
    delta.relocate_actor(&aid, "oi:cell4432:0", target_cell, true);
    {
        let a = delta.actors.get_mut(&aid).unwrap();
        a.mind.ltm.known_cells.insert(target_cell);
        a.mind.ltm.known_cells.insert(4616);
    }

    let sub_ref = super::island_substrate::compiled_island_substrate(&doc);
    let count_farms = |pack: &serde_json::Value| -> usize {
        let tr = decode_rle_b64(pack["terrain_rle_b64"].as_str().unwrap());
        sub_ref
            .cell_owner
            .iter()
            .enumerate()
            .filter(|&(pos, &own)| {
                own != u16::MAX
                    && sub_ref.cell_ids[own as usize] == target_cell
                    && tr[pos] == TERRAIN_FARM
            })
            .count()
    };

    let base_farms = count_farms(&build_viewer_substrate_json(
        &doc,
        &delta,
        delta.actors.get(&aid).unwrap(),
        &clk0,
    ));

    for idx in 0..4 {
        let c_act = winter_clock_at_day(280, 4 + idx);
        delta.actors.get_mut(&aid).unwrap().fatigue = 0;
        apply_world_action(
            &doc,
            &mut delta,
            &c_act,
            "settler_qa",
            "reclaim_farmland",
            "",
        )
        .expect("reclaim plot ok");
    }
    let after_reclaim_farms = count_farms(&build_viewer_substrate_json(
        &doc,
        &delta,
        delta.actors.get(&aid).unwrap(),
        &clk0,
    ));
    assert!(
        after_reclaim_farms > base_farms,
        "reclaimed_plots must render visible TERRAIN_FARM blocks ({after_reclaim_farms} > {base_farms})"
    );

    // 連續 26 天不耕作：田壟逐步退化，且白鷗港繩索巷 9 號空屋逐漸破敗傾頹成斷牆
    let rope_h9 = "oi:white_gull_harbor:rope_ln:h9";
    for d in 281..=355 {
        let cd = winter_clock_at_day(d, 4);
        tick_daily_live_land(&doc, &mut delta, &cd);
    }
    let plots_after_fallow = delta.live_land.cells[&target_cell].reclaimed_plots;
    assert!(
        plots_after_fallow < 4,
        "fallow farmland must regress after >= 18 untended days (got {plots_after_fallow})"
    );
    assert!(
        house_is_ruined(&delta, rope_h9),
        "long-uninhabited vacant house must decay into ruined wall state"
    );

    let clk_end = winter_clock_at_day(355, 4);
    // 開荒客仍在 #4432 未親臨 #4616 繩索巷：個人輿圖尚不知情，而 /world_atlas 全開底盤已可見斷牆！
    let atlas_ruin = build_full_revealed_substrate_json(&doc, &delta, &clk_end);
    let atlas_ruin_ov = decode_rle_b64(atlas_ruin["overlay_rle_b64"].as_str().unwrap());
    let h9_cells = &sub_ref.house_footprint_cells[rope_h9];
    assert!(
        h9_cells
            .iter()
            .all(|&pos| (atlas_ruin_ov[pos] >> 3) == BLDG_RUIN_WALL),
        "ruined vacant house footprint must render BLDG_RUIN_WALL on full atlas"
    );

    // 親赴繩索巷現場見證：個人輿圖隨即呈現 BLDG_RUIN_WALL！
    delta.relocate_actor(&aid, "oi:white_gull_harbor:rope_ln", 4616, true);
    let ruin_pack =
        build_viewer_substrate_json(&doc, &delta, delta.actors.get(&aid).unwrap(), &clk_end);
    let ruin_ov = decode_rle_b64(ruin_pack["overlay_rle_b64"].as_str().unwrap());
    assert!(
        h9_cells
            .iter()
            .all(|&pos| (ruin_ov[pos] >> 3) == BLDG_RUIN_WALL),
        "ruined vacant house footprint must render BLDG_RUIN_WALL when witnessed on site"
    );

    // 砍伐原木並修繕空屋：立即恢復 BLDG_VACANT_ROOF！
    delta.actors.get_mut(&aid).unwrap().fatigue = 0;
    delta.relocate_actor(&aid, "oi:cell4432:0", target_cell, true);
    apply_world_action(
        &doc,
        &mut delta,
        &clk_end,
        "settler_qa",
        "fell_timber_clear",
        "",
    )
    .expect("fell timber for repair");
    delta.relocate_actor(&aid, rope_h9, 4616, true);
    apply_world_action(
        &doc,
        &mut delta,
        &clk_end,
        "settler_qa",
        "repair_vacant_house",
        rope_h9,
    )
    .expect("repair vacant house");
    assert!(!house_is_ruined(&delta, rope_h9));

    let fixed_pack =
        build_viewer_substrate_json(&doc, &delta, delta.actors.get(&aid).unwrap(), &clk_end);
    let fixed_ov = decode_rle_b64(fixed_pack["overlay_rle_b64"].as_str().unwrap());
    assert!(
        h9_cells
            .iter()
            .all(|&pos| (fixed_ov[pos] >> 3) == BLDG_VACANT_ROOF),
        "repaired vacant house must return to BLDG_VACANT_ROOF"
    );

    // 驗證 WorldExtrasBlob CommitBatch 序列化與還原無損
    let batch = prepare_commit_batch(&doc, &delta, false).expect("commit batch ok");
    let decoded_extras: WorldExtrasBlob =
        serde_json::from_str(&batch.extras_json).expect("decode extras_json ok");
    assert!(decoded_extras.live_land.seeded);
    assert_eq!(
        decoded_extras.live_land.cells[&target_cell].reclaimed_plots,
        plots_after_fallow
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

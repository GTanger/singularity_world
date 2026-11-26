use super::clock_and_lang::solar_clock_for_settled_bell;
use super::engine::load_static_world_doc;
use super::island_substrate::{
    ROAD_BRIDGE_OR_PIER, TERRAIN_FOG, TERRAIN_RIVER, compiled_island_substrate,
};
use super::island_substrate_pack::{
    build_full_revealed_substrate_json, build_viewer_substrate_json,
};
use super::live_land::{ensure_live_land_seeded, is_bridge_washed_out, tick_daily_live_land};
use super::live_wilds::{GoblinBandState, ensure_live_wilds_seeded};
use super::map_memory::{apply_memory_map_travel, ingest_hearsay_rumor_pin};
use super::movement::apply_setup_origin;
use super::sim_tick::seed_genesis_actors_once;
use super::transit_and_cargo::ensure_transit_vehicles_initialized;
use super::types::{IslandWorldDelta, WorldStaticDoc};

fn decode_rle_b64(b64: &str) -> Vec<u8> {
    const LUT: [i16; 256] = {
        let mut t = [-1i16; 256];
        let tbl = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            t[tbl[i] as usize] = i as i16;
            i += 1;
        }
        t
    };
    let mut raw = Vec::new();
    let bytes = b64.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = LUT[bytes[i] as usize] as u32;
        let b = LUT[bytes[i + 1] as usize] as u32;
        raw.push(((a << 2) | (b >> 4)) as u8);
        if bytes[i + 2] != b'=' {
            let c = LUT[bytes[i + 2] as usize] as u32;
            raw.push((((b & 15) << 4) | (c >> 2)) as u8);
            if bytes[i + 3] != b'=' {
                let d = LUT[bytes[i + 3] as usize] as u32;
                raw.push((((c & 3) << 6) | d) as u8);
            }
        }
        i += 4;
    }
    let mut out = Vec::new();
    for pair in raw.chunks_exact(2) {
        for _ in 0..pair[0] {
            out.push(pair[1]);
        }
    }
    out
}

fn setup_test_world(
    day: i64,
    bell: u8,
) -> (
    WorldStaticDoc,
    IslandWorldDelta,
    super::clock_and_lang::SolarClockSnapshot,
) {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    ensure_transit_vehicles_initialized(&doc, &mut delta);
    let epoch = 1_760_000_000i64;
    let clk = solar_clock_for_settled_bell(day * 12 + i64::from(bell), epoch + day * 3600);
    ensure_live_land_seeded(&doc, &mut delta, clk.game_days);
    ensure_live_wilds_seeded(&doc, &mut delta, clk.game_days);
    (doc, delta, clk)
}

#[test]
fn skirting_cell_edge_keeps_village_hidden_until_entering_core() {
    let (doc, mut delta, clk) = setup_test_world(280, 4);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "skirt_qa",
        "擦邊旅人",
        "人類",
        "",
        "smith_apprentice",
    )
    .expect("setup ok");
    let pid = delta.resolve_actor_id("skirt_qa").unwrap();
    super::live_tide::relocate_actor_with_local_witness(
        &doc,
        &mut delta,
        &clk,
        &pid,
        "oi:white_gull_harbor:dock",
        "oi:cell4432:skirt_a",
        4432,
        false,
    );

    let sub_ref = compiled_island_substrate(&doc);
    let viewer = delta.actors.get(&pid).unwrap();
    let pack = build_viewer_substrate_json(&doc, &delta, viewer, &clk);
    let tr = decode_rle_b64(pack["terrain_rle_b64"].as_str().unwrap());

    let idx4432 = sub_ref.cell_index_by_id[&4432];
    let mut total_4432 = 0usize;
    let mut fog_4432 = 0usize;
    for (pos, &own) in sub_ref.cell_owner.iter().enumerate() {
        if own == idx4432 {
            total_4432 += 1;
            if tr[pos] == TERRAIN_FOG {
                fog_4432 += 1;
            }
        }
    }
    assert!(
        fog_4432 * 100 >= total_4432 * 70,
        "skirting cell4432:skirt_a must leave >= 70% of #4432 under fog (got {fog_4432}/{total_4432})"
    );
    let pois = pack["pois"].as_array().unwrap();
    assert!(
        !pois.iter().any(|p| p["id"] == "town:burg_4432"),
        "skirting edge node must not leak unvisited village POI"
    );
    let b_hits = pack["building_hits"].as_array().unwrap();
    assert!(
        !b_hits
            .iter()
            .any(|b| b["nid"].as_str().is_some_and(|s| s.contains("burg_4432"))),
        "skirting edge node must not leak village roof hits"
    );

    super::live_tide::relocate_actor_with_local_witness(
        &doc,
        &mut delta,
        &clk,
        &pid,
        "oi:cell4432:skirt_a",
        "oi:cell4432:0",
        4432,
        true,
    );
    let pack_core =
        build_viewer_substrate_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &clk);
    assert!(
        pack_core["pois"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "town:burg_4432"),
        "entering core gate oi:cell4432:0 must reveal village POI"
    );
}

#[test]
fn distant_disaster_and_band_stay_isolated_from_witnessed_memory_until_visited() {
    let (doc, mut delta, mut clk) = setup_test_world(34, 4);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "mem_qa",
        "記憶旅人",
        "人類",
        "",
        "smith_apprentice",
    )
    .expect("setup ok");
    let pid = delta.resolve_actor_id("mem_qa").unwrap();

    // 先於冬末（橋仍完好）親臨 #4850 見證木橋完好，再返回白鷗港 #4616
    super::live_tide::relocate_actor_with_local_witness(
        &doc,
        &mut delta,
        &clk,
        &pid,
        "oi:white_gull_harbor:dock",
        "oi:cell4850:0",
        4850,
        true,
    );
    super::live_tide::relocate_actor_with_local_witness(
        &doc,
        &mut delta,
        &clk,
        &pid,
        "oi:cell4850:0",
        "oi:white_gull_harbor:plaza",
        4616,
        true,
    );

    // 清空鎮庫並連續推進春汛至第 42 日：#4850 斷橋且 #4430 高壓哥布林營寨崛起
    let saved_tr = delta.municipal_treasury_copper;
    delta.silver_bank_reserve_copper += saved_tr;
    delta.municipal_treasury_copper = 0;
    for d in 35..=42 {
        clk = solar_clock_for_settled_bell(d * 12 + 4, 1_760_000_000i64 + d * 3600);
        tick_daily_live_land(&doc, &mut delta, &clk);
    }
    assert!(is_bridge_washed_out(&delta, 4850));
    delta.live_wilds.bands.insert(
        4430,
        GoblinBandState {
            cell_id: 4430,
            band_name_zh: "灰耳掠奪營".to_string(),
            camp_node_id: "oi:cell4430:camp".to_string(),
            headcount: 8,
            food_stores: 1,
            hoard_copper: 24,
            raid_pressure: 85,
            suppressed_until_day: 0,
            last_raid_day: 0,
            hoard_items: Vec::new(),
        },
    );

    // 人在白鷗港時，個人記憶輿圖中 #4850 依然顯示上次親見的完好木橋，且不出現 #4430 營寨 POI
    let sub_ref = compiled_island_substrate(&doc);
    let viewer_pack =
        build_viewer_substrate_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &clk);
    let viewer_ov = decode_rle_b64(viewer_pack["overlay_rle_b64"].as_str().unwrap());
    let bridges_in_mem = sub_ref
        .cell_owner
        .iter()
        .enumerate()
        .filter(|&(pos, &own)| {
            own != u16::MAX
                && sub_ref.cell_ids[own as usize] == 4850
                && sub_ref.base_terrain[pos] == TERRAIN_RIVER
                && (viewer_ov[pos] & 0x07) == ROAD_BRIDGE_OR_PIER
        })
        .count();
    assert!(
        bridges_in_mem > 0,
        "viewer sitting at plaza must still remember intact bridge on #4850"
    );
    assert!(
        !viewer_pack["pois"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "band:4430"),
        "distant unvisited goblin camp must never stream to player radar"
    );

    // 設計者 /world_atlas 全開底盤則必須即時反映 #4850 斷橋與 #4430 營寨
    let atlas_pack = build_full_revealed_substrate_json(&doc, &delta, &clk);
    let atlas_ov = decode_rle_b64(atlas_pack["overlay_rle_b64"].as_str().unwrap());
    let bridges_on_atlas = sub_ref
        .cell_owner
        .iter()
        .enumerate()
        .filter(|&(pos, &own)| {
            own != u16::MAX
                && sub_ref.cell_ids[own as usize] == 4850
                && sub_ref.base_terrain[pos] == TERRAIN_RIVER
                && (atlas_ov[pos] & 0x07) == ROAD_BRIDGE_OR_PIER
        })
        .count();
    assert_eq!(bridges_on_atlas, 0);
    assert!(
        atlas_pack["pois"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["label"] == "灰耳掠奪營")
    );

    // 聽聞 #4850 斷橋風聲 -> 僅增加 ？傳聞 問號釘，底圖木橋依然未改！
    ingest_hearsay_rumor_pin(
        &doc,
        &mut delta,
        &pid,
        "hearsay_4850",
        "聽行商提起 #4850 木橋被春汛沖斷了",
        clk.game_days,
    );
    let with_rumor =
        build_viewer_substrate_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &clk);
    assert!(
        with_rumor["rumor_pins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["label"].as_str().is_some_and(|s| s.starts_with("？傳聞"))),
        "hearsay must render ? rumor pin"
    );
    let rumor_ov = decode_rle_b64(with_rumor["overlay_rle_b64"].as_str().unwrap());
    let bridges_during_rumor = sub_ref
        .cell_owner
        .iter()
        .enumerate()
        .filter(|&(pos, &own)| {
            own != u16::MAX
                && sub_ref.cell_ids[own as usize] == 4850
                && sub_ref.base_terrain[pos] == TERRAIN_RIVER
                && (rumor_ov[pos] & 0x07) == ROAD_BRIDGE_OR_PIER
        })
        .count();
    assert!(
        bridges_during_rumor > 0,
        "rumor must NOT mutate witnessed bridge terrain"
    );

    // 親赴 #4850 現場：傳聞釘消解，個人底圖即時見證斷橋！
    super::live_tide::relocate_actor_with_local_witness(
        &doc,
        &mut delta,
        &clk,
        &pid,
        "oi:white_gull_harbor:plaza",
        "oi:cell4850:0",
        4850,
        true,
    );
    let onsite_pack =
        build_viewer_substrate_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &clk);
    assert!(
        onsite_pack["rumor_pins"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["id"] != "rumor:cell:4850"),
        "arriving on site must resolve rumor pin"
    );
    let onsite_ov = decode_rle_b64(onsite_pack["overlay_rle_b64"].as_str().unwrap());
    let bridges_onsite = sub_ref
        .cell_owner
        .iter()
        .enumerate()
        .filter(|&(pos, &own)| {
            own != u16::MAX
                && sub_ref.cell_ids[own as usize] == 4850
                && sub_ref.base_terrain[pos] == TERRAIN_RIVER
                && (onsite_ov[pos] & 0x07) == ROAD_BRIDGE_OR_PIER
        })
        .count();
    assert_eq!(bridges_onsite, 0);
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn memory_map_travel_follows_known_nodes_and_stops_on_surprise_or_tide() {
    let (doc, mut delta, clk) = setup_test_world(280, 15);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "nav_qa",
        "踏圖旅人",
        "人類",
        "",
        "smith_apprentice",
    )
    .expect("setup ok");
    let pid = delta.resolve_actor_id("nav_qa").unwrap();

    // 未探索過北城門等未知節點時，禁止直接盲點遠程傳送
    assert!(
        apply_memory_map_travel(&doc, &mut delta, &clk, "nav_qa", "oi:cell4850:0", 1_000).is_err(),
        "unknown target node must be rejected"
    );

    // 先步行踏足：dock -> plaza -> smithy_st -> north_gate（全數寫入 known_nodes）
    for (idx, step) in [
        "oi:white_gull_harbor:plaza",
        "oi:white_gull_harbor:smithy_st",
        "oi:white_gull_harbor:north_gate",
    ]
    .iter()
    .enumerate()
    {
        apply_memory_map_travel(
            &doc,
            &mut delta,
            &clk,
            "nav_qa",
            step,
            2_000 + (idx as u64) * 500,
        )
        .expect("step along exits");
    }

    // 從 north_gate 點地圖直返已探索之 dock：應沿 known_nodes 逐跳安全返抵
    let ret = apply_memory_map_travel(
        &doc,
        &mut delta,
        &clk,
        "nav_qa",
        "oi:white_gull_harbor:dock",
        5_000,
    )
    .expect("memory travel back to dock");
    assert!(ret.contains("抵達") && delta.actors[&pid].node_id == "oi:white_gull_harbor:dock");

    // 若支巷屋舍已在 known_nodes，點擊也能直抵且白鷗港支巷不會與門牌混為 adjacent_exits
    delta
        .actors
        .get_mut(&pid)
        .unwrap()
        .mind
        .ltm
        .known_nodes
        .insert("oi:white_gull_harbor:tavern_st".to_string());
    delta
        .actors
        .get_mut(&pid)
        .unwrap()
        .mind
        .ltm
        .known_nodes
        .insert("oi:white_gull_harbor:rope_ln".to_string());
    delta
        .actors
        .get_mut(&pid)
        .unwrap()
        .mind
        .ltm
        .known_nodes
        .insert("oi:white_gull_harbor:rope_ln:h1".to_string());
    apply_memory_map_travel(
        &doc,
        &mut delta,
        &clk,
        "nav_qa",
        "oi:white_gull_harbor:rope_ln",
        6_000,
    )
    .expect("walk to alley");
    let alley_sub = build_viewer_substrate_json(&doc, &delta, &delta.actors[&pid], &clk);
    let adj = alley_sub["adjacent_exits"].as_array().unwrap();
    assert!(
        !adj.iter()
            .any(|e| e["id"].as_str().is_some_and(|s| s.contains(":h"))),
        "alley adjacent_exits must omit per-house overlapping pills in favor of roof building_hits"
    );
    assert!(
        alley_sub["building_hits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["nid"] == "oi:white_gull_harbor:rope_ln:h1" && b["door_badge"] == "1號"),
        "alley roof hits must provide 1號 door badge directly on roof"
    );

    // 驗證記憶與實況落差時當場驚見停步（模擬記憶中 #4850 完好，抵達 #4850 卻見木橋已沖斷）
    let p_to_4850 = super::movement::shortest_node_path(
        &doc,
        "oi:white_gull_harbor:south_gate",
        "oi:cell4850:0",
    )
    .unwrap();
    for n in &p_to_4850 {
        delta
            .actors
            .get_mut(&pid)
            .unwrap()
            .mind
            .ltm
            .known_nodes
            .insert(n.clone());
        if let Some(nd) = doc.nodes.get(n) {
            delta
                .actors
                .get_mut(&pid)
                .unwrap()
                .mind
                .ltm
                .known_cells
                .insert(nd.cell_id);
        }
    }
    super::live_tide::relocate_actor_with_local_witness(
        &doc,
        &mut delta,
        &clk,
        &pid,
        "oi:white_gull_harbor:rope_ln",
        "oi:white_gull_harbor:south_gate",
        4616,
        true,
    );
    delta
        .actors
        .get_mut(&pid)
        .unwrap()
        .mind
        .ltm
        .map_memory
        .witnessed_cells
        .insert(
            4850,
            super::map_memory::WitnessedCellSnap {
                bridge_down: false,
                core_visited: true,
                ..Default::default()
            },
        );
    if let Some(cl) = delta.live_land.cells.get_mut(&4850) {
        cl.has_river_crossing = true;
        cl.bridge_integrity = 0;
        cl.flood_level = 2;
    }
    let near_4850 = p_to_4850[p_to_4850.len().saturating_sub(3)].clone();
    super::live_tide::relocate_actor_with_local_witness(
        &doc,
        &mut delta,
        &clk,
        &pid,
        "oi:white_gull_harbor:south_gate",
        &near_4850,
        doc.nodes[&near_4850].cell_id,
        true,
    );
    let surprise_msg =
        apply_memory_map_travel(&doc, &mut delta, &clk, "nav_qa", "oi:cell4850:0", 9_000)
            .expect("arriving at changed bridge must return warning message");
    assert!(
        surprise_msg.contains("沖斷") || surprise_msg.contains("停步"),
        "expected bridge washout discovery stop cue, got: {surprise_msg}"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

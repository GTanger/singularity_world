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
    assert_eq!(
        tr.iter().filter(|&&t| t == TERRAIN_FOG).count(),
        0,
        "skirting or explored map must contain ZERO TERRAIN_FOG across all 799,800 tiles"
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
    if let Some(&[ax, ay]) = sub_ref.node_anchors.get("oi:cell4432:0")
        && let Some(a) = delta.actors.get_mut(&pid)
    {
        a.map_gx = ax;
        a.map_gy = ay;
    }
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
    if let Some(&[ax, ay]) = sub_ref.node_anchors.get("oi:cell4850:0")
        && let Some(a) = delta.actors.get_mut(&pid)
    {
        a.map_gx = ax;
        a.map_gy = ay;
    }
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

    // 先步行踏足：dock -> market_avenue -> plaza -> craft_south_st -> smithy_st -> north_gate（全數寫入 known_nodes）
    for (idx, step) in [
        "oi:white_gull_harbor:market_avenue",
        "oi:white_gull_harbor:plaza",
        "oi:white_gull_harbor:craft_south_st",
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
        6_000,
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
        7_000,
    )
    .expect("walk to alley");
    let alley_sub = build_viewer_substrate_json(&doc, &delta, &delta.actors[&pid], &clk);
    let caps = alley_sub["proximity_capsules"].as_array().unwrap();
    assert!(
        caps.iter()
            .all(|c| c["dist"].as_f64().unwrap() <= 9.5 + 0.05),
        "all proximity_capsules must stay within 9.5 tiles"
    );
    assert!(
        alley_sub["building_hits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["nid"] == "oi:white_gull_harbor:rope_ln:h1" && b["door_badge"] == "1號"),
        "alley roof hits must provide 1號 door badge directly on roof"
    );

    // 室內角色不得出現於大地圖 nearby_actors，且戶外 nearby_actors 嚴守 <= 14.0 格
    let any_indoor_npc = delta
        .actors
        .iter()
        .find(|(id, a)| {
            *id != &pid
                && a.alive
                && a.setup_done
                && doc.nodes.get(&a.node_id).is_some_and(|n| n.indoor)
        })
        .map(|(_, a)| a.public_actor_token());
    if let Some(tok) = any_indoor_npc {
        assert!(
            !alley_sub["nearby_actors"]
                .as_array()
                .unwrap()
                .iter()
                .any(|na| na["id"] == tok),
            "indoor NPC must disappear from overland map nearby_actors"
        );
    }

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
    if let Some(&[ax, ay]) = compiled_island_substrate(&doc).node_anchors.get(&near_4850)
        && let Some(a) = delta.actors.get_mut(&pid)
    {
        a.map_gx = ax;
        a.map_gy = ay;
    }
    let surprise_msg = super::map_walk::handle_player_move_request(
        &doc,
        &mut delta,
        &clk,
        "nav_qa",
        "oi:cell4850:0",
        None,
        None,
        "",
        None,
        9_000,
    )
    .expect("arriving at changed bridge must return warning message");
    assert!(
        surprise_msg.contains("沖斷") || surprise_msg.contains("停步"),
        "expected bridge washout discovery stop cue, got: {surprise_msg}"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn local_tile_diffs_write_at_actor_xy_persist_and_isolate_distant_viewer() {
    use super::island_substrate::{
        BLDG_NONE, GRID_H, GRID_W, ROAD_NONE, TERRAIN_FARM, TERRAIN_LAND, TERRAIN_TREE,
    };
    use super::live_land_tiles::{carve_felled_tree_tile_at_actor, reclaim_arable_tiles_at_actor};
    use super::persist_pg::prepare_commit_batch;
    use super::persist_schema::WorldExtrasBlob;

    let (doc, mut delta, clk) = setup_test_world(280, 4);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "viewer_far",
        "遠望旅人",
        "人類",
        "",
        "smith_apprentice",
    )
    .expect("setup ok");
    let pid = delta.resolve_actor_id("viewer_far").unwrap();
    let sub = compiled_island_substrate(&doc);

    let dock = sub.node_anchors["oi:white_gull_harbor:dock"];
    let mut tree_xy: Option<(isize, isize, usize)> = None;
    let mut farm_xy: Option<(isize, isize)> = None;
    for (pos, &t) in sub.base_terrain.iter().enumerate() {
        if sub.cell_owner[pos] == u16::MAX
            || sub.sea_dist_by_cell_edge[pos] < 16
            || sub.base_road[pos] != ROAD_NONE
            || sub.base_bldg[pos] != BLDG_NONE
        {
            continue;
        }
        let gx = (pos % GRID_W) as isize;
        let gy = (pos / GRID_W) as isize;
        let d_dock = (gx as f32 - dock[0]).hypot(gy as f32 - dock[1]);
        if d_dock < 40.0 {
            continue;
        }
        if tree_xy.is_none() && t == TERRAIN_TREE {
            tree_xy = Some((gx, gy, pos));
        } else if farm_xy.is_none()
            && t == TERRAIN_LAND
            && tree_xy.is_some_and(|(tx, ty, _)| (gx - tx).abs() + (gy - ty).abs() > 40)
        {
            farm_xy = Some((gx, gy));
        }
        if tree_xy.is_some() && farm_xy.is_some() {
            break;
        }
    }
    let (tx, ty, _tpos) = tree_xy.expect("must find tree far from dock");
    let (fx, fy) = farm_xy.expect("must find land far from tree & dock");

    let felled = carve_felled_tree_tile_at_actor(sub, &mut delta, tx, ty, clk.game_days);
    assert!(
        felled > 0,
        "must carve real forest tiles near actor ({tx},{ty})"
    );
    let felled_pos_list: Vec<usize> = delta
        .tile_diffs
        .iter()
        .filter(|(_, e)| e.terrain == TERRAIN_LAND)
        .map(|(&p, _)| p as usize)
        .collect();
    assert_eq!(felled_pos_list.len(), felled);
    for &p in &felled_pos_list {
        let px = (p % GRID_W) as isize;
        let py = (p / GRID_W) as isize;
        assert!((px - tx).abs() <= 18 && (py - ty).abs() <= 18);
        assert_eq!(sub.base_terrain[p], TERRAIN_TREE);
    }

    let reclaimed = reclaim_arable_tiles_at_actor(sub, &mut delta, fx, fy, clk.game_days);
    assert!(
        reclaimed > 0,
        "must reclaim real arable tiles near actor ({fx},{fy})"
    );
    let farm_pos_list: Vec<usize> = delta
        .tile_diffs
        .iter()
        .filter(|(_, e)| e.terrain == TERRAIN_FARM)
        .map(|(&p, _)| p as usize)
        .collect();
    assert_eq!(farm_pos_list.len(), reclaimed);
    for &p in &farm_pos_list {
        let px = (p % GRID_W) as isize;
        let py = (p / GRID_W) as isize;
        assert!((px - fx).abs() <= 18 && (py - fy).abs() <= 18);
    }

    // 驗證 prepare_commit_batch 序列化與還原完整保存逐格 tile_diffs
    let batch = prepare_commit_batch(&doc, &delta, true).expect("commit batch ok");
    let roundtrip: WorldExtrasBlob =
        serde_json::from_str(&batch.extras_json).expect("deserialize extras");
    assert_eq!(roundtrip.tile_diffs.len(), delta.tile_diffs.len());
    for (&k, v) in &delta.tile_diffs {
        let rv = roundtrip
            .tile_diffs
            .get(&k)
            .expect("missing persisted tile_diff");
        assert_eq!(
            (rv.terrain, rv.road, rv.bldg, rv.changed_day),
            (v.terrain, v.road, v.bldg, v.changed_day)
        );
    }

    // 遠在港務碼頭尚未親臨現場時，私人底圖維持原貌（不透過衛星洩漏未見之砍樹與開墾）
    let far_pack = build_viewer_substrate_json(&doc, &delta, &delta.actors[&pid], &clk);
    let far_tr = decode_rle_b64(far_pack["terrain_rle_b64"].as_str().unwrap());
    for &p in &felled_pos_list {
        assert_eq!(
            far_tr[p], TERRAIN_TREE,
            "unwitnessed tree clearing must NOT leak to distant viewer"
        );
    }
    for &p in &farm_pos_list {
        assert_ne!(
            far_tr[p], TERRAIN_FARM,
            "unwitnessed reclaimed farmland must NOT leak to distant viewer"
        );
    }

    // 走近林場視距 (<= 18 格) 親見後，立即寫入 witnessed_tile_diffs 並反映於旅人底圖
    super::map_memory::record_local_witness_at_tile(
        &doc,
        &mut delta,
        &pid,
        tx as f32 + 0.5,
        ty as f32 + 0.5,
        clk.game_days,
    );
    if let Some(a) = delta.actors.get_mut(&pid) {
        a.map_gx = tx as f32 + 0.5;
        a.map_gy = ty as f32 + 0.5;
        a.map_pos_seeded = true;
    }
    let near_pack = build_viewer_substrate_json(&doc, &delta, &delta.actors[&pid], &clk);
    let near_tr = decode_rle_b64(near_pack["terrain_rle_b64"].as_str().unwrap());
    for &p in &felled_pos_list {
        assert_eq!(
            near_tr[p], TERRAIN_LAND,
            "local witness within 18 tiles must render newly felled clearing"
        );
    }

    // 驗證實地無樹石板廣場禁止憑空伐木、以及僅擦過遠橋邊界（>18 格）絕不劇透斷橋
    delta.relocate_actor(&pid, "oi:white_gull_harbor:plaza", 4616, true);
    assert!(
        super::actions::apply_world_action(
            &doc,
            &mut delta,
            &clk,
            "viewer_far",
            "fell_timber_clear",
            ""
        )
        .is_err(),
        "fell_timber_clear on town cobble plaza with zero adjacent trees must fail without minting timber"
    );

    if let Some(cl) = delta.live_land.cells.get_mut(&4850) {
        cl.has_river_crossing = true;
        cl.bridge_integrity = 0;
    }
    let bidx_4850 = sub.cell_index_by_id[&4850];
    if let Some(edge_pos) = sub.cell_owner.iter().enumerate().find_map(|(pos, &own)| {
        if own != bidx_4850 {
            return None;
        }
        let ex = (pos % GRID_W) as isize;
        let ey = (pos / GRID_W) as isize;
        let near_bridge = (0..GRID_W * GRID_H).any(|p2| {
            sub.cell_owner[p2] == bidx_4850
                && sub.base_terrain[p2] == TERRAIN_RIVER
                && (sub.base_road[p2] & 0x07) == ROAD_BRIDGE_OR_PIER
                && ((p2 % GRID_W) as isize - ex).pow(2) + ((p2 / GRID_W) as isize - ey).pow(2)
                    <= 361
        });
        (!near_bridge).then_some((ex, ey))
    }) {
        delta
            .actors
            .get_mut(&pid)
            .unwrap()
            .mind
            .ltm
            .map_memory
            .witnessed_cells
            .remove(&4850);
        super::map_memory::record_local_witness_at_tile(
            &doc,
            &mut delta,
            &pid,
            edge_pos.0 as f32 + 0.5,
            edge_pos.1 as f32 + 0.5,
            clk.game_days,
        );
        assert!(
            !delta.actors[&pid]
                .mind
                .ltm
                .map_memory
                .witnessed_cells
                .get(&4850)
                .is_some_and(|s| s.bridge_down),
            "grazing cell border >18 tiles from bridge must NOT spoil bridge_down"
        );
    }
}

#[test]
fn walking_along_street_keeps_terrain_digest_stable_until_real_tile_change() {
    use super::island_substrate::{GRID_W, TERRAIN_LAND};
    use super::map_memory_pack::{build_walk_map_sync_json, compute_viewer_terrain_digest};
    use super::types::TileDiffEntry;

    let (doc, mut delta, clk) = setup_test_world(280, 4);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "walk_digest_qa",
        "穩步旅人",
        "人類",
        "",
        "smith_apprentice",
    )
    .expect("setup ok");
    let pid = delta.resolve_actor_id("walk_digest_qa").unwrap();
    let dock = compiled_island_substrate(&doc).node_anchors["oi:white_gull_harbor:dock"];
    let plaza = compiled_island_substrate(&doc).node_anchors["oi:white_gull_harbor:plaza"];
    let mut prev_digest = compute_viewer_terrain_digest(&doc, &delta, &delta.actors[&pid], &clk);

    super::map_walk::handle_player_move_request(
        &doc,
        &mut delta,
        &clk,
        "walk_digest_qa",
        "",
        Some(plaza[0]),
        Some(plaza[1]),
        "plan",
        None,
        1_000,
    )
    .expect("plan walk toward plaza");

    let mut step_dirty_count = 0usize;
    let mut steps_taken = 0usize;
    for i in 0..8 {
        super::map_walk::handle_player_move_request(
            &doc,
            &mut delta,
            &clk,
            "walk_digest_qa",
            "",
            None,
            None,
            "step",
            Some(3),
            1_500 + (i + 1) * 500,
        )
        .expect("step walk ok");
        let sync =
            build_walk_map_sync_json(&doc, &delta, &delta.actors[&pid], &clk, Some(prev_digest));
        if sync["terrain_dirty"].as_bool().unwrap_or(false) {
            step_dirty_count += 1;
        }
        prev_digest = sync["terrain_digest"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap();
        steps_taken += 1;
        if delta.actors[&pid].walk_waypoints.is_empty() {
            break;
        }
    }
    assert!(steps_taken >= 3, "must complete >= 3 multi-step walk ticks");
    assert_eq!(
        step_dirty_count, 0,
        "walking past intact harbor houses must not false-trigger terrain_dirty (dock={dock:?})"
    );

    let ax = delta.actors[&pid].map_gx.floor() as usize;
    let ay = delta.actors[&pid].map_gy.floor() as usize;
    let nearby_pos = (ay * GRID_W + ax.saturating_add(2)) as u32;
    delta.tile_diffs.insert(
        nearby_pos,
        TileDiffEntry {
            terrain: TERRAIN_LAND,
            road: 1,
            bldg: 0,
            changed_day: clk.game_days,
        },
    );
    let after_change =
        build_walk_map_sync_json(&doc, &delta, &delta.actors[&pid], &clk, Some(prev_digest));
    assert_eq!(
        after_change["terrain_dirty"].as_bool(),
        Some(true),
        "real tile_diffs change within 18 tiles must flip terrain_dirty = true"
    );
}

#[test]
fn walking_past_sold_and_ruined_houses_never_flips_terrain_dirty_on_exit() {
    use super::map_memory_pack::{build_walk_map_sync_json, compute_viewer_terrain_digest};

    let (doc, mut delta, clk) = setup_test_world(280, 4);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "house_digest_qa",
        "察屋旅人",
        "人類",
        "",
        "smith_apprentice",
    )
    .expect("setup ok");
    let pid = delta.resolve_actor_id("house_digest_qa").unwrap();
    let sub = compiled_island_substrate(&doc);
    let mid_xy = sub.node_anchors["oi:white_gull_harbor:south_gate"];
    let approach_xy = sub.node_anchors["oi:cell4617:approach_a"];
    let far_xy = sub.node_anchors["oi:cell4617:0"];
    // Pick a point just south of south_gate where nearest outdoor node is non-core approach_a,
    // ensuring record_local_witness_at_tile handles the 18-tile house witness without entering harbor core.
    let skirt_peek_xy = [mid_xy[0], mid_xy[1] + 4.0];

    let mut candidates: Vec<String> = sub
        .house_footprint_cells
        .keys()
        .filter(|hnid| {
            sub.node_anchors.get(*hnid).is_some_and(|&[ax, ay]| {
                let d_peek = (ax - skirt_peek_xy[0]).hypot(ay - skirt_peek_xy[1]);
                let d_far = (ax - far_xy[0]).hypot(ay - far_xy[1]);
                d_peek <= 16.5 && d_far > 19.5
            })
        })
        .cloned()
        .collect();
    candidates.sort();
    assert!(
        candidates.len() >= 2,
        "expected >= 2 houses within 16.5 tiles of south_gate skirt outside cell4617:0 sight (approach={approach_xy:?}), got {}",
        candidates.len()
    );
    let sold_hnid = candidates[0].clone();
    let ruined_hnid = candidates[1].clone();
    delta.sold_houses.remove(&ruined_hnid);
    delta.live_land.house_integrity.insert(ruined_hnid.clone(), 10);
    delta
        .sold_houses
        .insert(sold_hnid.clone(), "npc:house_digest_buyer".to_string());

    let sold_xy = sub.node_anchors[&sold_hnid];
    let ruined_xy = sub.node_anchors[&ruined_hnid];

    // Walk first to far_xy (outside 19.5 tiles), clear any initial memory of the 2 houses, then walk into skirt_peek_xy and back out.
    super::map_walk::handle_player_move_request(
        &doc,
        &mut delta,
        &clk,
        "house_digest_qa",
        "",
        Some(far_xy[0]),
        Some(far_xy[1]),
        "plan",
        None,
        500,
    )
    .expect("plan to far_xy ok");
    for i in 0..40 {
        if delta.actors[&pid].walk_waypoints.is_empty() {
            break;
        }
        super::map_walk::handle_player_move_request(
            &doc,
            &mut delta,
            &clk,
            "house_digest_qa",
            "",
            None,
            None,
            "step",
            Some(3),
            800 + i * 500,
        )
        .expect("step to far_xy ok");
    }
    if let Some(a) = delta.actors.get_mut(&pid) {
        a.mind.ltm.map_memory.witnessed_sold_houses.remove(&sold_hnid);
        a.mind
            .ltm
            .map_memory
            .witnessed_ruined_houses
            .remove(&ruined_hnid);
    }

    // Step 1: Walk from cell4617:0 into skirt_peek_xy (entering <= 18 sight of both houses via non-core approach path)
    super::map_walk::handle_player_move_request(
        &doc,
        &mut delta,
        &clk,
        "house_digest_qa",
        "",
        Some(skirt_peek_xy[0]),
        Some(skirt_peek_xy[1]),
        "plan",
        None,
        25_000,
    )
    .expect("plan to skirt_peek_xy ok");
    for i in 0..40 {
        if delta.actors[&pid].walk_waypoints.is_empty() {
            break;
        }
        super::map_walk::handle_player_move_request(
            &doc,
            &mut delta,
            &clk,
            "house_digest_qa",
            "",
            None,
            None,
            "step",
            Some(3),
            25_500 + i * 500,
        )
        .expect("step to skirt_peek_xy ok");
    }
    let cur_xy = [delta.actors[&pid].map_gx, delta.actors[&pid].map_gy];
    assert!(
        (cur_xy[0] - sold_xy[0]).hypot(cur_xy[1] - sold_xy[1]) <= 18.0
            && (cur_xy[0] - ruined_xy[0]).hypot(cur_xy[1] - ruined_xy[1]) <= 18.0,
        "player must have entered 18-tile sight of both test houses at skirt_peek_xy (cur={cur_xy:?})"
    );
    assert!(
        delta.actors[&pid]
            .mind
            .ltm
            .map_memory
            .witnessed_sold_houses
            .contains(&sold_hnid),
        "witnessed_sold_houses must sync on tile walk"
    );
    assert!(
        delta.actors[&pid]
            .mind
            .ltm
            .map_memory
            .witnessed_ruined_houses
            .contains(&ruined_hnid),
        "witnessed_ruined_houses must sync on tile walk"
    );

    // Step 2: Walk outward beyond 18 tiles to cell4617:0 and verify no false terrain_dirty flip upon leaving sight
    let mut prev_digest = compute_viewer_terrain_digest(&doc, &delta, &delta.actors[&pid], &clk);
    super::map_walk::handle_player_move_request(
        &doc,
        &mut delta,
        &clk,
        "house_digest_qa",
        "",
        Some(far_xy[0]),
        Some(far_xy[1]),
        "plan",
        None,
        50_000,
    )
    .expect("plan back to cell4617:0 ok");

    let mut left_sight = false;
    let mut dirty_after_leaving = 0usize;
    for i in 0..40 {
        if delta.actors[&pid].walk_waypoints.is_empty() {
            break;
        }
        super::map_walk::handle_player_move_request(
            &doc,
            &mut delta,
            &clk,
            "house_digest_qa",
            "",
            None,
            None,
            "step",
            Some(3),
            50_500 + i * 500,
        )
        .expect("step outward ok");
        let sync =
            build_walk_map_sync_json(&doc, &delta, &delta.actors[&pid], &clk, Some(prev_digest));
        let px = delta.actors[&pid].map_gx;
        let py = delta.actors[&pid].map_gy;
        let dist_sold = (px - sold_xy[0]).hypot(py - sold_xy[1]);
        let dist_ruined = (px - ruined_xy[0]).hypot(py - ruined_xy[1]);
        if dist_sold > 18.5 && dist_ruined > 18.5 {
            if sync["terrain_dirty"].as_bool().unwrap_or(false) {
                dirty_after_leaving += 1;
            }
            left_sight = true;
        }
        prev_digest = sync["terrain_digest"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap();
    }
    assert!(left_sight, "must walk out beyond 18.5 tiles of test houses");
    assert_eq!(
        dirty_after_leaving, 0,
        "leaving sight of witnessed sold/ruined houses must not trigger false terrain_dirty"
    );
}

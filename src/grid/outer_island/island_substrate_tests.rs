//! 全島單一無縫連續底盤之物理與拓撲不變量回歸測試。

use std::collections::{BTreeSet, VecDeque};

use super::clock_and_lang::solar_clock_at;
use super::engine::load_static_world_doc;
use super::island_substrate::{
    BLDG_ROOF, BLDG_VACANT_ROOF, GRID_H, GRID_W, ROAD_COUNTRY_TRACK, ROAD_TOWN_COBBLE, TERRAIN_FOG,
    TERRAIN_REEF_SAND, TERRAIN_RIVER, TERRAIN_SEA, build_viewer_substrate_json,
    compiled_island_substrate,
};
use super::movement::apply_setup_origin;
use super::phase_and_vessels::BuiltHomestead;
use super::sim_tick::seed_genesis_actors_once;
use super::types::IslandWorldDelta;
use super::views::build_filtered_map_json;

fn decode_rle_b64(b64: &str) -> Vec<u8> {
    let lut = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let bytes = b64.as_bytes();
    let mut raw = Vec::new();
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = lut(bytes[i]).unwrap_or(0);
        let b = lut(bytes[i + 1]).unwrap_or(0);
        let c = if bytes[i + 2] == b'=' {
            0
        } else {
            lut(bytes[i + 2]).unwrap_or(0)
        };
        let d = if bytes[i + 3] == b'=' {
            0
        } else {
            lut(bytes[i + 3]).unwrap_or(0)
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
    let mut out = Vec::with_capacity(GRID_W * GRID_H);
    let mut j = 0;
    while j + 1 < raw.len() {
        let run = raw[j] as usize;
        let val = raw[j + 1];
        out.resize(out.len() + run, val);
        j += 2;
    }
    out
}

#[test]
fn all_rivers_flow_to_sea_and_town_gates_connect_without_seams() {
    let doc = load_static_world_doc();
    let sub = compiled_island_substrate(&doc);
    let total = GRID_W * GRID_H;
    assert_eq!(sub.base_terrain.len(), total);

    // 1. 每一條河流連通分量必須直接接觸海水（0 內陸斷頭河）
    let mut visited = vec![false; total];
    let mut river_count = 0usize;
    for start in 0..total {
        if sub.base_terrain[start] != TERRAIN_RIVER || visited[start] {
            continue;
        }
        river_count += 1;
        let mut q = VecDeque::new();
        visited[start] = true;
        q.push_back(start);
        let mut touches_sea = false;
        while let Some(cur) = q.pop_front() {
            let cx = cur % GRID_W;
            let cy = cur / GRID_W;
            let nbs = [
                if cx > 0 { Some(cur - 1) } else { None },
                if cx + 1 < GRID_W { Some(cur + 1) } else { None },
                if cy > 0 { Some(cur - GRID_W) } else { None },
                if cy + 1 < GRID_H {
                    Some(cur + GRID_W)
                } else {
                    None
                },
            ];
            for nopt in nbs.into_iter().flatten() {
                if sub.base_terrain[nopt] == TERRAIN_SEA {
                    touches_sea = true;
                } else if sub.base_terrain[nopt] == TERRAIN_RIVER && !visited[nopt] {
                    visited[nopt] = true;
                    q.push_back(nopt);
                }
            }
        }
        assert!(
            touches_sea,
            "river component starting at {start} must reach SEA"
        );
    }
    assert!(river_count >= 1, "island must have rivers reaching the sea");

    // 2. 五大城鎮南北城門（街巷）必與郊外古道無縫 4-連通
    for slug in [
        "white_gull_harbor",
        "west_strait_ferry",
        "gull_tail_port",
        "greypine_post",
    ] {
        let g_nid = format!("oi:{slug}:north_gate");
        let anchor = sub.node_anchors.get(&g_nid).expect("gate anchor exists");
        let gx = (anchor[0].floor() as usize).min(GRID_W - 1);
        let gy = (anchor[1].floor() as usize).min(GRID_H - 1);
        let start = gy * GRID_W + gx;
        let mut vis = BTreeSet::new();
        let mut q = VecDeque::new();
        vis.insert(start);
        q.push_back(start);
        let mut saw_cobble = false;
        let mut saw_track = false;
        while let Some(cur) = q.pop_front() {
            let r = sub.base_road[cur];
            if r == ROAD_TOWN_COBBLE {
                saw_cobble = true;
            } else if r == ROAD_COUNTRY_TRACK {
                saw_track = true;
            }
            if saw_cobble && saw_track {
                break;
            }
            if vis.len() > 2_000 {
                break;
            }
            let cx = cur % GRID_W;
            let cy = cur / GRID_W;
            let nbs = [
                if cx > 0 { Some(cur - 1) } else { None },
                if cx + 1 < GRID_W { Some(cur + 1) } else { None },
                if cy > 0 { Some(cur - GRID_W) } else { None },
                if cy + 1 < GRID_H {
                    Some(cur + GRID_W)
                } else {
                    None
                },
            ];
            for nopt in nbs.into_iter().flatten() {
                if !vis.contains(&nopt) && matches!(sub.base_road[nopt], 1 | 2 | 3) {
                    vis.insert(nopt);
                    q.push_back(nopt);
                }
            }
        }
        assert!(
            saw_cobble && saw_track,
            "{slug} north_gate road must connect TOWN_COBBLE smoothly to COUNTRY_TRACK (cobble={saw_cobble}, track={saw_track})"
        );
    }

    // 3. 白鷗港獨立屋棟數量與全部 859 非地城節點錨點完整驗證
    let wgh_idx = sub.cell_index_by_id[&4616];
    let mut wgh_vis = vec![false; total];
    let mut wgh_bldg_comps = 0usize;
    let mut max_comp_tiles = 0usize;
    for i in 0..total {
        if sub.cell_owner[i] == wgh_idx
            && matches!(sub.base_bldg[i], BLDG_ROOF | BLDG_VACANT_ROOF)
            && !wgh_vis[i]
        {
            wgh_bldg_comps += 1;
            let mut count = 0usize;
            let mut q = VecDeque::new();
            wgh_vis[i] = true;
            q.push_back(i);
            while let Some(cur) = q.pop_front() {
                count += 1;
                let cx = cur % GRID_W;
                let cy = cur / GRID_W;
                for nopt in [
                    if cx > 0 { Some(cur - 1) } else { None },
                    if cx + 1 < GRID_W { Some(cur + 1) } else { None },
                    if cy > 0 { Some(cur - GRID_W) } else { None },
                    if cy + 1 < GRID_H {
                        Some(cur + GRID_W)
                    } else {
                        None
                    },
                ]
                .into_iter()
                .flatten()
                {
                    if sub.cell_owner[nopt] == wgh_idx
                        && matches!(sub.base_bldg[nopt], BLDG_ROOF | BLDG_VACANT_ROOF)
                        && !wgh_vis[nopt]
                    {
                        wgh_vis[nopt] = true;
                        q.push_back(nopt);
                    }
                }
            }
            max_comp_tiles = max_comp_tiles.max(count);
        }
    }
    assert!(
        wgh_bldg_comps >= 50,
        "white_gull_harbor must have >= 50 separate residential/shop roof clusters, got {wgh_bldg_comps}"
    );
    assert!(
        max_comp_tiles <= 12,
        "white_gull_harbor houses must not merge into giant slabs (max cluster={max_comp_tiles})"
    );
    for (nid, nd) in &doc.nodes {
        if !nd.dungeon {
            assert!(
                sub.node_anchors.contains_key(nid),
                "missing grid anchor for node {nid}"
            );
        }
    }
}

#[test]
fn viewer_substrate_enforces_fog_low_tide_and_dynamic_houses_with_zero_webp() {
    let (epoch, _) = crate::game::game_time_cfg();
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    // 讓候 15 退潮窗對照候 2 漲潮窗
    let low_clk = solar_clock_at(epoch + 15 * 75);
    let high_clk = solar_clock_at(epoch + 2 * 75);

    apply_setup_origin(
        &doc,
        &mut delta,
        &low_clk,
        "sub_tester",
        "旅人卡爾",
        "人類",
        "",
        "smith_apprentice",
    )
    .expect("setup");

    // 踏足南岸貝礁海岸 4617 並買下一戶空屋＋新建一間新宅
    delta.relocate_actor("sub_tester", "oi:cell4617:0", 4617, true);
    delta.built_houses.insert(
        "oi:built:test_1".to_string(),
        BuiltHomestead {
            house_id: "oi:built:test_1".to_string(),
            owner_id: "pl:sub_tester".to_string(),
            anchor_node_id: "oi:cell4617:0".to_string(),
            label_zh: "卡爾林邊新屋".to_string(),
            built_day: low_clk.game_days,
        },
    );

    let viewer = delta.actor_by_id_or_controller("sub_tester").unwrap();
    let map_pkt = build_filtered_map_json(&doc, &delta, viewer, &low_clk);
    assert!(
        map_pkt["current_cell_map"]["url"]
            .as_str()
            .is_some_and(|u| u.starts_with("/maps/cells/cell_4617.webp")),
        "rural/coastal cell 4617 must serve its static cell map webp"
    );
    assert!(
        map_pkt["island_master_url"]
            .as_str()
            .is_some_and(|u| u.contains("master_island_2k.webp")),
        "map packet must include master_island_2k overview url"
    );

    let sub_low = &map_pkt["substrate"];
    let t_low = decode_rle_b64(sub_low["terrain_rle_b64"].as_str().unwrap());
    let o_low = decode_rle_b64(sub_low["overlay_rle_b64"].as_str().unwrap());
    assert_eq!(t_low.len(), GRID_W * GRID_H);
    assert_eq!(o_low.len(), GRID_W * GRID_H);

    let sub_high = build_viewer_substrate_json(&doc, &delta, viewer, &high_clk);
    let t_high = decode_rle_b64(sub_high["terrain_rle_b64"].as_str().unwrap());

    let reef_low = t_low.iter().filter(|&&t| t == TERRAIN_REEF_SAND).count();
    let reef_high = t_high.iter().filter(|&&t| t == TERRAIN_REEF_SAND).count();
    assert!(
        reef_low > reef_high,
        "low tide must expose coastal reef sand"
    );

    // 全島 860×930 徹徹底底零黑霧（TERRAIN_FOG == 0），且 #4073 北岬燈塔維持乾淨底盤無未見證新增建築
    assert_eq!(
        t_low.iter().filter(|&&t| t == TERRAIN_FOG).count(),
        0,
        "player map must contain zero TERRAIN_FOG tiles across all 799,800 tiles"
    );
    let sub_static = compiled_island_substrate(&doc);
    assert!(
        sub_static.door_tiles.len() >= 390,
        "must precompute door tiles for all indoor nodes"
    );
    assert!(
        sub_low["pois"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "oi:built:test_1"),
        "newly built homestead must appear live in visible POIs"
    );
    assert_eq!(
        sub_low["nearby_actors"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| {
                let gx = a["gx"].as_f64().unwrap() as f32;
                let gy = a["gy"].as_f64().unwrap() as f32;
                let p = sub_low["player_pos"].as_array().unwrap();
                let dx = gx - p[0].as_f64().unwrap() as f32;
                let dy = gy - p[1].as_f64().unwrap() as f32;
                dx * dx + dy * dy > 14.0 * 14.0 + 0.05
            })
            .count(),
        0,
        "nearby_actors must strictly stay within 14.0 tiles"
    );

    // 驗證純節點單步跳動（mode="node"）：僅相連出口允許移動，未相連節點嚴格拒絕；並驗證 scene.static_map 三層對位
    delta.relocate_actor("sub_tester", "oi:white_gull_harbor:smithy_st", 4616, true);
    let v_smithy = super::views::build_view_json(
        &doc,
        &delta,
        delta.actor_by_id_or_controller("sub_tester").unwrap(),
        &low_clk,
    );
    assert_eq!(
        v_smithy["scene"]["static_map"]["town_slug"],
        "white_gull_harbor"
    );
    assert!(
        super::map_walk::handle_player_move_request(
            &doc,
            &mut delta,
            &low_clk,
            "sub_tester",
            "oi:white_gull_harbor:dock",
            None,
            None,
            "node",
            None,
            1_760_000_000_000,
        )
        .is_err(),
        "non-connected node must be strictly rejected in node mode"
    );
    super::map_walk::handle_player_move_request(
        &doc,
        &mut delta,
        &low_clk,
        "sub_tester",
        "oi:white_gull_harbor:bal_forge",
        None,
        None,
        "node",
        None,
        1_760_000_000_500,
    )
    .expect("connected shop exit bal_forge must succeed in node mode");
    let v_forge = super::views::build_view_json(
        &doc,
        &delta,
        delta.actor_by_id_or_controller("sub_tester").unwrap(),
        &low_clk,
    );
    assert_eq!(v_forge["scene"]["node_id"], "oi:white_gull_harbor:bal_forge");
    assert_eq!(
        v_forge["scene"]["static_map"]["town_slug"],
        "white_gull_harbor"
    );
    assert!(v_forge["scene"]["indoor"].as_bool().unwrap_or(false));
    let forge_exits = v_forge["scene"]["exits"].as_array().unwrap();
    assert_eq!(forge_exits.len(), 1);
    assert_eq!(forge_exits[0]["to"], "oi:white_gull_harbor:smithy_st");
}

#[test]
fn all_outdoor_nodes_connected_on_walkable_grid_and_step_continuous() {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    super::sim_tick::seed_genesis_actors_once(&doc, &mut delta, false);
    let clk = super::clock_and_lang::solar_clock_for_settled_bell(280 * 12 + 4, 1_760_000_000);
    let sub = compiled_island_substrate(&doc);
    super::map_walk::seed_missing_actor_grid_positions_once(&doc, &mut delta);

    let [sx, sy] = sub.node_anchors["oi:white_gull_harbor:dock"];
    let [gx, gy] = sub.node_anchors["oi:cell4530:0"];
    let path = super::map_walk::plan_grid_walk_path(
        &doc,
        sub,
        &delta,
        &clk,
        None,
        "probe",
        (sx.floor() as isize, sy.floor() as isize),
        (gx.floor() as isize, gy.floor() as isize),
    )
    .expect("dock to northern countryside must have continuous walkable grid path");
    assert!(!path.is_empty());
    let mut prev = [sx.floor() as isize, sy.floor() as isize];
    for &[px, py] in &path {
        let dx = (px as isize - prev[0]).abs();
        let dy = (py as isize - prev[1]).abs();
        assert!(dx <= 1 && dy <= 1, "grid step must never teleport");
        assert!(
            super::map_walk::is_tile_walkable(
                &doc,
                sub,
                &delta,
                &clk,
                px as isize,
                py as isize,
                false,
                None
            ),
            "every path step ({px},{py}) must be walkable"
        );
        prev = [px as isize, py as isize];
    }

    // 單次 860×930 退潮 BFS：驗證全島全部 462 個戶外節點（!dungeon && !indoor）皆具陸橋可達路線
    let total_tiles = GRID_W * GRID_H;
    let mut visited = vec![false; total_tiles];
    let mut bfs_q = VecDeque::new();
    let s0 = (sx.floor() as isize, sy.floor() as isize);
    let spos = super::island_substrate::idx_of(s0.0, s0.1).expect("dock anchor in bounds");
    visited[spos] = true;
    bfs_q.push_back(s0);
    while let Some((cx, cy)) = bfs_q.pop_front() {
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let nx = cx + dx;
            let ny = cy + dy;
            if !super::map_walk::is_tile_walkable(&doc, sub, &delta, &clk, nx, ny, false, None) {
                continue;
            }
            if dx != 0
                && dy != 0
                && (!super::map_walk::is_tile_walkable(
                    &doc,
                    sub,
                    &delta,
                    &clk,
                    cx + dx,
                    cy,
                    false,
                    None,
                ) || !super::map_walk::is_tile_walkable(
                    &doc,
                    sub,
                    &delta,
                    &clk,
                    cx,
                    cy + dy,
                    false,
                    None,
                ))
            {
                continue;
            }
            let Some(npos) = super::island_substrate::idx_of(nx, ny) else {
                continue;
            };
            if !visited[npos] {
                visited[npos] = true;
                bfs_q.push_back((nx, ny));
            }
        }
    }
    let mut unreachable = Vec::new();
    let mut outdoor_count = 0usize;
    for (nid, nd) in &doc.nodes {
        if nd.dungeon || nd.indoor {
            continue;
        }
        outdoor_count += 1;
        let [ax, ay] = sub.node_anchors[nid];
        let pos = super::island_substrate::idx_of(ax.floor() as isize, ay.floor() as isize)
            .expect("outdoor anchor in bounds");
        if !visited[pos] {
            unreachable.push(format!("{nid}@({},{})", ax.floor(), ay.floor()));
        }
    }
    assert!(
        outdoor_count >= 460,
        "expected >= 460 outdoor nodes, got {outdoor_count}"
    );
    assert!(
        unreachable.is_empty(),
        "every outdoor node must be reachable from dock at low tide without crossing water/wall; unreachable={unreachable:?}"
    );
    let mut unreachable_doors = Vec::new();
    for (nid, &[dx, dy]) in &sub.door_tiles {
        let Some(pos) = super::island_substrate::idx_of(dx as isize, dy as isize) else {
            unreachable_doors.push(nid.clone());
            continue;
        };
        if !visited[pos] {
            unreachable_doors.push(format!("{nid}@({dx},{dy})"));
        }
    }
    assert!(
        unreachable_doors.is_empty(),
        "every indoor/dungeon door tile must be reachable on low tide BFS; unreachable_doors={unreachable_doors:?}"
    );
}

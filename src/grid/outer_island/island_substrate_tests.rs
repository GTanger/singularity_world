//! 全島單一無縫連續底盤之物理與拓撲不變量回歸測試。

use std::collections::{BTreeSet, VecDeque};

use super::clock_and_lang::solar_clock_at;
use super::engine::load_static_world_doc;
use super::island_substrate::{
    BLDG_NONE, BLDG_ROOF, BLDG_VACANT_ROOF, GRID_H, GRID_W, ROAD_COUNTRY_TRACK, ROAD_NONE,
    ROAD_TOWN_COBBLE, TERRAIN_FOG, TERRAIN_REEF_SAND, TERRAIN_RIVER, TERRAIN_SEA,
    build_viewer_substrate_json, compiled_island_substrate,
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
    let pkt_str = map_pkt.to_string();
    assert!(
        !pkt_str.contains(".webp"),
        "map packet must contain 0 .webp references"
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

    // 確認未踏足地塊（如 4073 北岬燈塔）100% 保持迷霧 `TERRAIN_FOG` 且無道路或屋頂外洩
    let sub_static = compiled_island_substrate(&doc);
    let hidden_idx = sub_static.cell_index_by_id[&4073];
    for i in 0..(GRID_W * GRID_H) {
        if sub_static.cell_owner[i] == hidden_idx {
            assert_eq!(t_low[i], TERRAIN_FOG);
            assert_eq!(o_low[i], ROAD_NONE | (BLDG_NONE << 3));
        }
    }
    assert!(
        sub_low["pois"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "oi:built:test_1"),
        "newly built homestead must appear live in visible POIs"
    );
}

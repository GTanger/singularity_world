//! 「真實 × 沉浸 × 遊戲性」主畫面與三槽物理載體回歸測試（對齊審辯室 Room `cf1bfab9-16d7-4de8-b742-8076dd910468`）。

use super::actions::apply_world_action;
use super::clock_and_lang::solar_clock_at;
use super::engine::load_static_world_doc;
use super::household_economy::ensure_household_larders_seeded_once;
use super::movement::{add_carry_item, apply_setup_origin};
use super::sim_tick::seed_genesis_actors_once;
use super::types::{CarryItem, IslandWorldDelta, WorldStaticDoc};
use super::views::build_view_json;

fn fresh_world() -> (WorldStaticDoc, IslandWorldDelta) {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    ensure_household_larders_seeded_once(&mut delta);
    assert_eq!(delta.conservation_diff(&doc), 0);
    (doc, delta)
}

#[test]
fn no_telepathic_cross_node_trade_and_apprentice_or_basket_prevents_softlock() {
    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let day_clk = solar_clock_at(epoch + 1_200); // 晨鐘三響 08:00

    apply_setup_origin(
        &doc,
        &mut delta,
        &day_clk,
        "imm_tester",
        "海客林恩",
        "人類",
        "披著舊麻斗篷",
        "smith_apprentice",
    )
    .expect("setup player");
    let pid = delta
        .resolve_actor_id("imm_tester")
        .expect("resolve player");
    {
        let p = delta.actors.get_mut(&pid).unwrap();
        p.copper = 30;
        delta.municipal_treasury_copper -= 30;
        p.bound_weapon.wear_stage = 2;
        p.mark_mutated();
    }
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 1. 移步至巴爾鍛鐵鋪，將鋪內所有鐵匠與學徒暫移至市集廣場 -> 隔空修刀必須失敗且不扣半枚銅幣！
    delta.relocate_actor(&pid, "oi:white_gull_harbor:bal_forge", 4616, true);
    let forge_staff: Vec<String> = delta
        .node_occupants
        .get("oi:white_gull_harbor:bal_forge")
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|id| id != &pid)
        .collect();
    for sid in &forge_staff {
        delta.relocate_actor(sid, "oi:white_gull_harbor:plaza", 4616, true);
    }

    assert!(
        apply_world_action(
            &doc,
            &mut delta,
            &day_clk,
            "imm_tester",
            "repair_weapon",
            ""
        )
        .is_err(),
        "telepathic repair_weapon across nodes must be rejected when smith & apprentice left forge"
    );
    assert_eq!(delta.actors.get(&pid).unwrap().copper, 30);
    assert_eq!(delta.actors.get(&pid).unwrap().bound_weapon.wear_stage, 2);

    // 2. 將鍛鐵匠學徒單獨移回鍛鐵鋪 -> 有當值學徒在爐邊即可正常修刀！
    let app_id = forge_staff
        .iter()
        .find(|id| {
            delta
                .actors
                .get(*id)
                .is_some_and(|a| a.role.contains("學徒"))
        })
        .or_else(|| forge_staff.first())
        .expect("forge staff exists");
    delta.relocate_actor(app_id, "oi:white_gull_harbor:bal_forge", 4616, true);
    apply_world_action(
        &doc,
        &mut delta,
        &day_clk,
        "imm_tester",
        "repair_weapon",
        "",
    )
    .expect("repair succeeds when apprentice is on watch at forge");
    assert_eq!(delta.actors.get(&pid).unwrap().copper, 26);
    assert_eq!(delta.actors.get(&pid).unwrap().bound_weapon.wear_stage, 0);
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 3. 深夜酒館：掌櫃外出時，值夜學徒仍可辦理投宿；若兩人皆暫離櫃台，人工買糧拒絕、竹簍投幣自取成功！
    let night_clk = solar_clock_at(epoch + 450); // 03:00 深夜
    delta.relocate_actor(&pid, "oi:white_gull_harbor:rusty_anchor_tavern", 4616, true);
    apply_world_action(&doc, &mut delta, &night_clk, "imm_tester", "rent_inn", "")
        .expect("night inn rental succeeds via tavern staff at 3 AM");
    assert_eq!(delta.actors.get(&pid).unwrap().inn_nights_left, 1);

    let tav_staff: Vec<String> = delta
        .node_occupants
        .get("oi:white_gull_harbor:rusty_anchor_tavern")
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|id| {
            id != &pid
                && delta
                    .actors
                    .get(id)
                    .is_some_and(|a| a.role.contains("酒館掌櫃"))
        })
        .collect();
    for tid in &tav_staff {
        delta.relocate_actor(tid, "oi:white_gull_harbor:plaza", 4616, true);
    }
    assert!(
        apply_world_action(&doc, &mut delta, &day_clk, "imm_tester", "buy_ration", "").is_err(),
        "counter buy_ration must fail when both keeper and apprentice left counter"
    );
    apply_world_action(
        &doc,
        &mut delta,
        &day_clk,
        "imm_tester",
        "buy_ration",
        "self_basket|2",
    )
    .expect("honesty basket buy_ration succeeds using real tavern stock when counter unattended");
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn sell_specialty_selects_exact_item_and_enforces_quote_and_socket_invariants() {
    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let day_clk = solar_clock_at(epoch + 1_200);

    apply_setup_origin(
        &doc,
        &mut delta,
        &day_clk,
        "sell_tester",
        "拾木漢克",
        "矮人",
        "背著麻布袋",
        "forest_hunter",
    )
    .expect("setup");
    let pid = delta.resolve_actor_id("sell_tester").unwrap();
    {
        let p = delta.actors.get_mut(&pid).unwrap();
        add_carry_item(
            &mut p.backpack,
            CarryItem {
                slug: "rune_stone".to_string(),
                name: "古符文原石".to_string(),
                count: 1,
                weight_units: 1,
                ..CarryItem::default()
            },
        );
        add_carry_item(
            &mut p.backpack,
            CarryItem {
                slug: "driftwood".to_string(),
                name: "海灣漂流硬木".to_string(),
                count: 1,
                weight_units: 2,
                ..CarryItem::default()
            },
        );
        p.fatigue = 30;
        p.wound_part = Some("左腕瘀腫".to_string());
        p.mark_mutated();
    }

    // 在碼頭指定只賣「海灣漂流硬木（4銅）」，絕不誤賣排在前面的高價【古符文原石】！
    let dock_view = build_view_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &day_clk);
    let npcs = dock_view["scene"]["npcs"].as_array().unwrap();
    let buyer_tok = npcs
        .iter()
        .find(|n| {
            n["deeds"]
                .as_array()
                .is_some_and(|ds| ds.iter().any(|d| d["id"] == "sell_specialty"))
        })
        .and_then(|n| n["id"].as_str())
        .expect("dock buyer has per-item sell_specialty deeds");

    // 報價不符必須拒絕扣物
    assert!(
        apply_world_action(
            &doc,
            &mut delta,
            &day_clk,
            "sell_tester",
            "sell_specialty",
            &format!("{buyer_tok}|driftwood|99")
        )
        .is_err()
    );

    // 正確指定品項與報價成交：原石必須完好保留在背囊！
    apply_world_action(
        &doc,
        &mut delta,
        &day_clk,
        "sell_tester",
        "sell_specialty",
        &format!("{buyer_tok}|driftwood|4"),
    )
    .expect("sell exact driftwood");
    let p_after = delta.actors.get(&pid).unwrap();
    assert!(
        p_after
            .backpack
            .iter()
            .any(|i| i.slug == "rune_stone" && i.count == 1)
    );
    assert!(
        !p_after
            .backpack
            .iter()
            .any(|i| i.slug == "driftwood" && i.count > 0)
    );
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 全場景三槽載體與純名詞物件硬斷言（杜絕動詞待辦標題與無主動作回潮）
    let banned_verb_prefixes = [
        "端詳",
        "翻閱",
        "購買",
        "租下",
        "出售",
        "嚼塊",
        "向記籌手",
        "搭手",
    ];
    for nid in [
        "oi:white_gull_harbor:dock",
        "oi:white_gull_harbor:plaza",
        "oi:white_gull_harbor:bal_forge",
        "oi:white_gull_harbor:rusty_anchor_tavern",
        "oi:white_gull_harbor:herb_hall",
        "oi:white_gull_harbor:inn_upstairs",
    ] {
        delta.relocate_actor(&pid, nid, 4616, true);
        let v = build_view_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &day_clk);
        for act in v["scene"]["actions"].as_array().unwrap() {
            let ck = act["carrier_kind"].as_str().unwrap_or("");
            let cid = act["carrier_id"].as_str().unwrap_or("");
            assert!(
                matches!(ck, "person" | "object" | "self") && !cid.is_empty(),
                "orphan action without physical carrier socket at {nid}: {act}"
            );
        }
        for obj in v["scene"]["objects"].as_array().unwrap() {
            let oname = obj["name"].as_str().unwrap_or("");
            assert!(
                !oname.is_empty() && !banned_verb_prefixes.iter().any(|vp| oname.starts_with(vp)),
                "scene.objects must be pure in-world nouns, got verb-prefixed '{oname}' at {nid}"
            );
        }
    }
}

#[test]
fn trial_pit_sliced_map_tiles_are_zero_spoiler_and_verified_on_disk() {
    let (epoch, _) = crate::game::game_time_cfg();
    let (doc, mut delta) = fresh_world();
    let clk = solar_clock_at(epoch + 1_200);

    let all_pit_rooms = [
        ("oi:dng_pit:entry", "seg_c181917d323d.webp"),
        ("oi:dng_pit:hall", "seg_f584bfecac23.webp"),
        ("oi:dng_pit:side", "seg_e1736f150e10.webp"),
        ("oi:dng_pit:stele", "seg_b4241cd9c5bc.webp"),
        ("oi:dng_pit:lair", "seg_18dd21289df4.webp"),
        ("oi:dng_pit:altar", "seg_97c8bfa33e77.webp"),
    ];

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (rid, expected_file) in all_pit_rooms {
        let sp = super::views::dungeon_room_tile_spec(rid)
            .unwrap_or_else(|| panic!("missing dungeon_room_tile_spec for {rid}"));
        assert!(sp.tile_url.contains(expected_file));
        let rel = sp
            .tile_url
            .split('?')
            .next()
            .unwrap()
            .trim_start_matches('/');
        let disk_path = manifest_dir.join("web").join(rel);
        let meta = std::fs::metadata(&disk_path)
            .unwrap_or_else(|e| panic!("missing slice file {}: {e}", disk_path.display()));
        assert!(
            meta.len() > 10_000,
            "slice too small: {}",
            disk_path.display()
        );
    }
    // 確保 web/maps 下絕無全幅原圖殘留，防止翻網址偷看未探索石室
    for forbidden in [
        "web/maps/trial_pit.webp",
        "web/maps/trial_pit_full.webp",
        "web/maps/trial_pit/master.webp",
    ] {
        assert!(
            !manifest_dir.join(forbidden).exists(),
            "full master dungeon map must never reside in web/: {forbidden}"
        );
    }

    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "pit_fog_tester",
        "探坑旅人",
        "人類",
        "披斗篷",
        "forest_hunter",
    )
    .expect("create pit_fog_tester");
    let pid = delta.resolve_actor_id("pit_fog_tester").unwrap();

    // 初踏入塌陷洞口：地底坑道僅回傳洞口 1 室且絕無任何 `.webp` 圖片與未探石室洩漏！
    delta.relocate_actor(&pid, "oi:dng_pit:entry", 4529, true);
    let m1 =
        super::views::build_filtered_map_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &clk);
    let raw1 = serde_json::to_string(&m1).unwrap();
    assert!(
        !raw1.contains(".webp"),
        "home map packet must never reference .webp: {raw1}"
    );
    assert!(raw1.contains("oi:dng_pit:entry"));
    for hidden_id in [
        "oi:dng_pit:hall",
        "oi:dng_pit:side",
        "oi:dng_pit:stele",
        "oi:dng_pit:lair",
        "oi:dng_pit:altar",
        "seg_",
        "rm_",
    ] {
        assert!(
            !raw1.contains(hidden_id),
            "unvisited room id or image leaked in map packet: {hidden_id}"
        );
    }
    assert_eq!(m1["dungeon_local"]["rooms"].as_array().unwrap().len(), 1);

    // 推進至斷柱前廳與古銘文石室：應解鎖 3 室程序草圖，偏室、蛛巢與凝石台依然 0 洩漏！
    delta.relocate_actor(&pid, "oi:dng_pit:hall", 4529, true);
    delta.relocate_actor(&pid, "oi:dng_pit:stele", 4529, true);
    let m3 =
        super::views::build_filtered_map_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &clk);
    let raw3 = serde_json::to_string(&m3).unwrap();
    assert!(!raw3.contains(".webp"));
    assert_eq!(m3["dungeon_local"]["rooms"].as_array().unwrap().len(), 3);
    assert!(raw3.contains("oi:dng_pit:entry"));
    assert!(raw3.contains("oi:dng_pit:hall"));
    assert!(raw3.contains("oi:dng_pit:stele"));
    for hidden_id in ["oi:dng_pit:side", "oi:dng_pit:lair", "oi:dng_pit:altar"] {
        assert!(
            !raw3.contains(hidden_id),
            "unvisited deep/side room leaked in map packet: {hidden_id}"
        );
    }

    // 設計者全開大地圖（/world_atlas）快照則完整回傳全部 6 室之手繪切片座標
    let atlas_snap = super::atlas_observatory::build_atlas_snapshot_json(&doc, &delta);
    let dng_rooms = atlas_snap["map"]["dungeon_rooms"].as_array().unwrap();
    assert_eq!(dng_rooms.len(), 6);
    for r in dng_rooms {
        assert!(
            r["tile"]["url"]
                .as_str()
                .is_some_and(|u| u.starts_with("/maps/trial_pit/seg_")),
            "atlas dungeon_rooms missing tile url: {r}"
        );
    }

    // 驗證 `/world_atlas` 保留之 36 格靜態插畫設定仍存在於磁碟，但正式首頁 `/api/island/map` 已全面切為零圖檔無縫水彩底盤
    let mut verified_cell_maps = 0usize;
    for c in doc.cells.values() {
        if matches!(c.kind.as_str(), "rural_settlement" | "plague_ruin")
            || matches!(c.cell_id, 4529 | 4617)
        {
            let csp = super::cell_maps::cell_illustrated_map_spec(c.cell_id).unwrap_or_else(|| {
                panic!(
                    "missing cell_illustrated_map_spec for #{} ({})",
                    c.cell_id, c.title
                )
            });
            let rel = csp.url.split('?').next().unwrap().trim_start_matches('/');
            let disk_path = manifest_dir.join("web").join(rel);
            let meta = std::fs::metadata(&disk_path)
                .unwrap_or_else(|e| panic!("missing cell webp {}: {e}", disk_path.display()));
            assert!(
                meta.len() > 40_000,
                "cell webp too small: {}",
                disk_path.display()
            );
            verified_cell_maps += 1;
        }
    }
    assert_eq!(verified_cell_maps, 36);
    for (cid, sample_node) in [
        (4529u32, "oi:cell4529:0"),
        (4432u32, "oi:burg_4432:yard"),
        (4069u32, "oi:burg_4069:yard"),
        (4849u32, "oi:cell4849:0"),
    ] {
        delta.relocate_actor(&pid, sample_node, cid, true);
        let mc = super::views::build_filtered_map_json(
            &doc,
            &delta,
            delta.actors.get(&pid).unwrap(),
            &clk,
        );
        assert!(mc["current_cell_map"].is_null());
        assert!(
            mc["substrate"]["terrain_rle_b64"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        );
        assert!(!serde_json::to_string(&mc).unwrap().contains(".webp"));
    }
    // 磁碟切片檔案檢核仍保留完整，且首頁 /api/island/map 在地底石室同樣 0 .webp 外洩
    let all_room_closeups = [
        ("oi:dng_pit:entry", "rm_8a41e207c9b3.webp"),
        ("oi:dng_pit:hall", "rm_3d7c95f104e8.webp"),
        ("oi:dng_pit:side", "rm_6f19b82a5d41.webp"),
        ("oi:dng_pit:stele", "rm_c450e73b91a6.webp"),
        ("oi:dng_pit:lair", "rm_52b8d094e7c2.webp"),
        ("oi:dng_pit:altar", "rm_9e36a418f0b7.webp"),
    ];
    for (rid, rm_file) in all_room_closeups {
        let sp = super::views::dungeon_room_tile_spec(rid).unwrap();
        assert!(sp.room_url.contains(rm_file));
        let rel = sp
            .room_url
            .split('?')
            .next()
            .unwrap()
            .trim_start_matches('/');
        let disk_path = manifest_dir.join("web").join(rel);
        let meta = std::fs::metadata(&disk_path)
            .unwrap_or_else(|e| panic!("missing pit room closeup {}: {e}", disk_path.display()));
        assert!(
            meta.len() > 40_000,
            "pit room closeup too small: {}",
            disk_path.display()
        );
    }
    delta.relocate_actor(&pid, "oi:dng_pit:entry", 4529, true);
    let md =
        super::views::build_filtered_map_json(&doc, &delta, delta.actors.get(&pid).unwrap(), &clk);
    let md_raw = serde_json::to_string(&md).unwrap();
    assert!(!md_raw.contains(".webp"));
    assert!(!md_raw.contains("rm_8a41e207c9b3.webp"));
    assert!(!md_raw.contains("rm_52b8d094e7c2.webp"));
    assert!(!md_raw.contains("rm_9e36a418f0b7.webp"));
}

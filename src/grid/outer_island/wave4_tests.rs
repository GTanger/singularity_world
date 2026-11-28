//! 第四刀（拆骨補實版）整合測試：嚴格驗證零無中生有、真實庫存扣減、冷爐重燃、斷橋實修、農田種子與 `tile_diffs` 持久化、手札真內文與 NPC 同構主迴圈。

use super::actions::apply_world_action;
use super::clock_and_lang::solar_clock_at;
use super::engine::load_static_world_doc;
use super::living_sim_and_farm::is_farmable_node;
use super::movement::{add_carry_item, apply_setup_origin};
use super::persist_schema::WorldExtrasBlob;
use super::types::{CarryItem, IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};
use super::views::build_view_json;
use super::wild_hunt_and_events::tick_npc_unified_world_habits;

fn seeded_delta_with_player(doc: &WorldStaticDoc, pid: &str, name: &str) -> IslandWorldDelta {
    let mut delta = IslandWorldDelta {
        genesis_seeded: true,
        genesis_total_copper: doc.ledger_genesis.municipal_treasury_copper
            + doc.ledger_genesis.silver_bank_reserve_copper
            + 60,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    let clock = solar_clock_at(1_764_300_000);
    apply_setup_origin(
        doc,
        &mut delta,
        &clock,
        pid,
        name,
        "人類",
        "披著舊斗篷",
        "smith_apprentice",
    )
    .expect("setup player");
    let aid = delta.resolve_actor_id(pid).expect("aid");
    if let Some(a) = delta.actors.get_mut(&aid) {
        a.copper = 60;
        a.fatigue = 55;
        a.mind.stm.affect.hunger = 60;
        a.inn_nights_left = 2;
    }
    delta
}

fn spawn_npc_at(
    delta: &mut IslandWorldDelta,
    id: &str,
    name: &str,
    role: &str,
    node_id: &str,
    copper: i64,
) {
    let mut npc = UnifiedMindBody::default();
    npc.actor_id = id.to_string();
    npc.display_name = name.to_string();
    npc.role = role.to_string();
    npc.setup_done = true;
    npc.alive = true;
    npc.node_id = node_id.to_string();
    npc.work_node = node_id.to_string();
    npc.home_node = node_id.to_string();
    npc.copper = copper;
    delta.genesis_total_copper += copper;
    delta.upsert_actor(npc);
}

#[test]
fn living_sim_rejects_conjuring_and_consumes_real_stock_and_materials() {
    let doc = load_static_world_doc();
    let mut delta = seeded_delta_with_player(&doc, "w4_life", "阿晴");
    let clock = solar_clock_at(1_764_300_000);
    let aid = delta.resolve_actor_id("w4_life").expect("aid");

    let tav_nd = doc
        .nodes
        .get("oi:white_gull_harbor:rusty_anchor_tavern")
        .expect("rusty_anchor_tavern");
    delta.relocate_actor(&aid, &tav_nd.id, tav_nd.cell_id, true);

    // 1. 無在場掌櫃時拒賣熱食、拒開浴桶（嚴禁公庫自動販賣機！）
    assert!(apply_world_action(&doc, &mut delta, &clock, "w4_life", "enjoy_hot_meal", "").is_err());
    assert!(apply_world_action(&doc, &mut delta, &clock, "w4_life", "take_warm_bath", "").is_err());

    // 2. 放入真實掌櫃但鋪倉沒糧 → 誠實售罄！放入 1 份真實海魚後才可烹煮並實扣該份海魚！
    spawn_npc_at(&mut delta, "npc:tav_keeper", "瑪莎掌櫃", "酒館掌櫃", &tav_nd.id, 20);
    let sold_out = apply_world_action(&doc, &mut delta, &clock, "w4_life", "enjoy_hot_meal", "");
    assert!(sold_out.is_err(), "empty tavern stock must reject hot meal");

    if let Some(k) = delta.actors.get_mut("npc:tav_keeper") {
        let locker = k.inn_lockers.entry(tav_nd.id.clone()).or_default();
        add_carry_item(
            locker,
            CarryItem {
                slug: "fresh_fish_basket".to_string(),
                name: "鮮活溪海魚簍".to_string(),
                count: 1,
                weight_units: 2,
                ..CarryItem::default()
            },
        );
    }
    let meal_msg = apply_world_action(&doc, &mut delta, &clock, "w4_life", "enjoy_hot_meal", "")
        .expect("hot meal with real stock ok");
    assert!(meal_msg.contains("鮮活溪海魚簍"));
    assert_eq!(delta.conservation_diff(&doc), 0);
    assert!(
        delta.actors["npc:tav_keeper"]
            .inn_lockers
            .get(&tav_nd.id)
            .is_none_or(|v| v.is_empty()),
        "tavern storeroom stock must be consumed"
    );

    let bath_msg = apply_world_action(&doc, &mut delta, &clock, "w4_life", "take_warm_bath", "")
        .expect("warm bath with keeper ok");
    assert!(bath_msg.contains("溫水"));
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 3. 穿搭與居所傢俱：缺料必拒、給真料才成、且零設計黑話與零數字外洩
    assert!(
        apply_world_action(&doc, &mut delta, &clock, "w4_life", "change_outfit", "waxed_hunter")
            .is_err(),
        "must reject outfit without real material"
    );
    spawn_npc_at(&mut delta, "npc:carp", "木匠老周", "木匠", &tav_nd.id, 10);
    if let Some(a) = delta.actors.get_mut(&aid) {
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "beast_tallow".to_string(),
                name: "精煉野獸脂塊".to_string(),
                count: 1,
                weight_units: 2,
                ..CarryItem::default()
            },
        );
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "iron_ore".to_string(),
                name: "粗鐵礦石".to_string(),
                count: 1,
                weight_units: 4,
                ..CarryItem::default()
            },
        );
    }
    let outfit_msg = apply_world_action(
        &doc,
        &mut delta,
        &clock,
        "w4_life",
        "change_outfit",
        "waxed_hunter",
    )
    .expect("tailor outfit with real beast_tallow ok");
    assert!(outfit_msg.contains("防風蠟布機能獵裝"));
    assert_eq!(delta.conservation_diff(&doc), 0);

    let furn_msg = apply_world_action(
        &doc,
        &mut delta,
        &clock,
        "w4_life",
        "buy_home_furnishing",
        "warm_brazier",
    )
    .expect("craft furnishing with real iron_ore ok");
    assert!(furn_msg.contains("鑄鐵烘衣炭火盆"));
    assert_eq!(delta.conservation_diff(&doc), 0);

    let viewer = delta.actors.get(&aid).expect("viewer");
    let view = build_view_json(&doc, &delta, viewer, &clock);
    let ls = &view["player"]["lifestyle"];
    assert_eq!(ls["outfit_name"].as_str(), Some("防風蠟布機能獵裝"));
    let mastery = view["player"]["mastery"].as_str().unwrap_or("");
    assert!(!mastery.chars().any(|c| c.is_ascii_digit()), "mastery leaked digit: {mastery}");
}

#[test]
fn farm_plot_requires_real_seeds_updates_tile_diffs_and_persists_in_extras() {
    let doc = load_static_world_doc();
    let mut delta = seeded_delta_with_player(&doc, "w4_farm", "老墾");
    let day0 = solar_clock_at(1_764_300_000);
    let aid = delta.resolve_actor_id("w4_farm").expect("aid");

    let farm_nd = doc
        .nodes
        .values()
        .find(|n| is_farmable_node(&doc, n))
        .expect("farmable outdoor node");
    delta.relocate_actor(&aid, &farm_nd.id, farm_nd.cell_id, true);

    // 無種子必拒（徹底防無中生有）
    assert!(
        apply_world_action(&doc, &mut delta, &day0, "w4_farm", "sow_farm_plot", "herb_garden")
            .is_err(),
        "sowing without real seeds must fail"
    );

    if let Some(a) = delta.actors.get_mut(&aid) {
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "herb_bundle".to_string(),
                name: "山林止血草束".to_string(),
                count: 1,
                weight_units: 1,
                ..CarryItem::default()
            },
        );
    }
    let sow_msg = apply_world_action(
        &doc,
        &mut delta,
        &day0,
        "w4_farm",
        "sow_farm_plot",
        "herb_garden",
    )
    .expect("sow with real herb_bundle ok");
    assert!(sow_msg.contains("山林止血藥圃"));
    assert_eq!(delta.farm_plots.len(), 1);
    assert!(!delta.tile_diffs.is_empty(), "sow_farm_plot must mutate real map tile_diffs");

    // 驗證 WorldExtrasBlob 持久化往返（重啟伺服器絕不丟失農田！）
    let blob = WorldExtrasBlob {
        farm_plots: delta.farm_plots.clone(),
        ..WorldExtrasBlob::default()
    };
    let raw = serde_json::to_string(&blob).expect("serialize extras");
    let restored: WorldExtrasBlob = serde_json::from_str(&raw).expect("deserialize extras");
    assert_eq!(restored.farm_plots.len(), 1);

    let tend_msg =
        apply_world_action(&doc, &mut delta, &day0, "w4_farm", "tend_farm_plot", "")
            .expect("tend ok");
    assert!(tend_msg.contains("山林止血藥圃"));

    let mut day1 = day0.clone();
    day1.game_days += 1;
    let harv_msg =
        apply_world_action(&doc, &mut delta, &day1, "w4_farm", "harvest_farm_plot", "")
            .expect("harvest ok");
    assert!(harv_msg.contains("山林止血草束"));
    assert!(delta.farm_plots.is_empty());
}

#[test]
fn real_crisis_events_only_fire_on_true_world_crises_and_zero_atm() {
    let doc = load_static_world_doc();
    let mut delta = seeded_delta_with_player(&doc, "w4_hunt", "巡獵者");
    let mut clock = solar_clock_at(1_764_300_000);
    let aid = delta.resolve_actor_id("w4_hunt").expect("aid");

    // 1. 驗證重複碼頭事件已拔除、且鍛爐助燃在未冷爐時拒絕，真冷爐交煤才補回燃料重燃！
    assert!(
        apply_world_action(&doc, &mut delta, &clock, "w4_hunt", "resolve_town_event", "harbor_tide_salvage")
            .is_err(),
        "duplicate harbor_tide_salvage must be removed"
    );
    let smithy_nd = doc
        .nodes
        .get("oi:white_gull_harbor:bal_forge")
        .expect("bal_forge");
    delta.relocate_actor(&aid, &smithy_nd.id, smithy_nd.cell_id, true);
    spawn_npc_at(&mut delta, "npc:bal_smith", "鐵砧巴爾", "鍛鐵匠", &smithy_nd.id, 30);
    delta.live_commodities.shop_stocks_seeded = true;
    // 爐倉無煤（冷爐）→ 交入 1 塊精煤塊應真正補進巴爾店倉並重燃！
    assert!(super::live_commodities::is_smithy_forge_cold(&delta, &smithy_nd.id));
    if let Some(a) = delta.actors.get_mut(&aid) {
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "black_coal".to_string(),
                name: "北林精煤塊".to_string(),
                count: 1,
                weight_units: 4,
                ..CarryItem::default()
            },
        );
    }
    let relight_msg = apply_world_action(
        &doc,
        &mut delta,
        &clock,
        "w4_hunt",
        "resolve_town_event",
        "forge_ember_relay",
    )
    .expect("cold forge coal relay ok");
    assert!(relight_msg.contains("重燃爐火"));
    assert!(
        !super::live_commodities::is_smithy_forge_cold(&delta, &smithy_nd.id),
        "smithy forge must genuinely relight after coal delivery"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 2. 市集廣場：橋全完好時拒發修橋賞銀；真實橋損＋繳真原木才修回 100！
    clock.is_night = false;
    let plaza = doc.nodes.get("oi:white_gull_harbor:plaza").expect("plaza");
    delta.relocate_actor(&aid, &plaza.id, plaza.cell_id, true);
    super::live_land::ensure_live_land_seeded(&doc, &mut delta, clock.game_days);
    for cl in delta.live_land.cells.values_mut() {
        cl.bridge_integrity = 100;
    }
    assert!(
        apply_world_action(&doc, &mut delta, &clock, "w4_hunt", "resolve_town_event", "bridge_relief_drive")
            .is_err(),
        "intact bridges must not spawn relief event"
    );
    let damaged_cid = *delta
        .live_land
        .cells
        .iter_mut()
        .find(|(_, cl)| cl.has_river_crossing)
        .map(|(cid, cl)| {
            cl.bridge_integrity = 50;
            cid
        })
        .expect("river crossing cell");
    if let Some(a) = delta.actors.get_mut(&aid) {
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "timber_log".to_string(),
                name: "灰松厚原木".to_string(),
                count: 1,
                weight_units: 4,
                ..CarryItem::default()
            },
        );
    }
    let br_msg = apply_world_action(
        &doc,
        &mut delta,
        &clock,
        "w4_hunt",
        "resolve_town_event",
        "bridge_relief_drive",
    )
    .expect("real bridge repair ok");
    assert!(br_msg.contains("恢復車馬通行"));
    assert_eq!(delta.live_land.cells[&damaged_cid].bridge_integrity, 100);
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 3. 巡防巨獸風聞：零免費發銅幣！
    let copper_before = delta.actors[&aid].copper;
    let watch_msg = apply_world_action(
        &doc,
        &mut delta,
        &clock,
        "w4_hunt",
        "resolve_town_event",
        "watch_bounty_brief",
    )
    .expect("watch rumor ok");
    assert!(watch_msg.contains("出城進山野"));
    assert_eq!(delta.actors[&aid].copper, copper_before, "watch rumor must pay 0 free copper");
}

#[test]
fn npc_unified_habits_and_true_manual_meta_same_method_different_phase() {
    let doc = load_static_world_doc();
    let mut delta = seeded_delta_with_player(&doc, "w4_alpha", "甲師");
    let clock = solar_clock_at(1_764_300_000);
    let aid_a = delta.resolve_actor_id("w4_alpha").expect("aid_a");

    // 1. 著譜寫入真實 manual_meta，交由不同骨相的乙徒研讀，驗證真同法不同相！
    apply_world_action(&doc, &mut delta, &clock, "w4_alpha", "write_martial_manual", "opening")
        .expect("write manual ok");
    let book = delta.actors[&aid_a]
        .backpack
        .iter()
        .find(|i| i.slug.starts_with("manual_"))
        .cloned()
        .expect("manual item");
    let meta = book.manual_meta.as_ref().expect("manual_meta persisted");
    assert_eq!(meta.author_name, "甲師");
    assert_eq!(meta.slot_code, "opening");
    assert_eq!(meta.motions.len(), 5);

    spawn_npc_at(
        &mut delta,
        "npc:disciple",
        "乙徒",
        "學徒",
        "oi:white_gull_harbor:plaza",
        10,
    );
    if let Some(b) = delta.actors.get_mut("npc:disciple") {
        add_carry_item(&mut b.backpack, book.clone());
    }
    let study_msg =
        super::martial_art_and_manuals::apply_study_martial_manual(&mut delta, &clock, "npc:disciple", &book.slug)
            .expect("disciple study ok");
    assert!(study_msg.contains("甲師"));
    assert!(study_msg.contains("原譜載"));

    // 2. NPC 主迴圈同構：持韌藤自動綁戴護腕＋同場交換真實背景記憶！
    spawn_npc_at(
        &mut delta,
        "npc:peer_two",
        "丙鄰",
        "碼頭苦力",
        "oi:white_gull_harbor:plaza",
        10,
    );
    if let Some(b) = delta.actors.get_mut("npc:disciple") {
        add_carry_item(
            &mut b.backpack,
            CarryItem {
                slug: "tough_vine".to_string(),
                name: "韌藤纖維".to_string(),
                count: 2,
                weight_units: 1,
                ..CarryItem::default()
            },
        );
    }
    let target_bell = (4 - (delta.actors["npc:disciple"].soul_seed % 4)) as i64;
    tick_npc_unified_world_habits(&doc, &mut delta, &clock, "npc:disciple", target_bell);
    assert!(
        delta.actors["npc:disciple"].body_gear.arms.is_some(),
        "NPC should bind and equip vine bracers via unified habit tick"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

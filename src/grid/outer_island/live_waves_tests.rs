//! 銀鷗島第四刀整合測試：哥布林劫掠屯贓追回、破屋佔據清場、落敗原石異變頭目、生老病死繼承、絕戶符文復歸與新生接班。

use super::actions::apply_world_action;
use super::clock_and_lang::{WORLD_OPEN_DAY, solar_clock_for_settled_bell};
use super::engine::load_static_world_doc;
use super::explore_and_combat::{apply_explore, resolve_skirmish_turn, trigger_non_lethal_defeat};
use super::household_economy::ensure_household_larders_seeded_once;
use super::live_land::{ensure_live_land_seeded, house_is_ruined};
use super::live_lifecycle::{
    actor_lived_years, execute_npc_passing_and_inheritance, tick_daily_lifecycle,
};
use super::live_wilds::{ensure_live_wilds_seeded, has_squatter_at_node, tick_daily_live_wilds};
use super::movement::{add_carry_item, apply_setup_origin};
use super::phase_and_vessels::{count_world_active_runes, ensure_chaos_rune_pool_seeded};
use super::sim_tick::seed_genesis_actors_once;
use super::transit_and_cargo::ensure_transit_vehicles_initialized;
use super::types::{
    BoundTool, CarryItem, ContractRecord, DroppedLootBag, IslandWorldDelta, VehicleCargoCrate,
    WorldStaticDoc,
};

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
    ensure_live_land_seeded(&doc, &mut delta, WORLD_OPEN_DAY);
    ensure_live_wilds_seeded(&doc, &mut delta, WORLD_OPEN_DAY);
    ensure_chaos_rune_pool_seeded(&mut delta);
    assert_eq!(delta.conservation_diff(&doc), 0);
    (doc, delta)
}

#[test]
fn goblin_band_shortage_raids_carriage_hoard_and_camp_raid_reclaims_copper_conserved() {
    let (doc, mut delta) = fresh_world();
    ensure_transit_vehicles_initialized(&doc, &mut delta);
    delta.live_wilds.last_wilds_day = WORLD_OPEN_DAY;

    // 將高脊礦岩坡（4430）鐵礦採空、部族缺糧、並將陸路貨車置於鄰格 4429（灰松石堡）裝入貨物且無護衛
    delta.cell_stocks.insert(4430, 1);
    if let Some(b) = delta.live_wilds.bands.get_mut(&4430) {
        b.food_stores = 1;
        b.raid_pressure = 56;
    }
    let head_before = delta.live_wilds.bands.get(&4430).unwrap().headcount;
    let car_idx = delta
        .transit_vehicles
        .iter()
        .position(|v| v.mode == "carriage")
        .expect("carriage exists");
    let stop_4429 = delta.transit_vehicles[car_idx]
        .route_nodes
        .iter()
        .position(|nid| doc.nodes.get(nid).is_some_and(|n| n.cell_id == 4429))
        .expect("ew carriage visits cell 4429");
    delta.transit_vehicles[car_idx].route_idx = stop_4429;
    delta.transit_vehicles[car_idx].passengers.clear();
    delta.transit_vehicles[car_idx]
        .cargo_hold
        .push(VehicleCargoCrate {
            slug: "wheat_grain".to_string(),
            name_zh: "飽滿黑麥穀袋".to_string(),
            count: 2,
            dest_node: "town_any".to_string(),
            owner_id: "cell:4429".to_string(),
        });

    // 同時於 4430 放置一筆遺落行囊（自公庫撥出 5 銅維持守恆），驗證斥候摸走遺落包銅幣亦 100% 對帳
    delta.municipal_treasury_copper -= 5;
    delta
        .dropped_loot_by_node
        .entry("oi:cell4430:0".to_string())
        .or_default()
        .push(DroppedLootBag {
            owner_id: "tester:drop".to_string(),
            copper: 5,
            items: Vec::new(),
            promoted_boss_name: None,
        });

    let clk1 = solar_clock_for_settled_bell((WORLD_OPEN_DAY + 1) * 12 + 5, 1_700_000_000);
    tick_daily_live_wilds(&doc, &mut delta, &clk1);

    assert!(
        !delta.dropped_loot_by_node.contains_key("oi:cell4430:0"),
        "scavenger band must scoop abandoned loot bag into hoard"
    );
    let band_after_raid = delta.live_wilds.bands.get(&4430).unwrap();
    assert!(
        band_after_raid.hoard_copper > 5,
        "goblin band raid + scavenge must deposit stolen copper into hoard"
    );
    assert!(
        band_after_raid.headcount > head_before,
        "stolen grain feeds band reproduction"
    );
    assert_eq!(
        delta.live_wilds.total_band_hoard_copper(),
        delta.goblin_hoard_copper
    );
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 旅人趕赴 4430 營寨清剿頭目，全額追回藏匿銅幣後帳本依然嚴格守恆
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk1,
        "camp_raider",
        "清剿漢斯",
        "矮人",
        "提著鐵錘",
        "smith_apprentice",
    )
    .expect("setup raider");
    let pid = delta.resolve_actor_id("camp_raider").unwrap();
    if let Some(p) = delta.actors.get_mut(&pid) {
        p.copper = 10;
        delta.municipal_treasury_copper -= 10;
        p.bound_weapon.rune_mark = Some("穩固鐵紋".to_string());
        p.mark_mutated();
    }
    let stolen_amt = delta.goblin_hoard_copper;
    delta.relocate_actor(&pid, "oi:cell4430:camp", 4430, true);
    apply_world_action(&doc, &mut delta, &clk1, &pid, "raid_goblin_camp", "4430")
        .expect("start camp raid skirmish");
    let win_msg = resolve_skirmish_turn(&doc, &mut delta, &clk1, &pid, "skirmish_heavy")
        .expect("win camp raid");
    assert!(win_msg.contains("藏匿銅幣"));
    assert_eq!(delta.goblin_hoard_copper, 0);
    assert_eq!(delta.live_wilds.total_band_hoard_copper(), 0);
    assert_eq!(delta.actors.get(&pid).unwrap().copper, 10 + stolen_amt);
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn ruined_vacant_house_gets_squatter_that_blocks_buy_and_repair_until_evicted() {
    let (doc, mut delta) = fresh_world();
    let house_nid = doc
        .households
        .iter()
        .find(|h| h.vacant_for_sale)
        .map(|h| h.node_id.clone())
        .expect("vacant house exists");

    delta
        .live_land
        .house_integrity
        .insert(house_nid.clone(), 20);
    assert!(house_is_ruined(&delta, &house_nid));

    // 第 4 的倍數日觸發失修破屋遷入佔巢者
    let day4 = ((WORLD_OPEN_DAY / 4) + 1) * 4;
    delta.live_wilds.last_wilds_day = day4 - 1;
    let clk4 = solar_clock_for_settled_bell(day4 * 12 + 5, 1_700_000_000);
    tick_daily_live_wilds(&doc, &mut delta, &clk4);

    assert!(
        has_squatter_at_node(&delta, &house_nid).is_some(),
        "ruined house should attract a squatter"
    );

    apply_setup_origin(
        &doc,
        &mut delta,
        &clk4,
        "house_cleaner",
        "整屋人",
        "人類",
        "帶著柴刀",
        "smith_apprentice",
    )
    .expect("setup");
    let pid = delta.resolve_actor_id("house_cleaner").unwrap();
    if let Some(p) = delta.actors.get_mut(&pid) {
        p.copper = 400;
        delta.municipal_treasury_copper -= 400;
        add_carry_item(
            &mut p.backpack,
            CarryItem {
                slug: "timber_log".to_string(),
                name: "灰松厚原木".to_string(),
                count: 1,
                weight_units: 4,
                ..CarryItem::default()
            },
        );
        p.mark_mutated();
    }
    delta.relocate_actor(&pid, &house_nid, 4616, true);

    assert!(
        apply_world_action(
            &doc,
            &mut delta,
            &clk4,
            &pid,
            "repair_vacant_house",
            &house_nid
        )
        .is_err(),
        "repair must fail while squatter occupies house"
    );
    assert!(
        apply_world_action(&doc, &mut delta, &clk4, &pid, "buy_vacant_house", "").is_err(),
        "buying must fail while squatter occupies house"
    );

    apply_world_action(&doc, &mut delta, &clk4, &pid, "evict_squatter", &house_nid)
        .expect("start eviction skirmish");
    let evict_msg = resolve_skirmish_turn(&doc, &mut delta, &clk4, &pid, "skirmish_heavy")
        .expect("evict squatter");
    assert!(evict_msg.contains("重歸清靜"));
    assert!(has_squatter_at_node(&delta, &house_nid).is_none());

    apply_world_action(
        &doc,
        &mut delta,
        &clk4,
        &pid,
        "repair_vacant_house",
        &house_nid,
    )
    .expect("repair succeeds after eviction");
    apply_world_action(&doc, &mut delta, &clk4, &pid, "buy_vacant_house", "")
        .expect("buy succeeds after eviction");
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn defeat_with_rune_stone_promotes_mutated_boss_and_conserves_world_rune_budget() {
    let (doc, mut delta) = fresh_world();
    let clk = solar_clock_for_settled_bell(WORLD_OPEN_DAY * 12 + 5, 1_700_000_000);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "rune_bearer",
        "持石客",
        "精靈",
        "裹著麻袍",
        "forest_hunter",
    )
    .expect("setup");
    let pid = delta.resolve_actor_id("rune_bearer").unwrap();
    delta.chaos_rune_pool = delta.chaos_rune_pool.saturating_sub(1);
    if let Some(p) = delta.actors.get_mut(&pid) {
        add_carry_item(
            &mut p.backpack,
            CarryItem {
                slug: "rune_stone".to_string(),
                name: "古符文原石".to_string(),
                count: 1,
                weight_units: 2,
                ..CarryItem::default()
            },
        );
        p.mark_mutated();
    }
    let total_runes_before = u16::from(count_world_active_runes(&delta)) + delta.chaos_rune_pool;
    assert!(total_runes_before <= 12);

    let fight_node = "oi:cell4526:0";
    delta.relocate_actor(&pid, fight_node, 4526, true);
    let defeat_msg = trigger_non_lethal_defeat(
        &doc,
        &mut delta,
        &clk,
        &pid,
        fight_node,
        "cave_lizard",
        "岩縫巨蜥",
    )
    .expect("defeat absorbs dropped rune stone");
    assert!(defeat_msg.contains("吞石異變的岩縫巨蜥"));
    assert!(
        delta
            .live_wilds
            .mutated_boss_by_node
            .contains_key(fight_node)
    );
    assert_eq!(
        u16::from(count_world_active_runes(&delta)) + delta.chaos_rune_pool,
        total_runes_before
    );

    // 養傷歸來踏回原處搜巡直接遭遇異變頭目並奪回原石
    if let Some(p) = delta.actors.get_mut(&pid) {
        p.wound_part = None;
        p.fatigue = 0;
        p.bound_weapon.rune_mark = Some("穩固鐵紋".to_string());
        p.mark_mutated();
    }
    delta.relocate_actor(&pid, fight_node, 4526, true);
    let exp_msg =
        apply_explore(&doc, &mut delta, &clk, &pid).expect("explore triggers mutated boss");
    assert!(exp_msg.contains("吞石異變的岩縫巨蜥"));
    let win_msg = resolve_skirmish_turn(&doc, &mut delta, &clk, &pid, "skirmish_heavy")
        .expect("slay mutated boss");
    assert!(win_msg.contains("撬回"));
    assert!(
        !delta
            .live_wilds
            .mutated_boss_by_node
            .contains_key(fight_node)
    );
    assert!(
        delta
            .actors
            .get(&pid)
            .unwrap()
            .backpack
            .iter()
            .any(|i| i.slug == "rune_stone" && i.count == 1)
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn passing_smith_transfers_shop_stock_promotes_apprentice_heirloom_house_and_obituary() {
    let (doc, mut delta) = fresh_world();
    let clk = solar_clock_for_settled_bell(WORLD_OPEN_DAY * 12 + 5, 1_700_000_000);
    let smith_id = "res:white_gull_harbor:shop_0_0";
    let app_id = "res:white_gull_harbor:shop_0_1";
    assert!(delta.actors.contains_key(smith_id) && delta.actors.contains_key(app_id));

    let vac_house = doc
        .households
        .iter()
        .find(|h| h.vacant_for_sale && h.node_id.contains("white_gull_harbor"))
        .map(|h| h.node_id.clone())
        .expect("white gull vacant house");
    delta
        .sold_houses
        .insert(vac_house.clone(), smith_id.to_string());
    delta.live_land.house_integrity.remove(&vac_house);

    let smith_copper_before;
    let forge_node;
    {
        let sm = delta.actors.get_mut(smith_id).unwrap();
        forge_node = sm.work_node.clone();
        smith_copper_before = sm.copper;
        sm.owned_house_node = Some(vac_house.clone());
        sm.bound_weapon = BoundTool {
            uid: "tool:bal_legacy".to_string(),
            slug: "smith_hammer".to_string(),
            name: "赤爐傳承鍛錘".to_string(),
            provenance: "鐵砧巴爾鍛打成名".to_string(),
            wear_stage: 0,
            grip_familiarity: 30,
            rune_mark: Some("穩固鐵紋".to_string()),
        };
        sm.mark_mutated();
    }
    let app_copper_before = delta.actors.get(app_id).unwrap().copper;
    if let Some(app) = delta.actors.get_mut(app_id) {
        app.owned_house_node = None;
        app.role = "鍛鐵匠學徒".to_string();
        app.work_node = forge_node.clone();
        app.mark_mutated();
    }

    let ob =
        execute_npc_passing_and_inheritance(&doc, &mut delta, &clk, smith_id, "於爐邊安詳壽終")
            .expect("smith passing produces obituary");
    assert_eq!(ob.deceased_id, smith_id);
    let chron_row = delta
        .living_chronicle
        .pending_outbox
        .last()
        .expect("passing writes living_chronicle row");
    let chron_fact: super::living_chronicler::ChronicleFactPacket =
        serde_json::from_str(&chron_row.fact_json).expect("valid fact json");
    super::living_chronicler::validate_polished_deed_prose(
        &chron_fact,
        "actor",
        &chron_row.draft_zh,
        &chron_row.prose_zh,
    )
    .expect("obituary prose must pass living_chronicler mechanical anti-hallucination gate");
    assert_eq!(
        ob.heir_name.as_deref(),
        Some(delta.actors.get(app_id).unwrap().display_name.as_str())
    );
    assert_eq!(ob.heirloom_zh.as_deref(), Some("赤爐傳承鍛錘"));

    let sm_after = delta.actors.get(smith_id).unwrap();
    assert!(!sm_after.alive);
    assert_eq!(sm_after.copper, 0);
    assert!(
        !delta
            .node_occupants
            .get(&sm_after.node_id)
            .is_some_and(|s| s.contains(smith_id))
    );

    let app_after = delta.actors.get(app_id).unwrap();
    assert_eq!(app_after.role, "鍛鐵匠");
    assert_eq!(app_after.copper, app_copper_before + smith_copper_before);
    assert_eq!(
        app_after.owned_house_node.as_deref(),
        Some(vac_house.as_str())
    );
    assert_eq!(app_after.bound_weapon.name, "赤爐傳承鍛錘");
    assert!(app_after.bound_weapon.provenance.contains("傳家遺物"));
    assert!(
        !app_after
            .inn_lockers
            .get(&forge_node)
            .is_none_or(Vec::is_empty),
        "smithy stock locker must transfer to promoted apprentice"
    );
    assert_eq!(
        delta.sold_houses.get(&vac_house).map(String::as_str),
        Some(app_id)
    );

    delta.relocate_actor(app_id, "oi:white_gull_harbor:plaza", 4616, true);
    let roll_txt = apply_world_action(&doc, &mut delta, &clk, app_id, "read_memorial_roll", "")
        .expect("read memorial roll at plaza");
    assert!(roll_txt.contains("赤爐傳承鍛錘"));
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn heirless_npc_passing_returns_runes_to_pool_releases_house_and_refunds_escrow() {
    let (doc, mut delta) = fresh_world();
    let clk = solar_clock_for_settled_bell(WORLD_OPEN_DAY * 12 + 5, 1_700_000_000);
    let loner_id = delta
        .actors
        .iter()
        .find(|(aid, a)| {
            a.alive
                && a.controller_id.is_none()
                && !aid.starts_with("pl:")
                && *aid != "res:white_gull_harbor:shop_0_0"
                && !a.role.contains("掌櫃")
        })
        .map(|(aid, _)| aid.clone())
        .expect("non-player actor exists");

    let vac_house = doc
        .households
        .iter()
        .find(|h| h.vacant_for_sale)
        .map(|h| h.node_id.clone())
        .expect("vacant house");
    delta
        .sold_houses
        .insert(vac_house.clone(), loner_id.clone());

    delta.chaos_rune_pool = delta.chaos_rune_pool.saturating_sub(1);
    let loner_copper;
    {
        let a = delta.actors.get_mut(&loner_id).unwrap();
        a.household_id = "hh:isolated:loner_test".to_string();
        a.work_node = "oi:cell4068:camp".to_string();
        a.mind.ltm.social.clear();
        a.owned_house_node = Some(vac_house.clone());
        a.bound_weapon.rune_mark = Some("穩固鐵紋".to_string());
        a.copper += 10;
        delta.municipal_treasury_copper -= 10;
        loner_copper = a.copper;
        a.mark_mutated();
    }

    // 託管契由孤身 NPC 承接 -> 死亡後必須視同不可抗力退押予委託人且不卡在 contract_escrow
    let issuer_id = "res:white_gull_harbor:shop_0_0";
    assert!(delta.transfer_copper(
        clk.game_days,
        clk.bell_index,
        "escrow_lock",
        issuer_id,
        "contract_escrow",
        6,
        "測死退押".to_string(),
    ));
    let issuer_after_lock = delta.actors.get(issuer_id).unwrap().copper;
    delta.contracts.insert(
        "pact:test_dead_deb".to_string(),
        ContractRecord {
            contract_id: "pact:test_dead_deb".to_string(),
            kind: "supply_commission".to_string(),
            creditor_id: issuer_id.to_string(),
            debtor_id: loner_id.clone(),
            amount_copper: 6,
            due_day: clk.game_days + 5,
            status: "in_progress".to_string(),
            note_zh: "iron_ore｜測死退押".to_string(),
        },
    );

    let pool_before = delta.chaos_rune_pool;
    let treasury_before = delta.municipal_treasury_copper;
    let ob = execute_npc_passing_and_inheritance(&doc, &mut delta, &clk, &loner_id, "老病辭世")
        .expect("heirless passing");
    assert!(ob.heir_name.is_none());
    assert_eq!(delta.chaos_rune_pool, (pool_before + 1).min(12));
    assert!(!delta.sold_houses.contains_key(&vac_house));
    assert_eq!(
        delta.municipal_treasury_copper,
        treasury_before + loner_copper
    );
    assert_eq!(
        delta.actors.get(issuer_id).unwrap().copper,
        issuer_after_lock + 6
    );
    assert_eq!(delta.contract_escrow_copper, 0);
    assert_eq!(delta.conservation_diff(&doc), 0);
}

#[test]
fn lifecycle_tick_ages_spawns_youth_from_household_copper_and_never_kills_player() {
    let (doc, mut delta) = fresh_world();
    let clk0 = solar_clock_for_settled_bell(WORLD_OPEN_DAY * 12 + 5, 1_700_000_000);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk0,
        "immortal_player",
        "長生旅人",
        "人類",
        "斗篷舊客",
        "forest_hunter",
    )
    .expect("setup player");
    let pid = delta.resolve_actor_id("immortal_player").unwrap();
    if let Some(p) = delta.actors.get_mut(&pid) {
        p.birth_year = 100; // 極端高齡玩家亦不可被自然壽終刪角
        p.severe_wrist_warning = true;
        p.mark_mutated();
    }

    // 挑出 1 名超高齡居民推進至下一個 7 日週期 -> 觸發壽終 + 新世代成年補血
    let target_elder = delta
        .actors
        .iter()
        .find(|(id, a)| a.alive && a.controller_id.is_none() && !id.starts_with("pl:"))
        .map(|(id, _)| id.clone())
        .unwrap();
    if let Some(e) = delta.actors.get_mut(&target_elder) {
        e.birth_year = 100;
        e.mark_mutated();
    }

    delta.live_lifecycle.last_lifecycle_day = WORLD_OPEN_DAY;
    let clk7 = solar_clock_for_settled_bell((WORLD_OPEN_DAY + 7) * 12 + 5, 1_700_000_000);
    assert!(actor_lived_years(delta.actors.get(&pid).unwrap(), clk7.game_days) > 100);
    tick_daily_lifecycle(&doc, &mut delta, &clk7);

    assert!(
        delta.actors.get(&pid).unwrap().alive,
        "player must never auto-die of age"
    );
    assert!(!delta.actors.get(&target_elder).unwrap().alive);
    assert_eq!(delta.live_lifecycle.spawned_youth_seq, 1);
    assert!(delta.actors.contains_key("res:gen_youth:1"));
    assert_eq!(delta.actors.get("res:gen_youth:1").unwrap().copper, 12);

    // 真人玩家重傷孤身落敗亦絕不強制刪角
    let msg = trigger_non_lethal_defeat(
        &doc,
        &mut delta,
        &clk7,
        &pid,
        "oi:cell4526:0",
        "grey_ear_goblin",
        "灰耳斥候",
    )
    .expect("player defeat stays non-lethal");
    assert!(msg.contains("昏厥倒地"));
    assert!(delta.actors.get(&pid).unwrap().alive);
    assert_eq!(delta.conservation_diff(&doc), 0);
}

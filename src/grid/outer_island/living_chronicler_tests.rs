//! 背景活史官（Living Chronicler）單元與回歸測試：
//! 驗證 `slow:*` 口號零洩漏、雙邊不對稱視角、上升沿鎖存器、M4 玩家「外來旅人」遮罩、機械防幻覺閘門、原子暫存目錄匯出與銅幣守恆。

use super::actions::apply_world_action;
use super::chronicle_export::{
    build_chronicle_actor_json, build_chronicle_article_json, build_chronicle_index_json,
    export_markdown_chronicles_staged_atomic,
};
use super::clock_and_lang::solar_clock_at;
use super::engine::load_static_world_doc;
use super::gazetteer::build_in_world_scroll_bundle_json;
use super::living_chronicler::{
    ChronicleFactPacket, num_zh, record_crisis_rescue, record_pact_default,
    record_primordial_spider_slain, validate_polished_deed_prose,
};
use super::living_chronicler_worker::apply_living_chronicler_v4_upgrade;
use super::movement::apply_setup_origin;
use super::sim_tick::seed_genesis_actors_once;
use super::types::IslandWorldDelta;
use super::world_weave_bio::refresh_actor_biography_if_needed;

fn seeded_world() -> (super::types::WorldStaticDoc, IslandWorldDelta) {
    let doc = load_static_world_doc();
    let mut delta = IslandWorldDelta {
        genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
        municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
        silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
        ..IslandWorldDelta::default()
    };
    seed_genesis_actors_once(&doc, &mut delta, false);
    assert_eq!(delta.conservation_diff(&doc), 0);
    (doc, delta)
}

#[test]
fn slow_think_slogans_never_pollute_biographies_and_v4_upgrade_cleans_existing() {
    let (doc, mut delta) = seeded_world();
    let npc_id = "res:white_gull_harbor:shop_3_0";
    {
        let a = delta.actors.get_mut(npc_id).expect("shop_3_0 exists");
        a.bio_core_zh =
            "止血黛安自幼在白鷗港草藥鋪守著石臼，長年替過往船工搗製苦艾敷藥。".to_string();
        a.bio_post_chapters_zh = vec![
            "太陽曆三二九年金秋，自家事自家做，誰也別來管".to_string(),
            "太陽曆三二九年金秋，手頭得留點底，先把眼前活幹穩".to_string(),
        ];
        a.personal_bio_zh = format!(
            "{}\n\n近歲新篇：{}。",
            a.bio_core_zh,
            a.bio_post_chapters_zh.join("；其後")
        );
        a.mind.push_turning_point(
            "slow:res:white_gull_harbor:shop_3_0:6500:2",
            6500,
            "自家事自家做，誰也別來管",
            "work_steady",
        );
    }
    let tp_len_before = delta.actors[npc_id].mind.ltm.turning_points.len();

    let cleaned = apply_living_chronicler_v4_upgrade(&doc, &mut delta);
    assert!(cleaned >= 1);

    let a_after = &delta.actors[npc_id];
    assert!(a_after.bio_post_chapters_zh.is_empty());
    assert!(!a_after.personal_bio_zh.contains("自家事自家做"));
    assert_eq!(
        a_after.mind.ltm.turning_points.len(),
        tp_len_before,
        "turning_points ring buffer must remain intact for AI decision continuity"
    );

    let mut a_mut = a_after.clone();
    let _ = refresh_actor_biography_if_needed(&doc, &mut a_mut, 6501);
    assert!(
        !a_mut.personal_bio_zh.contains("自家事自家做") && a_mut.bio_post_chapters_zh.is_empty(),
        "refresh_actor_biography_if_needed must never scan turning_points again"
    );
}

#[test]
fn player_buying_vacant_house_masks_name_and_updates_neighbor_region_and_annals() {
    let (doc, mut delta) = seeded_world();
    let clk = solar_clock_at(1_776_750_000);
    apply_setup_origin(
        &doc,
        &mut delta,
        &clk,
        "secret_buyer_77",
        "隱密買屋真名",
        "矮人",
        "肩寬手穩的旅人",
        "forest_hunter",
    )
    .expect("setup player");

    let house_id = "oi:white_gull_harbor:iron_anvil_ln:h10";
    let pl_id = "pl:secret_buyer_77";
    // 從公庫守恆撥發購屋款供測試買下鐵砧巷10號空屋
    assert!(delta.transfer_copper(
        clk.game_days,
        clk.bell_index,
        "test_grant",
        "municipal_treasury",
        pl_id,
        400,
        "測試購屋撥款".to_string(),
    ));
    delta.relocate_actor(pl_id, house_id, 4616, true);
    apply_world_action(
        &doc,
        &mut delta,
        &clk,
        "secret_buyer_77",
        "buy_vacant_house",
        "",
    )
    .expect("buy_vacant_house succeeds");
    assert_eq!(delta.conservation_diff(&doc), 0);

    // 1. 玩家本人絕不出現在公開 `/chronicle`
    assert!(build_chronicle_actor_json(&doc, &delta, pl_id).is_none());

    // 2. 隔壁關聯藥鋪（`res:white_gull_harbor:shop_3_0`）列傳自動長出空屋重燃炊煙章節，且嚴格顯示「外來旅人」
    let nb_json = build_chronicle_actor_json(&doc, &delta, "res:white_gull_harbor:shop_3_0")
        .expect("neighbor actor json")
        .to_string();
    assert!(
        nb_json.contains("外來旅人") && !nb_json.contains("隱密買屋真名"),
        "neighbor chronicle must mask player name: {nb_json}"
    );
    assert!(nb_json.contains("鐵砧巷十號"));

    // 3. 卷一《白鷗港風土志》與卷二《白鷗舊巷空戶重燃燈火記》及廣場石碑同步推進！
    let reg_json = build_chronicle_article_json(&doc, &delta, "reg:white_gull_harbor")
        .expect("region article")
        .to_string();
    assert!(reg_json.contains("外來旅人") && !reg_json.contains("隱密買屋真名"));

    let ann_json = build_chronicle_article_json(&doc, &delta, "ann:live:wgh_vacant_rekindled")
        .expect("live annals article")
        .to_string();
    assert!(ann_json.contains("白鷗舊巷空戶重燃燈火記") && !ann_json.contains("隱密買屋真名"));

    let stele_json = build_in_world_scroll_bundle_json(
        &doc,
        &delta,
        "white_gull_harbor",
        "stele",
        Some("ann:live:wgh_vacant_rekindled"),
    )
    .to_string();
    assert!(stele_json.contains("白鷗舊巷空戶重燃燈火記") && !stele_json.contains("隱密買屋真名"));
}

#[test]
fn bilateral_rescue_and_spider_milestone_preserve_perspectives_and_cooldowns() {
    let (doc, mut delta) = seeded_world();
    let clk = solar_clock_at(1_776_760_000);
    let rescuer = "res:greypine_post:shop_1_0";
    let rescued = "res:greypine_post:shop_2_0";
    let rescuer_name = delta.actors[rescuer].display_name.clone();
    let rescued_name = delta.actors[rescued].display_name.clone();

    record_crisis_rescue(
        &doc,
        &mut delta,
        &clk,
        rescuer,
        rescued,
        "oi:north_woods:broken_bridge",
        "重鍛鐵斧",
        "灰耳哥布林斥候",
    );
    // 同日重複觸發應受冷卻保護不洗版
    record_crisis_rescue(
        &doc,
        &mut delta,
        &clk,
        rescuer,
        rescued,
        "oi:north_woods:broken_bridge",
        "重鍛鐵斧",
        "灰耳哥布林斥候",
    );

    let r1_paras = delta.living_chronicle.actor_paragraphs(rescuer);
    let r2_paras = delta.living_chronicle.actor_paragraphs(rescued);
    assert_eq!(r1_paras.len(), 1);
    assert_eq!(r2_paras.len(), 1);
    assert!(r1_paras[0].contains(&format!("撞見{rescued_name}")));
    assert!(r2_paras[0].contains(&format!("幸得{rescuer_name}")));

    // 首殺吞石巨蛛觸發卷二新篇與灰松林驛內陸方輿更新（零濱海詞彙）
    record_primordial_spider_slain(
        &doc,
        &mut delta,
        &clk,
        rescuer,
        Some(rescued),
        "oi:north_woods:spider_pit",
        "重鍛鐵斧",
    );
    let idx = build_chronicle_index_json(&doc, &delta).to_string();
    assert!(idx.contains("ann:live:north_mine_spider_slain"));
    let spider_ann = build_chronicle_article_json(&doc, &delta, "ann:live:north_mine_spider_slain")
        .expect("spider annals")
        .to_string();
    assert!(spider_ann.contains("吞石巨蛛") && !spider_ann.contains("海灣"));

    // 同鎮累積 3 樁違約觸發城鎮銀根吃緊方輿段落（前 2 樁絕不提前誤觸！）
    for i in 0..2 {
        record_pact_default(
            &doc,
            &mut delta,
            &clk,
            &format!("ct_test_{i}"),
            rescuer,
            rescued,
            45,
        );
    }
    let gp_before_third = build_chronicle_article_json(&doc, &delta, "reg:greypine_post")
        .expect("greypine region")
        .to_string();
    assert!(
        !gp_before_third.contains("催討糾紛"),
        "2 defaults must not prematurely trigger >=3 pact_strain latch"
    );
    record_pact_default(&doc, &mut delta, &clk, "ct_test_2", rescuer, rescued, 45);
    let gp_reg = build_chronicle_article_json(&doc, &delta, "reg:greypine_post")
        .expect("greypine region")
        .to_string();
    assert!(gp_reg.contains("催討糾紛"));

    // 驗證原子暫存目錄置換匯出 `.md`
    let tmp_base = std::env::temp_dir().join(format!("sw_chronicle_test_{}", std::process::id()));
    let target_dir = tmp_base.join("銀鷗島人物列傳");
    let count = export_markdown_chronicles_staged_atomic(&doc, &delta, &target_dir)
        .expect("staged export succeeds");
    assert!(count >= 600);
    let ann_md =
        std::fs::read_to_string(target_dir.join("02_卷二紀事本末_銀鷗風變與蒼淵兵燹戰史.md"))
            .expect("read exported annals md");
    assert!(ann_md.contains("北林試煉深坑誅除吞石巨蛛本末"));
    let _ = std::fs::remove_dir_all(&tmp_base);
}

#[test]
fn mechanical_polisher_gatekeeper_blocks_quotes_number_drift_and_role_inversion() {
    assert_eq!(num_zh(360), "三百六十");
    assert_eq!(num_zh(120), "一百二十");
    assert_eq!(num_zh(12), "十二");
    assert_eq!(
        super::living_chronicler::normalize_chronicler_output(
            "\u{6751}\u{91cc}\u{4eba}\u{624b}\u{91cc}\u{6e0a}\u{9ccc}\u{54b8}\u{76d0}\u{90bb}\u{91cc}\u{7740}\u{4e34}\u{5374}\u{7ec7}\u{7eb9}\u{575f}"
        ),
        "村裡人手裡淵鰲鹹鹽鄰里著臨卻織紋墳"
    );

    let fact = ChronicleFactPacket {
        deed_id: "deed:rescue:6500:a:b".to_string(),
        kind: "crisis_rescue".to_string(),
        town_slug: "greypine_post".to_string(),
        town_zh: "灰松林驛".to_string(),
        place_zh: "灰松斷橋".to_string(),
        agent_public_label: "鐵砧巴克".to_string(),
        patient_public_label: "外來旅人".to_string(),
        forbidden_raw_names: vec!["葉卅".to_string()],
        ..ChronicleFactPacket::default()
    };
    let draft = "太陽曆三二九年金秋，時值晨鐘二響，山谷冷風掠過高處的松林梢頭。鐵砧巴克行經灰松斷橋時撞見外來旅人正遭灰耳哥布林斥候逼入險境，當即握緊【重鍛鐵斧】自側翼踏步切入戰圈，硬生生震偏敵勢並合擊逼退強敵，結下一樁過命的患難交情。";

    // 合格白話潤飾與「先寫受困現場、再寫施救者切入」之自然敘事皆應放行
    let valid_cand = "太陽曆三二九年金秋，晨鐘二響的山谷冷風吹過松林梢頭。鐵砧巴克走過灰松斷橋時，正好撞見外來旅人被灰耳哥布林斥候逼到絕處，便毫不遲疑地握緊手中【重鍛鐵斧】從側翼切入戰局，沉猛劈開敵勢並合力擊退強敵，自此結下過命交情。";
    assert!(validate_polished_deed_prose(&fact, "actor", draft, valid_cand).is_ok());
    let scene_first_cand = "太陽曆三二九年金秋，晨鐘二響的山谷冷風吹過松林梢頭。灰松斷橋邊外來旅人正遭灰耳哥布林斥候團團逼入險境，危急之際，鐵砧巴克握緊【重鍛鐵斧】從側翼切入戰圈震偏敵勢，兩人合力逼退強敵，自此結下深厚交情。";
    assert!(validate_polished_deed_prose(&fact, "actor", draft, scene_first_cand).is_ok());
    let truncated_cand = "太陽曆三二九年金秋，晨鐘二響的山谷冷風吹過松林梢頭。鐵砧巴克走過灰松斷橋時撞見外來旅人遭灰耳哥布林斥候逼入險境，當即握緊【重鍛鐵斧】從側翼切入戰圈震偏敵";
    assert_eq!(
        validate_polished_deed_prose(&fact, "actor", draft, truncated_cand),
        Err("truncated_incomplete_sentence")
    );

    // 1. 捏造對話引號必須攔截
    let bad_quote = "太陽曆三二九年金秋，晨鐘二響的山谷冷風掠過松林梢頭，鐵砧巴克在灰松斷橋大喊「快退後！」隨即揮動【重鍛鐵斧】救下外來旅人並擊退灰耳哥布林斥候，從此結下深厚交情。";
    assert_eq!(
        validate_polished_deed_prose(&fact, "actor", draft, bad_quote),
        Err("fabricated_quote_detected")
    );

    // 2. 竄改年份或數字必須攔截
    let bad_num = "太陽曆三三〇年金秋，晨鐘二響的山谷冷風吹過松林。鐵砧巴克在灰松斷橋撞見外來旅人遭灰耳哥布林斥候圍攻，立刻揮動【重鍛鐵斧】切入戰圈合力擊退強敵，結下患難交情。";
    assert_eq!(
        validate_polished_deed_prose(&fact, "actor", draft, bad_num),
        Err("altered_number_detected")
    );

    // 3. 主受詞順序顛倒（甲救乙變乙救甲）必須攔截
    let bad_order = "太陽曆三二九年金秋，晨鐘二響的山谷冷風吹過松林。外來旅人行經灰松斷橋時拔刀助鐵砧巴克擊退灰耳哥布林斥候，在灰松林驛留下一段佳話與深厚交情。";
    assert!(matches!(
        validate_polished_deed_prose(&fact, "actor", draft, bad_order),
        Err("inverted_rescue_direction" | "inverted_subject_object_order")
    ));

    // 4. 內陸灰松林驛混入濱海潮汐字眼必須攔截
    let bad_coastal = "太陽曆三二九年金秋，趁著海灣漲潮的海風，鐵砧巴克在灰松斷橋撞見外來旅人遭灰耳哥布林斥候逼入險境，當即揮動【重鍛鐵斧】切入戰圈擊退強敵，結下患難交情。";
    assert_eq!(
        validate_polished_deed_prose(&fact, "actor", draft, bad_coastal),
        Err("coastal_word_in_inland_greypine")
    );

    // 5. 洩漏玩家真名必須攔截
    let bad_leak = "太陽曆三二九年金秋，晨鐘二響的山谷冷風吹過松林。鐵砧巴克在灰松斷橋撞見外來旅人葉卅遭灰耳哥布林斥候圍困，立刻揮動【重鍛鐵斧】切入戰圈合力擊退強敵。";
    assert_eq!(
        validate_polished_deed_prose(&fact, "actor", draft, bad_leak),
        Err("player_real_name_leaked")
    );

    // 6. 受救者視角（`patient` 在前、`agent` 在後）中間夾雜地點副詞之施受反轉必須攔截，而合法受救視角與「鄰里」正體字應放行！
    let pat_fact = ChronicleFactPacket {
        deed_id: "deed:rescue:6500:a:b".to_string(),
        kind: "crisis_rescue_patient".to_string(),
        solar_year: 329,
        town_slug: "greypine_post".to_string(),
        town_zh: "灰松林驛".to_string(),
        place_zh: "灰松斷橋".to_string(),
        agent_public_label: "鐵砧巴克".to_string(),
        patient_public_label: "巴克".to_string(), // 特意測試子字串名稱不互撞！
        ..ChronicleFactPacket::default()
    };
    let pat_draft = "太陽曆三二九年金秋，時值晨鐘二響，山谷冷風掠過松林梢頭。巴克在灰松斷橋迎戰灰耳哥布林斥候時險象環生，危急關頭幸得鐵砧巴克揮動【重鍛鐵斧】自旁仗義切入合擊解圍，鄰里傳為美談。";
    let pat_valid = "太陽曆三二九年金秋，晨鐘二響的山風吹過松林。巴克在灰松斷橋迎戰灰耳哥布林斥候時陷入險境，危急時刻幸得鐵砧巴克揮動【重鍛鐵斧】從旁切入戰局合力解圍，鄰里間皆記著這樁恩情。";
    let pat_ok = validate_polished_deed_prose(&pat_fact, "actor", pat_draft, pat_valid)
        .expect("valid patient rescue prose");
    assert!(pat_ok.contains("鄰里"));

    let pat_inverted = "太陽曆三二九年金秋，晨鐘二響的山風吹過松林。巴克在灰松斷橋揮斧救下鐵砧巴克並擊退灰耳哥布林斥候，鄰里間皆記著這樁過命交情。";
    assert_eq!(
        validate_polished_deed_prose(&pat_fact, "actor", pat_draft, pat_inverted),
        Err("inverted_rescue_direction")
    );
    let pat_cross_clause_inv = "太陽曆三二九年金秋，晨鐘二響的山風吹過松林。巴克行經灰松斷橋，見鐵砧巴克遭灰耳哥布林斥候圍攻，當即揮動【重鍛鐵斧】切入戰圈擊退強敵。";
    assert_eq!(
        validate_polished_deed_prose(&pat_fact, "actor", pat_draft, pat_cross_clause_inv),
        Err("inverted_rescue_direction")
    );

    // 7. 驗證 `preserve_polished_projections` 不會因 `working` 新增草稿而遺失背景剛完成的 `polished` 文本
    let mut live_lc = super::living_chronicler::LivingChronicleState::default();
    live_lc.push_new_projection(super::living_chronicler::ChronicleProjectionRow {
        deed_id: "deed:1".to_string(),
        scope: "actor".to_string(),
        target_id: "res:white_gull_harbor:shop_3_0".to_string(),
        kind: "buy_vacant_house".to_string(),
        prose_zh: "舊草稿".to_string(),
        draft_zh: "舊草稿".to_string(),
        status: "draft".to_string(),
        game_day: 6500,
        ..Default::default()
    });
    let mut working_lc = live_lc.clone();
    working_lc.push_new_projection(super::living_chronicler::ChronicleProjectionRow {
        deed_id: "deed:2".to_string(),
        scope: "actor".to_string(),
        target_id: "res:white_gull_harbor:shop_3_0".to_string(),
        kind: "forge_vessel".to_string(),
        prose_zh: "新草稿".to_string(),
        draft_zh: "新草稿".to_string(),
        status: "draft".to_string(),
        game_day: 6501,
        ..Default::default()
    });
    live_lc.apply_row_in_memory(super::living_chronicler::ChronicleProjectionRow {
        deed_id: "deed:1".to_string(),
        scope: "actor".to_string(),
        target_id: "res:white_gull_harbor:shop_3_0".to_string(),
        kind: "buy_vacant_house".to_string(),
        prose_zh: "潤飾定稿".to_string(),
        draft_zh: "舊草稿".to_string(),
        status: "polished".to_string(),
        updated_seq: 1,
        game_day: 6500,
        ..Default::default()
    });
    super::living_chronicler::preserve_polished_projections(&live_lc, &mut working_lc);
    assert_eq!(
        working_lc.actor_paragraphs("res:white_gull_harbor:shop_3_0"),
        vec!["潤飾定稿".to_string(), "新草稿".to_string()]
    );
}

#[test]
fn consolidated_region_prose_and_simulation_guards_hold() {
    let (doc, mut delta) = seeded_world();
    let clk = (0..24)
        .map(|k| solar_clock_at(1_776_750_000 + k * 300))
        .find(|c| !c.is_night)
        .expect("daytime solar clock");

    // 1. 白鷗港酒館掌櫃（`shop_1_0`）即便揣著 800 銅幣站在灰松林驛待售空屋，也絕不跨鎮搶屋！
    let keeper_id = "res:white_gull_harbor:shop_1_0".to_string();
    let gp_house_1 = "oi:greypine_post:moss_well_ln:h9";
    let gp_house_2 = "oi:greypine_post:pine_resin_ln:h9";
    assert!(delta.transfer_copper(
        clk.game_days,
        clk.bell_index,
        "test_grant",
        "municipal_treasury",
        &keeper_id,
        800,
        "測試掌櫃積蓄".to_string(),
    ));
    delta.relocate_actor(&keeper_id, gp_house_1, 4525, true);
    let _ = super::unified_kernel::execute_player_parity_bell_actions(
        &doc,
        &mut delta,
        &clk,
        std::slice::from_ref(&keeper_id),
    );
    assert!(
        !delta.sold_houses.contains_key(gp_house_1),
        "shopkeeper must never snatch cross-town vacant house"
    );

    // 2. 灰松林驛本鎮成年非當家居民站在本鎮待售空屋可正常購屋分家，且連買兩戶＋首殺巨蛛僅維持單一當代段落！
    let gp_adult_1 = delta
        .actors
        .values()
        .find(|a| {
            a.household_id.starts_with("hh:greypine_post:")
                && !a.household_id.contains(":shop_")
                && !a.household_bond.contains("當家")
                && !a.household_bond.contains("屋主")
                && (329 - a.birth_year) >= super::world_weave_bio::racial_adult_age(&a.race)
        })
        .map(|a| a.actor_id.clone())
        .expect("greypine adult non-head exists");
    let gp_adult_2 = delta
        .actors
        .values()
        .find(|a| {
            a.actor_id != gp_adult_1
                && a.household_id.starts_with("hh:greypine_post:")
                && !a.household_id.contains(":shop_")
                && !a.household_bond.contains("當家")
                && !a.household_bond.contains("屋主")
                && (329 - a.birth_year) >= super::world_weave_bio::racial_adult_age(&a.race)
        })
        .map(|a| a.actor_id.clone())
        .expect("second greypine adult non-head exists");

    for (aid, h_nid) in [(&gp_adult_1, gp_house_1), (&gp_adult_2, gp_house_2)] {
        assert!(delta.transfer_copper(
            clk.game_days,
            clk.bell_index,
            "test_grant",
            "municipal_treasury",
            aid,
            400,
            "測試本鎮購屋".to_string(),
        ));
        delta.relocate_actor(aid, h_nid, 4525, true);
        let _ = super::unified_kernel::execute_player_parity_bell_actions(
            &doc,
            &mut delta,
            &clk,
            std::slice::from_ref(aid),
        );
        assert_eq!(
            delta.sold_houses.get(h_nid).map(String::as_str),
            Some(aid.as_str())
        );
    }
    delta.first_spider_slain = Some((
        "res:white_gull_harbor:seabreeze_ln_6_0".to_string(),
        "獸夾布蘭與風歌".to_string(),
        clk.game_days,
    ));
    record_primordial_spider_slain(
        &doc,
        &mut delta,
        &clk,
        "res:white_gull_harbor:seabreeze_ln_6_0",
        Some("res:gull_tail_port:kelp_ln_7_0"),
        "oi:dng_pit:lair",
        "硬木柄獵刀",
    );

    let gp_dyn_paras = delta
        .living_chronicle
        .region_paragraphs("reg:greypine_post");
    assert_eq!(
        gp_dyn_paras.len(),
        1,
        "reg:greypine_post must keep a single consolidated living paragraph: {gp_dyn_paras:?}"
    );
    assert!(gp_dyn_paras[0].contains("苔井巷九號") && gp_dyn_paras[0].contains("松脂巷九號"));
    assert!(
        !gp_dyn_paras[0].contains("空屋")
            && !gp_dyn_paras[0].contains("入太陽曆")
            && !gp_dyn_paras[0].contains("愈顯活絡")
            && !gp_dyn_paras[0].contains("莫不稱快")
    );
    let gp_full = build_chronicle_article_json(&doc, &delta, "reg:greypine_post")
        .expect("greypine article")
        .to_string();
    assert!(!gp_full.contains("漂木") && gp_full.contains("風倒枯木"));

    // 3. 普通無符文木杖（`rune_opt == None`）僅記錄生平首件出道作，後續普通木杖絕不洗版；嵌符文銘器則正常入傳！
    let crafter = "res:white_gull_harbor:shop_2_0";
    super::living_chronicler::record_vessel_forged(
        &doc,
        &mut delta,
        &clk,
        crafter,
        "oi:white_gull_harbor:iron_forge",
        "沉木護身手杖",
        None,
    );
    let clk_later = solar_clock_at(1_776_750_900 + 3600 * 10);
    super::living_chronicler::record_vessel_forged(
        &doc,
        &mut delta,
        &clk_later,
        crafter,
        "oi:white_gull_harbor:iron_forge",
        "沉木護身手杖",
        None,
    );
    assert_eq!(
        delta.living_chronicle.actor_paragraphs(crafter).len(),
        1,
        "ordinary wooden staffs after debut must not spam forge_vessel chapters"
    );
    let clk_master = solar_clock_at(1_776_750_900 + 3600 * 20);
    super::living_chronicler::record_vessel_forged(
        &doc,
        &mut delta,
        &clk_master,
        crafter,
        "oi:white_gull_harbor:iron_forge",
        "裂甲重鍛鐵斧",
        Some("吞石巨蛛厚甲殼"),
    );
    assert_eq!(
        delta.living_chronicle.actor_paragraphs(crafter).len(),
        2,
        "genuine carapace/rune masterwork must still be recorded"
    );
    assert_eq!(delta.conservation_diff(&doc), 0);
}

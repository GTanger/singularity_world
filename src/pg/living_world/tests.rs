#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_detects_the_all_gather_pathology() {
        let audit = LivingWorldAudit {
            character_count: 2,
            npc_count: 2,
            continuity_count: 2,
            autonomous_count: 2,
            obligation_count: 0,
            relationship_count: 0,
            knowledge_count: 0,
            action_receipt_count: 0,
            source_record_count: 0,
            distinct_npc_activities: 1,
            gathering_npc_count: 2,
            trade_count: 0,
            trade_receipt_count: 0,
            unbalanced_trade_count: 0,
            work_count: 0,
            work_receipt_count: 0,
            unbalanced_work_count: 0,
        };
        assert!(audit.all_npcs_gathering());
    }

    #[test]
    fn breath_habit_has_inertia_and_vessel_grindstone_respects_rolled_edge() {
        BREATH_HABIT_CACHE.write().unwrap().remove("test_ent_habit");
        VESSEL_STATE_CACHE.write().unwrap().remove(&("test_ent_vessel".to_string(), "wt_boning_knife".to_string()));
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM character_breath_habit WHERE entity_id = 'test_ent_habit'", &[]);
            let _ = client.execute("DELETE FROM character_vessel_mastery WHERE entity_id = 'test_ent_vessel'", &[]);
        }
        let h0 = record_breath_activity("test_ent_habit", BreathHabitKind::ForestLong, 5, 100);
        assert_eq!(h0.dominant, BreathHabitKind::ForestLong);
        // Small activity in another habit should not immediately flip dominant habit (inertia)
        let h1 = record_breath_activity("test_ent_habit", BreathHabitKind::ForgeHeavy, 3, 101);
        assert_eq!(h1.dominant, BreathHabitKind::ForestLong);
        // Sustained forge work shifts the breath habit
        let h2 = record_breath_activity("test_ent_habit", BreathHabitKind::ForgeHeavy, 10, 102);
        assert_eq!(h2.dominant, BreathHabitKind::ForgeHeavy);

        // Vessel state progression & grindstone vs forge repair
        let v1 = record_vessel_use("test_ent_vessel", "wt_boning_knife", 4, 3, 100);
        assert_eq!(v1.condition, VesselCondition::SlightlyDull);
        assert_eq!(v1.familiarity(), VesselFamiliarity::GettingSmooth);
        let sharpened = sharpen_vessel_at_grindstone("test_ent_vessel", "wt_boning_knife", 105).unwrap();
        assert_eq!(sharpened.condition, VesselCondition::Sharp);
        let initial_use_count = sharpened.use_count;
        // Grindstone on an already sharp and unworn blade does NOT increment use_count (F8)
        let sharpened_again = sharpen_vessel_at_grindstone("test_ent_vessel", "wt_boning_knife", 106).unwrap();
        assert_eq!(sharpened_again.use_count, initial_use_count);
        let v_rolled = record_vessel_use("test_ent_vessel", "wt_boning_knife", 9, 5, 110);
        assert_eq!(v_rolled.condition, VesselCondition::RolledEdge);
        assert!(sharpen_vessel_at_grindstone("test_ent_vessel", "wt_boning_knife", 115).is_err());
        let tempered = temper_vessel_at_forge("test_ent_vessel", "wt_boning_knife", 120);
        assert_eq!(tempered.condition, VesselCondition::Sharp);
        assert_eq!(tempered.familiarity(), VesselFamiliarity::WellBalanced);
    }

    #[test]
    fn end_to_end_organic_living_world_nine_systems_loop() {
        let pid = "player_e2e_hunter_living_loop";
        let now = 1_760_000_000i64;
        BREATH_HABIT_CACHE.write().unwrap().remove(pid);
        VESSEL_STATE_CACHE.write().unwrap().remove(&(pid.to_string(), "craft_steel_hunting_knife".to_string()));
        LINEAGE_IMPRINT_CACHE.write().unwrap().remove(pid);
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM character_breath_habit WHERE entity_id = $1", &[&pid]);
            let _ = client.execute("DELETE FROM character_vessel_mastery WHERE entity_id = $1", &[&pid]);
            let _ = client.execute("DELETE FROM character_lineage_imprints WHERE entity_id = $1", &[&pid]);
        }

        // 1. 部位傷勢與分級療傷
        record_entity_affliction(pid, AfflictionKind::LegSprain, "cliff_fall", now);
        record_entity_affliction(pid, AfflictionKind::Miasma, "swamp_fog", now);
        assert_eq!(get_entity_afflictions(pid).len(), 2);
        let _ = clear_entity_afflictions(pid, false, now + 5); // 一般止血散不拔除瘴毒
        assert_eq!(get_entity_afflictions(pid), vec![AfflictionKind::Miasma]);
        let _ = clear_entity_afflictions(pid, true, now + 8); // 診所／草藥拔除瘴毒
        assert!(get_entity_afflictions(pid).is_empty());

        // 2. 氣機自動繞路與動中體察（零數字、零 G0 代碼）
        let somatic = crate::combat::wordplate::somatic_meridian_awareness(42_4242);
        assert!(!somatic.is_empty());
        assert!(!somatic.chars().any(|c| c.is_ascii_digit()));
        assert!(!somatic.contains("G0"));

        // 3. 內息底色養成與慣性
        let h_init = get_or_derive_breath_habit(pid, "山林獵戶", 42_4242);
        assert_eq!(h_init.dominant, BreathHabitKind::ForestLong);
        let h_minor = record_breath_activity(pid, BreathHabitKind::ForgeHeavy, 2, now + 10);
        assert_eq!(h_minor.dominant, BreathHabitKind::ForestLong);
        let h_shifted = record_breath_activity(pid, BreathHabitKind::ForgeHeavy, 8, now + 20);
        assert_eq!(h_shifted.dominant, BreathHabitKind::ForgeHeavy);

        // 4. 人器磨合 × 鋒刃器況與街邊磨刀槽 vs 鍛爐回火
        let v_used = record_vessel_use(pid, "craft_steel_hunting_knife", 4, 4, now + 30);
        assert_eq!(v_used.condition, VesselCondition::SlightlyDull);
        assert_eq!(v_used.familiarity(), VesselFamiliarity::GettingSmooth);
        let v_honed = sharpen_vessel_at_grindstone(pid, "craft_steel_hunting_knife", now + 40).unwrap();
        assert_eq!(v_honed.condition, VesselCondition::Sharp);
        let use_after_hone = v_honed.use_count;
        // 對已完好之刀重複推磨不無限刷磨合度
        let v_hone_again = sharpen_vessel_at_grindstone(pid, "craft_steel_hunting_knife", now + 45).unwrap();
        assert_eq!(v_hone_again.use_count, use_after_hone);

        // 5. 師承烙印與親筆傳世手札觸發
        let (imp1, gifted1) = record_mentor_imprint_progress(
            pid,
            "岑鐵匠",
            "岑氏沉腕落錘法",
            "沉腕蓄勁、順紋落錘",
            now + 50,
        );
        assert_eq!(imp1.interaction_count, 1);
        assert!(!gifted1);
        let (imp2, gifted2) = record_mentor_imprint_progress(
            pid,
            "岑鐵匠",
            "岑氏沉腕落錘法",
            "沉腕蓄勁、順紋落錘",
            now + 60,
        );
        assert_eq!(imp2.interaction_count, 2);
        assert!(gifted2);

        // 6. 百工高光分層 LLM 資料轉描述與事實保真護欄
        let craft_pkt = crate::ai::CraftActionFactPacket {
            actor_name: "林山行".to_string(),
            tool_name: "鍛鐵重錘".to_string(),
            tool_familiarity: "沉穩趁手".to_string(),
            breath_habit: "鍛爐沉息".to_string(),
            material_names: vec!["赤鐵礦".to_string(), "硬木炭".to_string()],
            output_name: "精鍛防身獵刀".to_string(),
            highlight_kind: "工坊器物異變".to_string(),
            environment_hint: "文曜分家巷鍛爐火光熾烈".to_string(),
        };
        let craft_prose = crate::ai::compose_craft_highlight_from_data(&craft_pkt);
        assert!(crate::ai::verify_craft_output(&craft_prose, &craft_pkt).is_ok());
    }

    #[test]
    fn flagship_specimen_1_qin_shuisheng_eight_dimensions_coherent() {
        let now = 1_793_632_000i64;
        {
            let mut guard = VESSEL_STATE_CACHE.write().unwrap();
            let s = guard.entry(("秦水生".to_string(), "wt_cat_0973".to_string())).or_insert_with(|| VesselStateRecord {
                entity_id: "秦水生".to_string(),
                item_id: "wt_cat_0973".to_string(),
                use_count: 68,
                wear_points: 0,
                condition: VesselCondition::Sharp,
                updated_at: now,
            });
            s.wear_points = 0;
            s.condition = VesselCondition::Sharp;
            let c = guard.entry(("秦水生".to_string(), "wt_cat_0984".to_string())).or_insert_with(|| VesselStateRecord {
                entity_id: "秦水生".to_string(),
                item_id: "wt_cat_0984".to_string(),
                use_count: 42,
                wear_points: 0,
                condition: VesselCondition::Sharp,
                updated_at: now,
            });
            c.wear_points = 0;
            c.condition = VesselCondition::Sharp;
        }
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute(
                "UPDATE character_vessel_mastery SET wear_points = 0, condition_code = 'sharp'
                 WHERE entity_id = '秦水生' AND item_id IN ('wt_cat_0973', 'wt_cat_0984')",
                &[],
            );
            let _ = client.execute(
                "UPDATE resident_obligations SET status = 'active'
                 WHERE obligation_id = 'ob_qin_shuisheng_morning_ricemilk_wenyao'",
                &[],
            );
        }
        seed_flagship_specimen_qin_shuisheng_once(now);

        let manuals = get_entity_studied_manuals("秦水生");
        assert!(manuals.iter().any(|m| m == "堰渠水輪分動考工譜"));
        assert!(manuals.iter().any(|m| m == "石磨齒距微調與水磨澄漿手札"));

        let v_sieve = get_vessel_state("秦水生", "wt_cat_0973");
        assert!(v_sieve.use_count >= 68);
        assert_eq!(v_sieve.condition, VesselCondition::Sharp);
        assert_eq!(v_sieve.familiarity(), VesselFamiliarity::LikeOwnArm);

        // 驗證羊角鐵錘打石手鏨（wt_cat_0984）真正走玩家同構磨損與活水磨石推磨修復閉環（而非整點鎖血歸零）
        let v_chisel_0 = get_vessel_state("秦水生", "wt_cat_0984");
        assert!(v_chisel_0.use_count >= 42);
        let v_worn = record_vessel_use("秦水生", "wt_cat_0984", 3, 3, now + 10);
        assert!(v_worn.wear_points >= 3);
        assert_eq!(v_worn.condition, VesselCondition::SlightlyDull);
        // 再次呼叫 seed_flagship_specimen_qin_shuisheng_once 絕不會把 wear_points 洗回 0
        seed_flagship_specimen_qin_shuisheng_once(now + 20);
        assert!(get_vessel_state("秦水生", "wt_cat_0984").wear_points >= 3);
        // 透過玩家同構之 sharpen_vessel_at_grindstone 推磨後，wear_points 歸零、重回 Sharp
        let v_honed = sharpen_vessel_at_grindstone("秦水生", "wt_cat_0984", now + 30).unwrap();
        assert_eq!(v_honed.wear_points, 0);
        assert_eq!(v_honed.condition, VesselCondition::Sharp);

        let fam_total = get_entity_anatomy_familiarity_total("秦水生");
        assert!(fam_total >= 64);

        let equip_json = r#"{"head":"wt_cat_0875","body":"wt_cat_0869","legs":"starter_legs_m","feet":"starter_feet_m","wrist":"wt_cat_0852","waist":"wt_cat_0897","hold_r":"wt_cat_0984","hold_l":"wt_cat_0973"}"#;
        assert!(!crate::db::is_naked(equip_json));
        let names = crate::db::get_item_names(equip_json);
        assert_eq!(names.get("hold_r").map(String::as_str), Some("羊角鐵錘打石手鏨"));
        assert_eq!(names.get("hold_l").map(String::as_str), Some("平底石磨分粉篩架"));
        assert_eq!(names.get("waist").map(String::as_str), Some("薄層重礦烏木膽隨身保溫瓶"));
        assert_eq!(names.get("body").map(String::as_str), Some("粗麻雙帶圍胸短褂"));

        // 驗證 Store 層四問決策閉環：耗糧碾磨 -> 累積手鏨磨損 -> 自動推磨修復 -> 斷糧向周萬生守恆買米
        let mut store = crate::store::Store::new_in_memory();
        let mut qin_ent = crate::store::Entity::new_npc("秦水生", "水");
        qin_ent.display_title = "老堰頭水輪石磨首座|秦水生".to_string();
        qin_ent.grid_x = Some(-3);
        qin_ent.grid_y = Some(125);
        qin_ent.vit = 90;
        qin_ent.qi = 90;
        qin_ent.magnesium = 50;
        qin_ent.inventory = r#"[{"item_id":"wt_cat_0388","qty":4},{"item_id":"wt_cat_0279","qty":4},{"item_id":"wt-good-009","qty":2}]"#.to_string();
        let _ = store.upsert_entity_full(qin_ent);

        let mut zhou_ent = crate::store::Entity::new_npc("周萬生", "周");
        zhou_ent.magnesium = 30;
        zhou_ent.inventory = r#"[{"item_id":"wt-good-009","qty":12}]"#.to_string();
        let _ = store.upsert_entity_full(zhou_ent);

        // Step A：耗 1 袋精碾白米（2 -> 1），產出溫米漿（4 -> 5）與在來米粉（4 -> 5），手鏨 wear_points 0 -> 1
        let (x1, y1, act1) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 10, 1_800_000_000);
        assert_eq!((x1, y1), (-3, 125));
        assert!(act1.contains("大石磨"));
        let qin_after_1 = store.get_entity("秦水生").unwrap();
        assert_eq!(inv_count_item(&qin_after_1.inventory, "wt-good-009"), 1);
        assert_eq!(inv_count_item(&qin_after_1.inventory, "wt_cat_0388"), 5);
        assert_eq!(get_vessel_state("秦水生", "wt_cat_0984").wear_points, 1);

        // Step B：因為 wt-good-009 只剩 1 袋（< 2），下一輪自動觸發斷糧打斷，向周萬生支付 12 Mg 補入 6 袋新米（1 -> 7）
        let (_, _, act2) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 11, 1_800_000_060);
        assert!(act2.contains("周萬生"));
        let qin_after_2 = store.get_entity("秦水生").unwrap();
        let zhou_after_2 = store.get_entity("周萬生").unwrap();
        assert_eq!(inv_count_item(&zhou_after_2.inventory, "wt-good-009"), 6);
        assert_eq!(inv_count_item(&qin_after_2.inventory, "wt-good-009"), 7);
        assert_eq!(qin_after_2.magnesium, 38);
        assert_eq!(zhou_after_2.magnesium, 42);
        assert_eq!(qin_after_2.magnesium + zhou_after_2.magnesium, 80); // 雙向鎂幣守恆

        // Step C：再連續磨米兩次讓手鏨 wear_points 達到 3（SlightlyDull），下一步自動打斷磨米、在活水磨石推磨歸零
        let _ = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 12, 1_800_000_120);
        let _ = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 13, 1_800_000_180);
        assert_eq!(get_vessel_state("秦水生", "wt_cat_0984").wear_points, 3);
        let (_, _, act_hone) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 14, 1_800_000_240);
        assert!(act_hone.contains("活水磨石"));
        assert_eq!(get_vessel_state("秦水生", "wt_cat_0984").wear_points, 0);
        assert_eq!(get_vessel_state("秦水生", "wt_cat_0984").condition, VesselCondition::Sharp);

        // Step D：驗證送交早市米漿（hour = 6）與陳萬福交易。
        // 先在遠處時應為「沿山道前往」，不瞬移亦不交割；抵達 (-4, 4) 後才實物交割與鎂幣轉移
        let mut chen_ent = crate::store::Entity::new_npc("陳萬福", "陳");
        chen_ent.magnesium = 20;
        let _ = store.upsert_entity_full(chen_ent);
        // 遠處：目前位於 (-3, 125)，往 (-4, 4) 移動一步
        let (x_far, y_far, act_far) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 6, 1_800_000_300);
        assert_eq!((x_far, y_far), (-4, 4));
        let qin_far = store.get_entity("秦水生").unwrap();
        assert_eq!((qin_far.grid_x, qin_far.grid_y), (Some(-4), Some(124))); // step 1 cell
        assert!(act_far.contains("沿山道前往"));
        // 手動將秦水生置於 (-4, 4) 模擬抵達大食堂
        let _ = store.update_entity("秦水生", |e| {
            e.grid_x = Some(-4);
            e.grid_y = Some(4);
        });
        let (_, _, act_near) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 6, 1_800_000_360);
        assert!(act_near.contains("文曜三水大食堂"));
        let qin_delivered = store.get_entity("秦水生").unwrap();
        let chen_received = store.get_entity("陳萬福").unwrap();
        assert_eq!(inv_count_item(&chen_received.inventory, "wt_cat_0388"), 2);
        assert_eq!(inv_count_item(&chen_received.inventory, "wt_cat_0279"), 1);
        assert_eq!(chen_received.magnesium, 4); // 20 - 16 = 4
        assert_eq!(qin_delivered.magnesium, 38 + 16); // 38 + 16 = 54

        // 驗證當秦水生仍在文曜三水大食堂（-4, 4），且進入日間常規磨米時段（hour = 10，目標為老堰頭 -3, 125）時，
        // 絕不在文曜遠端隔空耗糧碾磨，而是處於「正沿山道返回老堰頭水輪石磨工坊」路途步態，待回到工坊後才執行實體磨務
        let qin_before_return = store.get_entity("秦水生").unwrap();
        let grain_before = inv_count_item(&qin_before_return.inventory, "wt-good-009");
        let ricemilk_before = inv_count_item(&qin_before_return.inventory, "wt_cat_0388");
        let (x_ret, y_ret, act_ret) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 10, 1_800_000_420);
        assert_eq!((x_ret, y_ret), (-3, 125));
        assert!(act_ret.contains("返回老堰頭水輪石磨工坊"));
        let qin_walking = store.get_entity("秦水生").unwrap();
        assert_eq!(inv_count_item(&qin_walking.inventory, "wt-good-009"), grain_before);
        assert_eq!(inv_count_item(&qin_walking.inventory, "wt_cat_0388"), ricemilk_before);
        // Step E：驗證「萬人萬相・同境不同選」——在完全相同的外部處境（wear_points == 3 且早市交貨時間已到 hour == 6）下，
        // 依不同 soul_seed 與所熟歷練，有人選擇「先磨利手鏨再出門（RepairTool）」，有人選擇「先趕送早市米漿履約（DeliverContract）」，非死板單一機器人。
        let mut saw_repair = false;
        let mut saw_deliver = false;
        for candidate_seed in 1u64..120u64 {
            let choice = choose_resident_life_action(
                "老堰頭水輪石磨首座",
                "老堰頭",
                candidate_seed.wrapping_mul(0x9E37_79B9_7F4A_7C15),
                85,
                30,
                4,
                4,
                3,
                true,
                6,
                crate::game::ZoneWeatherPhase::ClearBreeze,
                crate::game::ZoneTidePhase::SlackTide,
                68,
                &[],
            );
            if choice == ResidentLifeActionChoice::RepairTool {
                saw_repair = true;
            } else if choice == ResidentLifeActionChoice::DeliverContract {
                saw_deliver = true;
            }
        }
        assert!(
            saw_repair && saw_deliver,
            "Identical situation (wear=3, morning delivery due) must yield both RepairTool and DeliverContract across different soul_seeds"
        );
    }

    #[test]
    fn resident_life_loop_is_shared_and_factor_driven_across_vocations() {
        let now = 1_800_100_000i64;
        let smith_id = "岑守岳_test_shared_loop";
        {
            let mut guard = VESSEL_STATE_CACHE.write().unwrap();
            guard.insert(
                (smith_id.to_string(), "wt_blacksmith_hammer".to_string()),
                VesselStateRecord {
                    entity_id: smith_id.to_string(),
                    item_id: "wt_blacksmith_hammer".to_string(),
                    use_count: 55,
                    wear_points: 0,
                    condition: VesselCondition::Sharp,
                    updated_at: now,
                },
            );
        }

        let mut store = crate::store::Store::new_in_memory();
        let mut smith_ent = crate::store::Entity::new_npc(smith_id, "岑");
        smith_ent.display_title = "文曜鐵坊掌錘|岑守岳".to_string();
        smith_ent.grid_x = Some(6);
        smith_ent.grid_y = Some(4);
        smith_ent.vit = 90;
        smith_ent.qi = 90;
        smith_ent.magnesium = 40;
        smith_ent.equipment_slots = r#"{"hold_r":"wt_blacksmith_hammer","body":"starter_chest_m"}"#.to_string();
        smith_ent.inventory = r#"[{"item_id":"wt_cat_0387","qty":4},{"item_id":"wt-good-019","qty":2}]"#.to_string();
        let _ = store.upsert_entity_full(smith_ent);

        let mut zhou_ent = crate::store::Entity::new_npc("周萬生", "周");
        zhou_ent.magnesium = 30;
        zhou_ent.inventory = r#"[{"item_id":"wt-good-019","qty":10}]"#.to_string();
        let _ = store.upsert_entity_full(zhou_ent);

        let mut chen_ent = crate::store::Entity::new_npc("陳萬福", "陳");
        chen_ent.magnesium = 25;
        let _ = store.upsert_entity_full(chen_ent);

        // 1. Work (MillGrain choice): 消耗 1 簍赤鐵粗礦（2 -> 1），產出百鍛農具鐵件（4 -> 5），鐵錘磨損 0 -> 1
        let (wx, wy, w_act) = step_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            10,
            now,
        );
        assert_eq!((wx, wy), (6, 4));
        assert!(w_act.contains("百鍛鐵工坊"));
        let smith_after_work = store.get_entity(smith_id).unwrap();
        assert_eq!(inv_count_item(&smith_after_work.inventory, "wt-good-019"), 1);
        assert_eq!(inv_count_item(&smith_after_work.inventory, "wt_cat_0387"), 5);
        assert_eq!(get_vessel_state(smith_id, "wt_blacksmith_hammer").wear_points, 1);

        // 2. Restock: 因為赤鐵粗礦只剩 1 簍（< 2），下一輪自動向周萬生支付 12 Mg 補入 6 簍赤鐵粗礦（1 -> 7）
        let (_, _, r_act) = step_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            11,
            now + 60,
        );
        assert!(r_act.contains("周萬生") && r_act.contains("赤鐵粗礦"));
        let smith_after_restock = store.get_entity(smith_id).unwrap();
        let zhou_after_restock = store.get_entity("周萬生").unwrap();
        assert_eq!(inv_count_item(&smith_after_restock.inventory, "wt-good-019"), 7);
        assert_eq!(inv_count_item(&zhou_after_restock.inventory, "wt-good-019"), 4);
        assert_eq!(smith_after_restock.magnesium, 28);
        assert_eq!(zhou_after_restock.magnesium, 42);

        // 3. RepairTool: 再鍛打兩次使鐵錘磨損達 3，下一輪自動在活水磨石台推磨歸零
        let _ = step_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            12,
            now + 120,
        );
        let _ = step_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            13,
            now + 180,
        );
        assert_eq!(get_vessel_state(smith_id, "wt_blacksmith_hammer").wear_points, 3);
        let (_, _, h_act) = step_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            14,
            now + 240,
        );
        assert!(h_act.contains("活水磨石"));
        assert_eq!(get_vessel_state(smith_id, "wt_blacksmith_hammer").wear_points, 0);

        // 4. DeliverContract: 早市（hour = 6）先沿途移動（不隔空交割），抵達 (-4, 4) 後將 2 件百鍛農具鐵件交割予陳萬福並收受 16 Mg
        let (_, _, d_far_act) = step_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            6,
            now + 300,
        );
        assert!(d_far_act.contains("沿山道前往"));
        let _ = store.update_entity(smith_id, |e| {
            e.grid_x = Some(-4);
            e.grid_y = Some(4);
        });
        let (_, _, d_near_act) = step_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            6,
            now + 360,
        );
        assert!(d_near_act.contains("文曜三水大食堂") && d_near_act.contains("百鍛農具鐵件"));
        let smith_delivered = store.get_entity(smith_id).unwrap();
        let chen_received = store.get_entity("陳萬福").unwrap();
        assert_eq!(inv_count_item(&chen_received.inventory, "wt_cat_0387"), 2);
        assert_eq!(chen_received.magnesium, 9); // 25 - 16
        assert_eq!(smith_delivered.magnesium, 28 + 16); // 44
    }

    #[test]
    fn restock_grain_steps_toward_supplier_when_supplier_has_grid_coord() {
        let mut store = crate::store::Store::new_in_memory();
        let smith_id = "岑守岳_test_restock_step";
        let now = 1_800_000_000i64;

        let mut smith_ent = crate::store::Entity::new_npc(smith_id, "岑");
        smith_ent.display_title = "文曜百鍛鐵匠|岑守岳".to_string();
        smith_ent.grid_x = Some(6);
        smith_ent.grid_y = Some(4);
        smith_ent.vit = 80;
        smith_ent.magnesium = 40;
        smith_ent.equipment_slots = r#"{"hold_r":"wt_blacksmith_hammer","body":"starter_chest_m"}"#.to_string();
        smith_ent.inventory = r#"[{"item_id":"wt_cat_0387","qty":4},{"item_id":"wt-good-019","qty":1}]"#.to_string();
        let _ = store.upsert_entity_full(smith_ent);

        let mut zhou_ent = crate::store::Entity::new_npc("周萬生", "周");
        zhou_ent.grid_x = Some(-2);
        zhou_ent.grid_y = Some(-4);
        zhou_ent.magnesium = 30;
        zhou_ent.inventory = r#"[{"item_id":"wt-good-019","qty":10}]"#.to_string();
        let _ = store.upsert_entity_full(zhou_ent);

        // 1. 原料僅剩 1 簍（< 2）且周萬生具有實體座標 (-2, -4) 時，
        // 觸發 RestockGrain，應沿途移動走向 (-2, -4)，在途嚴禁隔空扣款提料
        let (tx, ty, transit_act) = step_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            11,
            now,
        );
        assert_eq!((tx, ty), (-2, -4));
        assert!(transit_act.contains("步向大圳頭周萬生處商議補進赤鐵粗礦"));

        // 驗證沿途移動中嚴禁空中扣減鎂幣或預先入庫
        let smith_mid = store.get_entity(smith_id).unwrap();
        let zhou_mid = store.get_entity("周萬生").unwrap();
        assert_eq!(inv_count_item(&smith_mid.inventory, "wt-good-019"), 1);
        assert_eq!(inv_count_item(&zhou_mid.inventory, "wt-good-019"), 10);
        assert_eq!(smith_mid.magnesium, 40);
        assert_eq!(zhou_mid.magnesium, 30);

        // 2. 抵達周萬生座標 (-2, -4) 後，下一輪執行現場交割與雙向鎂幣轉移
        let _ = store.update_entity(smith_id, |e| {
            e.grid_x = Some(-2);
            e.grid_y = Some(-4);
        });
        let (_, _, onsite_act) = step_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            11,
            now + 60,
        );
        assert!(onsite_act.contains("周萬生") && onsite_act.contains("赤鐵粗礦"));

        let smith_final = store.get_entity(smith_id).unwrap();
        let zhou_final = store.get_entity("周萬生").unwrap();
        assert_eq!(inv_count_item(&smith_final.inventory, "wt-good-019"), 7);
        assert_eq!(inv_count_item(&zhou_final.inventory, "wt-good-019"), 4);
        assert_eq!(smith_final.magnesium, 28);
        assert_eq!(zhou_final.magnesium, 42);
        assert_eq!(smith_final.magnesium + zhou_final.magnesium, 70);
    }

    #[test]
    fn autonomous_choice_diverges_by_personality_and_triggers_dyadic_events() {
        let now = 1_800_300_000i64;
        // 1. 同境不同選：當手頭器物磨損 wear_points == 3 且恰逢早市契約（hour = 6, ricemilk_ob_open = true）：
        //    - 高匠心／守序角色（如秦水生 seed）拒絕帶著鈍鏨出工，選擇先 RepairTool；
        //    - 重信諾且累積深厚信任鉤子的角色，選擇先 DeliverContract 履約，回來再磨刀。
        let craft_choice = choose_resident_life_action(
            "老堰頭水輪石磨首座|秦水生",
            "老堰頭",
            4_088_606_620_229_995_941,
            85,
            30,
            6,
            6,
            3,
            true,
            6,
            crate::game::ZoneWeatherPhase::ClearBreeze,
            crate::game::ZoneTidePhase::SlackTide,
            120,
            &[],
        );
        assert_eq!(craft_choice, ResidentLifeActionChoice::RepairTool);

        let trust_hooks = vec![crate::npc::causal_core::RelationalHook {
            target_id: "陳萬福".to_string(),
            target_name: "陳萬福".to_string(),
            kind_label: "守諾交割".to_string(),
            summary: "長年守信交割".to_string(),
            intensity: 45,
            updated_at_unix: now,
        }];
        let honor_choice = choose_resident_life_action(
            "古道行商|馬山行",
            "文曜",
            7,
            85,
            4,
            6,
            6,
            3,
            true,
            6,
            crate::game::ZoneWeatherPhase::ClearBreeze,
            crate::game::ZoneTidePhase::SlackTide,
            5,
            &trust_hooks,
        );
        assert_eq!(honor_choice, ResidentLifeActionChoice::DeliverContract);

        // 2. 自主尋商（傲骨拒向仇家買料）＋ 甲的行為變成乙的事件：
        //    鐵匠缺赤鐵粗礦（qty = 1），場上有兩個擁有赤鐵粗礦的賣家：
        //    - 「惡霸礦頭」（距離近，但有嫌隙結怨鉤子）
        //    - 「石萬鈞」（有信任關係）
        //    鐵匠自主避開仇家，選擇向石萬鈞採買；採買完成後，石萬鈞身上立即產生對應交易思緒與關係鉤子。
        let smith_id = "岑守岳_test_autonomy";
        crate::npc::causal_core::upsert_relational_hook(
            smith_id,
            "惡霸礦頭",
            "惡霸礦頭",
            "結怨嫌隙",
            "曾以次充好刁難礦價".to_string(),
            35,
            now,
        );
        let mut store = crate::store::Store::new_in_memory();
        let mut smith = crate::store::Entity::new_npc(smith_id, "岑");
        smith.display_title = "文曜鐵坊掌錘|岑守岳".to_string();
        smith.grid_x = Some(6);
        smith.grid_y = Some(4);
        smith.magnesium = 36;
        smith.equipment_slots = r#"{"hold_r":"wt_blacksmith_hammer"}"#.to_string();
        smith.inventory = r#"[{"item_id":"wt_cat_0387","qty":4},{"item_id":"wt-good-019","qty":1}]"#.to_string();
        let _ = store.upsert_entity_full(smith);

        let mut bully = crate::store::Entity::new_npc("惡霸礦頭", "惡");
        bully.magnesium = 50;
        bully.inventory = r#"[{"item_id":"wt-good-019","qty":10}]"#.to_string();
        let _ = store.upsert_entity_full(bully);

        let mut shi = crate::store::Entity::new_npc("石萬鈞", "石");
        shi.magnesium = 20;
        shi.inventory = r#"[{"item_id":"wt-good-019","qty":10}]"#.to_string();
        let _ = store.upsert_entity_full(shi);

        let (_, _, act) = step_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            11,
            now + 60,
        );
        assert!(act.contains("石萬鈞"), "expected autonomous choice of 石萬鈞 over grudge supplier: {act}");
        assert!(!act.contains("惡霸礦頭"));
        let shi_after = store.get_entity("石萬鈞").unwrap();
        assert_eq!(shi_after.magnesium, 32);
        assert!(
            shi_after.thoughts.iter().any(|t| t.flavor.as_deref().unwrap_or("").contains("赤鐵粗礦")),
            "expected supplier 石萬鈞 to receive dyadic event thought from buyer's action"
        );
    }
    #[test]
    fn test_need_decision_spatial_tag_and_uniform_counter_discovery() {
        use crate::grid::{GridCell, SquareCoord, SquareGrid, Terrain};
        use crate::store::Entity;

        let forbidden_names = [
            "周萬生", "陳萬福", "石萬鈞", "梁千木", "葉青囊", "馬山行",
            "顧海平", "蘇錦心", "田百稼", "柯成器", "溫懷瑾", "羅烈",
            "許海音", "沈清河",
        ];

        // 1. 驗證 ResidentIntrinsicFactors::resolve 在 Store 無任何實體時，
        //    產生的 supplier_id 與 counterparty_id 完全為空，絕不包含任何寫死姓名
        let test_npc_definitions = [
            ("秦水生", "水", "老堰頭・水輪石磨工坊", "碾米"),
            ("岑守岳", "岑", "文曜・百鍛鐵工坊", "鍛打鐵器"),
            ("柯成器", "柯", "文曜・巧木居所", "刨削木料"),
            ("葉青囊", "葉", "文曜・百草診所", "分揀草藥"),
            ("馬山行", "馬", "文曜・皮革行", "熟化鹿皮"),
            ("顧海平", "顧", "文曜・水產行", "醃漬鮮魚"),
            ("蘇錦心", "蘇", "文曜・雲錦織造", "裁剪布料"),
        ];

        for (id, char_sym, bldg, act) in test_npc_definitions {
            let ent = Entity::new_npc(id, char_sym);
            let factors = ResidentIntrinsicFactors::resolve(&ent, 0, 0, bldg, act);

            assert!(
                factors.supplier_id.is_empty(),
                "factors.supplier_id for {id} must be empty by default, got: {}",
                factors.supplier_id
            );
            assert!(
                factors.counterparty_id.is_empty(),
                "factors.counterparty_id for {id} must be empty by default, got: {}",
                factors.counterparty_id
            );

            for forbidden in &forbidden_names {
                assert!(
                    !factors.supplier_id.contains(forbidden),
                    "factors.supplier_id for {id} contains hardcoded name: {forbidden}"
                );
                assert!(
                    !factors.counterparty_id.contains(forbidden),
                    "factors.counterparty_id for {id} contains hardcoded name: {forbidden}"
                );
            }
            assert!(!factors.need_supply_tag.is_empty());
            assert!(!factors.need_buyer_tag.is_empty());
        }

        // 2. 驗證空間距離加權：沙鹿巷口便利小鋪（距離 2） vs concat!("竹北高", "鐵便利大行")（距離 200）
        //    無論竹北物資多豐厚，角色絕不從沙鹿遠遁竹北尋小七，必定向就近之沙鹿巷口小七補料
        let mut grid = SquareGrid::new();
        let near_coord = SquareCoord::new(2, 0);
        let mut near_room = GridCell::new(near_coord, Terrain::GeneralStore, "沙鹿巷口便利小鋪");
        near_room.tags.push("grain".to_string());
        grid.insert(near_room);

        let far_coord = SquareCoord::new(200, 0);
        let mut far_room = GridCell::new(far_coord, Terrain::GeneralStore, concat!("竹北高", "鐵便利大行"));
        far_room.tags.push("grain".to_string());
        grid.insert(far_room);

        let store = crate::store::Store::new_in_memory();
        let qin_ent = Entity::new_npc("秦水生", "水");
        let qin_factors = ResidentIntrinsicFactors::resolve(&qin_ent, 0, 0, "老堰頭・水輪石磨工坊", "碾米");
        let pw = crate::npc::causal_core::PluralPersonalityWeights::default();

        let chosen_supplier = resolve_need_decision_supplier(
            &store,
            "秦水生",
            (0, 0), // 當前座標 (0, 0)
            &qin_factors,
            &pw,
            &[],
            Some(&grid),
        );

        assert!(chosen_supplier.is_some(), "expected to find a matching room supplier");
        let (sup_id, sup_place, sup_coord) = chosen_supplier.unwrap();
        assert_eq!(
            sup_coord, (2, 0),
            "character must choose near room at (2,0) over faraway room at (200,0)"
        );
        assert!(
            sup_id.contains("沙鹿") || sup_place.contains("沙鹿"),
            "supplier must be the nearby 沙鹿 room, got id: {sup_id}, place: {sup_place}"
        );
        assert!(!sup_id.contains("竹北") && !sup_place.contains("竹北"));

        // 3. 驗證進店辨識：區分店內閒聊路人顧客 vs 穿制服店員 vs 穿便服但站在非夥計禁止入內櫃檯區的掌事
        let room_coord = (2, 0);
        let needed_item = "wt-good-009";

        // (a) 路人顧客：無制服、不在櫃檯內場、無店家原料庫存
        let mut passerby = Entity::new_npc("路人顧客_阿明", "客");
        passerby.display_title = "店內喝茶閒聊的茶客顧客|阿明".to_string();
        passerby.grid_x = Some(2);
        passerby.grid_y = Some(0);
        let (u_passerby, c_passerby) = is_recognizable_room_staff_or_counter_holder_entity(&passerby, room_coord, needed_item);
        assert!(
            !u_passerby && !c_passerby,
            "passerby customer must NOT be recognized as staff or counter holder"
        );

        // (b) 穿制服店員：身穿制服
        let mut clerk = Entity::new_npc("鋪子夥計_小張", "員");
        clerk.display_title = "身穿制服店員|小張".to_string();
        clerk.grid_x = Some(2);
        clerk.grid_y = Some(0);
        let (u_clerk, _c_clerk) = is_recognizable_room_staff_or_counter_holder_entity(&clerk, room_coord, needed_item);
        assert!(
            u_clerk,
            "uniformed clerk must have visible uniform"
        );

        // (c) 穿便服但站在非夥計禁止入內櫃檯區的掌事：
        //     即便身穿便服（無制服），因站於非夥計禁止入內櫃檯管制區，必定被精準辨識為掌事
        let mut manager = Entity::new_npc("沙鹿掌事_老王", "王");
        manager.display_title = "穿便服但站在非夥計禁止入內櫃檯區的掌事|老王".to_string();
        manager.grid_x = Some(2);
        manager.grid_y = Some(0);
        let (u_manager, c_manager) = is_recognizable_room_staff_or_counter_holder_entity(&manager, room_coord, needed_item);
        assert!(
            !u_manager,
            "manager in plain clothes has no visible uniform"
        );
        assert!(
            c_manager,
            "manager standing in staff-only counter zone MUST be recognized as counter holder"
        );
    }
    #[test]
    fn test_all_1785_residents_receive_manuals_familiarity_breath_and_lineage() {
        let now = 1_800_200_000i64;
        seed_flagship_specimen_qin_shuisheng_once(now);

        let wenyao_name = crate::store::WENYAO_HOUSEHOLD_SLOTS[10].name();
        let outer_name = crate::store::OUTER_HOUSEHOLD_SLOTS[10].name();

        // Wenyao resident parity checks
        let wy_manuals = get_entity_studied_manuals(wenyao_name);
        assert!(
            !wy_manuals.is_empty(),
            "wenyao resident {wenyao_name} must receive studied manuals"
        );
        let wy_fam = get_entity_anatomy_familiarity_total(wenyao_name);
        assert!(
            wy_fam >= 24,
            "wenyao resident {wenyao_name} familiarity must be at least 24, got {wy_fam}"
        );
        let wy_lineage = get_entity_lineage_imprints(wenyao_name);
        assert!(
            !wy_lineage.is_empty(),
            "wenyao resident {wenyao_name} must receive lineage imprints"
        );

        // Outer resident parity checks
        let out_manuals = get_entity_studied_manuals(outer_name);
        assert!(
            !out_manuals.is_empty(),
            "outer resident {outer_name} must receive studied manuals"
        );
        let out_fam = get_entity_anatomy_familiarity_total(outer_name);
        assert!(
            out_fam >= 24,
            "outer resident {outer_name} familiarity must be at least 24, got {out_fam}"
        );
        let out_lineage = get_entity_lineage_imprints(outer_name);
        assert!(
            !out_lineage.is_empty(),
            "outer resident {outer_name} must receive lineage imprints"
        );
    }
    #[test]
    fn test_non_miller_vocations_never_hijacked_into_rice_milk_or_umbrella_tool() {
        let mut acct = crate::store::Entity::new_npc("秦明誠", "秦");
        acct.display_title = "文曜商算核契師|秦明誠".to_string();
        acct.equipment_slots = r#"{"hold_r":"wt_cat_0954"}"#.to_string();
        acct.inventory = r#"[{"item_id":"wt-good-009","qty":3},{"item_id":"wt_cat_0781","qty":4}]"#.to_string();
        let factors_acct = ResidentIntrinsicFactors::resolve(&acct, 0, 0, "文曜・百工商算館", "核對商契");
        assert!(
            !factors_acct.tool_name.contains("傘") && !factors_acct.tool_name.contains("膏"),
            "umbrella or ointment must never be selected as vocational tool: {}",
            factors_acct.tool_name
        );
        assert!(
            factors_acct.outputs.iter().all(|(_, name, _, _)| !name.contains("米漿")),
            "accountant must never produce rice milk: {:?}",
            factors_acct.outputs
        );

        let mut mason = crate::store::Entity::new_npc("鄭烏石", "鄭");
        mason.display_title = "三號坑鑿石工匠|鄭烏石".to_string();
        let factors_mason = ResidentIntrinsicFactors::resolve(&mason, -18, 22, "三號坑・鑿石切石場", "鑿切玄武岩");
        assert_eq!(factors_mason.raw_item_name, "玄武岩原石");
        assert!(
            factors_mason.outputs.iter().any(|(_, name, _, _)| name.contains("玄武岩基石")),
            "mason must produce basalt foundation stones: {:?}",
            factors_mason.outputs
        );

        assert!(
            crate::grid::resolve_any_room_venue_object("雲腰雪林", "").is_none(),
            "wilderness forest must not resolve to a counter sawmill venue"
        );
        assert!(
            crate::grid::resolve_any_room_venue_object("翠冠巨木林", "").is_none(),
            "wilderness forest must not resolve to a counter sawmill venue"
        );
        assert!(
            crate::game::peninsula_scripts::scripts_for_room("文曜・分家巷百工石道").is_empty(),
            "streets without an explicit script must not inherit unrelated settlement workshop scripts"
        );
        assert_eq!(
            crate::game::peninsula_scripts::scripts_for_room("文曜・三水匯流大石橋").len(),
            1,
            "bridge must only match its own explicit script, never settlement-wide workshop scripts"
        );
    }

}

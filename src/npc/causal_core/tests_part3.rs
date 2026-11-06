    #[test]
    fn test_offline_player_grid_stepping_and_material_transformation() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if crate::store::get_store().is_none() {
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let rooms = manifest_dir.join("data/rooms");
            let runtime = manifest_dir.join("data/runtime");
            let data = manifest_dir.join("data");
            let _ = crate::store::init(
                rooms.to_str().unwrap_or("data/rooms"),
                runtime.to_str().unwrap_or("data/runtime"),
                data.to_str().unwrap_or("data"),
            );
        }

        let pid = "test_offline_walker_crafter";
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut ent = crate::store::Entity::default();
            ent.id = pid.to_string();
            ent.display_title = "老堰頭水輪石磨工匠".to_string();
            ent.equipment_slots = r#"{"hold_r":"wt_cat_0984"}"#.to_string();
            ent.inventory = r#"[{"item_id":"wt-good-009","qty":5}]"#.to_string();
            ent.grid_x = Some(0);
            ent.grid_y = Some(100);
            ent.vit = 80;
            ent.qi = 60;
            s.entities.insert(ent.id.clone(), ent);
        }

        set_player_offline_directive(pid, OfflineAutonomyDirective::WorkshopCraftAndAssist);

        let now = 2010000;
        let step_res = step_offline_player_once(pid, now);
        assert!(step_res.is_some());

        // 1. 驗證實體 (grid_x, grid_y) 朝向老堰頭水輪石磨工坊目標座標 (-3, 125) 邁進
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            let ent = s.entities.get(pid).expect("player entity should exist in store");
            let nx = ent.grid_x.expect("grid_x should be Some");
            let ny = ent.grid_y.expect("grid_y should be Some");
            assert_ne!((nx, ny), (0, 100));
            assert_eq!(ent.target_x, Some(-3));
            assert_eq!(ent.target_y, Some(125));

            // 2. 驗證原料 (wt-good-009) 經工坊轉化為成品 (wt_cat_0388 溫米漿)
            let entries: Vec<crate::db::inventory::InventoryEntry> =
                serde_json::from_str(&ent.inventory).expect("inventory must be valid json");
            let raw_item = entries.iter().find(|it| it.item_id == "wt-good-009");
            assert_eq!(raw_item.map(|it| it.qty), Some(4));
            let finished_item = entries.iter().find(|it| it.item_id == "wt_cat_0388");
            assert!(finished_item.is_some());
            assert!(finished_item.unwrap().qty >= 1);
        }

        // 3. 驗證已成功登記至 RESIDENT_MOVE_TARGETS
        {
            let targets = crate::server::grid_manager::RESIDENT_MOVE_TARGETS.read();
            let entry = targets.get(pid);
            assert!(entry.is_some());
            let (tx, ty, bldg, act) = entry.unwrap();
            assert_eq!(*tx, -3);
            assert_eq!(*ty, 125);
            assert!(bldg.contains("石磨") || bldg.contains("老堰頭"));
            assert!(act.contains("工坊"));
        }
    }

    #[test]
    fn test_offline_player_and_canteen_spatial_need_decision() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if crate::store::get_store().is_none() {
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let rooms = manifest_dir.join("data/rooms");
            let runtime = manifest_dir.join("data/runtime");
            let data = manifest_dir.join("data");
            let _ = crate::store::init(
                rooms.to_str().unwrap_or("data/rooms"),
                runtime.to_str().unwrap_or("data/runtime"),
                data.to_str().unwrap_or("data"),
            );
        }

        // 1. 空間需求決策：resolve_nearest_room_by_tag 就近選取食堂，拒絕跨全地圖硬編碼至 (-4, 4)
        let mut test_grid = crate::grid::SquareGrid::new();
        let mut far_canteen = crate::grid::GridCell::new(
            crate::grid::SquareCoord::new(-4, 4),
            crate::grid::Terrain::Tavern,
            "文曜・三水大食堂",
        );
        far_canteen.tags.push("canteen".to_string());
        test_grid.insert(far_canteen);

        let mut near_canteen = crate::grid::GridCell::new(
            crate::grid::SquareCoord::new(20, 20),
            crate::grid::Terrain::Tavern,
            "東街・聚仙熱食館",
        );
        near_canteen.tags.push("canteen".to_string());
        near_canteen.tags.push("hot_food".to_string());
        test_grid.insert(near_canteen);

        // 位於 (21, 20) 的角色查詢最近食堂，應選中 (20, 20)
        let (chosen_coord, chosen_bldg) = crate::pg::living_world::resolve_nearest_room_by_tag(
            (21, 20),
            &["canteen", "hot_food", "fresh_drink"],
            Some(&test_grid),
            (-4, 4),
            "文曜・三水大食堂",
        );
        assert_eq!(chosen_coord, (20, 20));
        assert!(chosen_bldg.contains("聚仙") || chosen_bldg.contains("東街"));
        assert!(!chosen_bldg.contains("三水大食堂"));

        // 驗證若未提供 grid 且執行期 grid 未就緒時，保留既有 fallback 設施
        let (fb_coord, fb_bldg) = crate::pg::living_world::resolve_nearest_room_by_tag(
            (-3, 125),
            &["canteen", "hot_food", "fresh_drink"],
            None,
            (1, 124),
            "老堰頭・堰渠水畔食堂",
        );
        assert_eq!(fb_coord, (1, 124));
        assert_eq!(fb_bldg, "老堰頭・堰渠水畔食堂");

        // 2. 離線玩家工坊缺料補料 (RestockGrain)：依 resolve_need_decision_supplier 動態採買原料
        let sup_room_coord = crate::grid::SquareCoord::new(21, 20);
        let mut sup_room = crate::grid::GridCell::new(
            sup_room_coord,
            crate::grid::Terrain::GeneralStore,
            "東街米糧行",
        );
        sup_room.tags.push("grain".to_string());
        test_grid.insert(sup_room);

        // 注入測試地圖至執行期快照
        crate::server::grid_manager::set_runtime_square_grid_for_testing(test_grid.clone());

        let restock_pid = "test_offline_need_restocker";
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut ent = crate::store::Entity::default();
            ent.id = restock_pid.to_string();
            ent.display_title = "老堰頭水輪石磨工匠".to_string();
            ent.equipment_slots = r#"{"hold_r":"wt_cat_0984"}"#.to_string();
            ent.inventory = "[]".to_string(); // 原料存量為 0
            ent.grid_x = Some(21);
            ent.grid_y = Some(20);
            ent.vit = 80;
            ent.qi = 60;
            ent.magnesium = 50;
            s.entities.insert(ent.id.clone(), ent);
        }
        set_player_offline_directive(restock_pid, OfflineAutonomyDirective::WorkshopCraftAndAssist);

        let now = 2010000;
        let step_res = step_offline_player_once(restock_pid, now);
        assert!(step_res.is_some());
        let body = step_res.unwrap();
        assert!(body.contains("採買") || body.contains("用料"));
        let recaps = drain_player_offline_recap(restock_pid, now);
        assert!(!recaps.is_empty());
        assert!(recaps[0].1.contains("採購") || recaps[0].1.contains("採買"));

        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get(restock_pid) {
                let entries: Vec<crate::db::inventory::InventoryEntry> =
                    serde_json::from_str(&e.inventory).unwrap_or_default();
                let raw_count = entries
                    .iter()
                    .find(|it| it.item_id == "wt-good-009")
                    .map(|it| it.qty)
                    .unwrap_or(0);
                assert!(raw_count > 0, "原料應已成功採買補充至玩家行囊");
                assert!(e.magnesium < 50, "採買原料應已扣除相應鎂幣");
            }
        }

        // 3. 離線玩家完工交割 (DeliverContract)：依 resolve_need_decision_buyer 動態送交成品
        let buy_room_coord = crate::grid::SquareCoord::new(20, 21);
        let mut buy_room = crate::grid::GridCell::new(
            buy_room_coord,
            crate::grid::Terrain::Tavern,
            "東街四海茶樓",
        );
        buy_room.tags.push("canteen".to_string());
        test_grid.insert(buy_room);
        crate::server::grid_manager::set_runtime_square_grid_for_testing(test_grid);

        let deliver_pid = "test_offline_need_deliverer";
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut ent = crate::store::Entity::default();
            ent.id = deliver_pid.to_string();
            ent.display_title = "老堰頭水輪石磨工匠".to_string();
            ent.equipment_slots = r#"{"hold_r":"wt_cat_0984"}"#.to_string();
            ent.inventory = r#"[{"item_id":"wt-good-009","qty":2},{"item_id":"wt_cat_0388","qty":3}]"#.to_string();
            ent.grid_x = Some(20);
            ent.grid_y = Some(21);
            ent.vit = 80;
            ent.qi = 60;
            ent.magnesium = 10;
            s.entities.insert(ent.id.clone(), ent);
        }
        set_player_offline_directive(deliver_pid, OfflineAutonomyDirective::WorkshopCraftAndAssist);

        let step_deliv = step_offline_player_once(deliver_pid, now);
        assert!(step_deliv.is_some());
        let d_body = step_deliv.unwrap();
        assert!(d_body.contains("成品") || d_body.contains("點交") || d_body.contains("交割"));
        let d_recaps = drain_player_offline_recap(deliver_pid, now);
        assert!(!d_recaps.is_empty());
        assert!(d_recaps[0].1.contains("交割") || d_recaps[0].1.contains("貨款"));

        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get(deliver_pid) {
                let entries: Vec<crate::db::inventory::InventoryEntry> =
                    serde_json::from_str(&e.inventory).unwrap_or_default();
                let fin_count = entries
                    .iter()
                    .find(|it| it.item_id == "wt_cat_0388")
                    .map(|it| it.qty)
                    .unwrap_or(0);
                assert!(fin_count < 3, "交割後成品應已扣除");
                assert!(e.magnesium > 10, "交割後應已獲取貨款鎂幣進帳");
            }
        }

        // 4. 離線玩家古道巡護 (RoadEscortAndPatrol)：距離半徑過濾與動態同伴標籤
        let patrol_pid = "test_offline_need_patroller";
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut ent = crate::store::Entity::default();
            ent.id = patrol_pid.to_string();
            ent.display_title = "古道護衛巡客".to_string();
            ent.grid_x = Some(20);
            ent.grid_y = Some(20);
            ent.vit = 80;
            ent.qi = 60;
            s.entities.insert(ent.id.clone(), ent);

            // 設置近處同行商客 (距離 2 <= 30)
            let mut traveler = crate::store::Entity::default();
            traveler.id = "npc_traveler_east_01".to_string();
            traveler.kind = "npc".to_string();
            traveler.display_title = "東街同行商旅客".to_string();
            traveler.grid_x = Some(22);
            traveler.grid_y = Some(20);
            s.entities.insert(traveler.id.clone(), traveler);
        }
        set_player_offline_directive(patrol_pid, OfflineAutonomyDirective::RoadEscortAndPatrol);

        let step_patrol = step_offline_player_once(patrol_pid, now);
        assert!(step_patrol.is_some());
        let p_body = step_patrol.unwrap();
        assert!(p_body.contains("東街同行商旅客"), "必須選取距離近之行旅 NPC");
        assert!(!p_body.contains("古道行商阿福"), "不得使用硬編碼之特定 NPC 姓名");

        // 若四周無近處 NPC，同伴稱謂應依聚落動態生成，而非固定為古道行商阿福
        let lonely_pid = "test_offline_lonely_patroller";
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut ent = crate::store::Entity::default();
            ent.id = lonely_pid.to_string();
            ent.display_title = "荒野哨探巡行者".to_string();
            ent.grid_x = Some(900); // 孤立遠處無任何 NPC
            ent.grid_y = Some(900);
            ent.vit = 80;
            ent.qi = 60;
            s.entities.insert(ent.id.clone(), ent);
        }
        set_player_offline_directive(lonely_pid, OfflineAutonomyDirective::RoadEscortAndPatrol);

        let step_lonely = step_offline_player_once(lonely_pid, now);
        assert!(step_lonely.is_some());
        let l_body = step_lonely.unwrap();
        assert!(!l_body.contains("古道行商阿福"), "無同伴時不得出現硬編碼古道行商阿福");
        assert!(l_body.contains("往來行旅同伴"), "應使用動態同伴稱謂");
    }

    #[test]
    fn test_crop_plot_maturation_and_harvest() {
        let coord = (99, 101);
        let now = 3000000;

        // Register sown plot
        let sown = register_crop_plot_sown(coord.0, coord.1, "water_field", now);
        assert_eq!(sown.stage, CropGrowthStage::Seedling);
        assert_eq!(sown.crop_name, "大圳頭飽滿新米");

        // Harvesting now should fail (not mature)
        let early_harvest = harvest_crop_plot_once(coord.0, coord.1, "water_field", now + 10);
        assert!(early_harvest.is_err());

        // Fast forward 50s (water_nourished: 50 * 2 = 100s >= 90s -> Mature)
        let mature = inspect_or_advance_crop_plot(coord.0, coord.1, "water_field", now + 50);
        assert_eq!(mature.stage, CropGrowthStage::Mature);

        // Harvest mature crop
        let harvest = harvest_crop_plot_once(coord.0, coord.1, "water_field", now + 50);
        assert!(harvest.is_ok());
        let (item_id, item_name, qty, prose) = harvest.unwrap();
        assert_eq!(item_id, "mat_grain_sack");
        assert_eq!(item_name, "大圳頭飽滿新米");
        assert_eq!(qty, 2);
        assert!(prose.contains("熟成"));

        // Lazy-seeding native map field starts at Mature
        let native_plot = inspect_or_advance_crop_plot(88, 77, "herb_garden", now);
        assert_eq!(native_plot.stage, CropGrowthStage::Mature);
        assert_eq!(native_plot.crop_item_id, "harvest_herb");
    }

    #[test]
    fn test_player_bank_ledger_sfc_invariance() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if crate::store::get_store().is_none() {
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let rooms = manifest_dir.join("data/rooms");
            let runtime = manifest_dir.join("data/runtime");
            let data = manifest_dir.join("data");
            let _ = crate::store::init(
                rooms.to_str().unwrap_or("data/rooms"),
                runtime.to_str().unwrap_or("data/runtime"),
                data.to_str().unwrap_or("data"),
            );
        }

        let player_id = "test_player_bank_sfc_inv";
        {
            let store_arc = crate::store::get_store().expect("store available");
            let mut store = store_arc.write();
            normalize_and_seed_circular_economy_once(&mut store);
            let player: crate::store::Entity = serde_json::from_str(&format!(
                r#"{{"id":"{}","kind":"player","magnesium":500,"display_title":"白塔旅人"}}"#,
                player_id
            )).unwrap();
            store.entities.insert(player_id.to_string(), player);
        }

        {
            let mut ledger = LIVE_PLAYER_BANK_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            ledger.remove(player_id);
        }

        let store_arc = crate::store::get_store().expect("store available");
        let initial_total = total_conserved_magnesium(&store_arc.read());

        // Deposit 200
        let (actual_dep, remaining_wallet, new_bank) = deposit_player_bank_magnesium(player_id, 200);
        assert_eq!(actual_dep, 200);
        assert_eq!(remaining_wallet, 300);
        assert_eq!(new_bank, 200);
        assert_eq!(get_player_bank_balance(player_id), 200);
        let after_dep_total = total_conserved_magnesium(&store_arc.read());
        assert_eq!(initial_total, after_dep_total);

        // Withdraw 80
        let (actual_with, new_wallet, remaining_bank) = withdraw_player_bank_magnesium(player_id, 80);
        assert_eq!(actual_with, 80);
        assert_eq!(new_wallet, 380);
        assert_eq!(remaining_bank, 120);
        assert_eq!(get_player_bank_balance(player_id), 120);
        let after_with_total = total_conserved_magnesium(&store_arc.read());
        assert_eq!(initial_total, after_with_total);

        // Withdraw all remaining
        let (over_with, final_wallet, zero_bank) = withdraw_player_bank_magnesium(player_id, 9999);
        assert_eq!(over_with, 120);
        assert_eq!(final_wallet, 500);
        assert_eq!(zero_bank, 0);
        let final_total = total_conserved_magnesium(&store_arc.read());
        assert_eq!(initial_total, final_total);
    }

    #[test]
    fn test_player_home_stash_store_and_retrieve() {
        let pid = "test_player_home_stash_user";
        {
            let mut stash = LIVE_PLAYER_HOME_STASH.write().unwrap_or_else(|e| e.into_inner());
            stash.remove(pid);
        }
        assert_eq!(list_player_home_stash(pid).len(), 0);
        assert!(retrieve_item_from_home_stash(pid).is_none());

        for i in 1..=24 {
            let res = store_item_in_home_stash(pid, &format!("item_{}", i), &format!("道具_{}", i));
            assert_eq!(res, Ok(i));
        }
        assert_eq!(list_player_home_stash(pid).len(), 24);

        // 25th item should fail (cap 24)
        let overflow = store_item_in_home_stash(pid, "item_25", "道具_25");
        assert!(overflow.is_err());

        // Pop last
        let pop1 = retrieve_item_from_home_stash(pid);
        assert_eq!(pop1, Some(("item_24".to_string(), "道具_24".to_string(), 23)));
        assert_eq!(list_player_home_stash(pid).len(), 23);

        // Store again
        let store_again = store_item_in_home_stash(pid, "item_new", "新玉珮");
        assert_eq!(store_again, Ok(24));
    }

    #[test]
    fn test_compile_live_peninsula_gazette_and_mailbox_letter() {
        let now = 1700000000;
        let gazette = compile_live_peninsula_gazette("文曜・市集石坪", now);
        assert!(gazette.contains("【四鎮商貿與緊缺行情】"));
        assert!(gazette.contains("【山海聯運與受阻急貨】"));
        assert!(gazette.contains("【山海物候與獵場警訊】"));
        assert!(gazette.contains("【街坊新近見聞】"));
        assert!(!gazette.contains("SFC"));
        assert!(!gazette.contains("Stock-Flow"));
        assert!(!gazette.contains("RelationalHook"));
        assert!(!gazette.contains("WorldRippleFact"));
        assert!(!gazette.contains("LazyLock"));
        assert!(!gazette.contains("剛才"));
        assert!(!gazette.contains("剛剛"));
        assert!(!gazette.contains("方才"));

        let pid = "test_player_mailbox_user";
        {
            let mut mb = LIVE_ACQUAINTANCE_MAILBOX.write().unwrap_or_else(|e| e.into_inner());
            mb.remove(pid);
        }

        let (letter1, gift1) = claim_acquaintance_mailbox_letter(pid, "旅人小林", now);
        assert!(letter1.contains("【竹筒信箱・親善手札】"));
        assert!(gift1.is_some());
        assert!(!letter1.contains("剛才"));
        assert!(!letter1.contains("剛剛"));
        assert!(!letter1.contains("方才"));

        // Second claim in same window returns None for gift
        let (letter2, gift2) = claim_acquaintance_mailbox_letter(pid, "旅人小林", now + 10);
        assert!(letter2.contains("【竹筒信箱・親善手札】"));
        assert!(gift2.is_none());

        // Claim in next window returns Some gift
        let (_letter3, gift3) = claim_acquaintance_mailbox_letter(pid, "旅人小林", now + 301);
        assert!(gift3.is_some());
    }

    #[test]
    fn test_record_player_npc_fellowship_hook_and_mailbox() {
        let npc_id = "test_npc_chess_fellow";
        let pid = "test_player_fellow_01";
        let now = 1710000000;
        let summary = "曾在茶屋烏木棋秤前對弈手談，棋路相契";

        record_player_npc_fellowship_hook(npc_id, pid, summary, now);

        // 1. Verify in LIVE_NPC_CAUSAL_STATES
        {
            let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
            let state = states.get(npc_id).expect("npc state must exist");
            let hook = state.hooks.iter().find(|h| h.target_id == pid).expect("hook toward player must exist");
            assert_eq!(hook.intensity, 75);
            assert_eq!(hook.summary, summary);
            assert_eq!(hook.kind_label, "結識同行");
        }

        // 2. Verify letter queued in mailbox and can be claimed
        let (letter, gift) = claim_acquaintance_mailbox_letter(pid, pid, now);
        assert!(letter.contains("【竹筒信箱・親善手札】"));
        assert!(letter.contains(summary));
        assert!(gift.is_some());
        assert!(!letter.contains("剛才"));
        assert!(!letter.contains("剛剛"));
        assert!(!letter.contains("方才"));
    }


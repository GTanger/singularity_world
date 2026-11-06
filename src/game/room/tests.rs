#[cfg(test)]
mod tests {
    use super::*;

    static TEST_TROPHIC_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_ensure_ambient_npc_at_grid() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let west_gate = ensure_ambient_npc_at_grid(-18, 0, "西麓林道・道路");
        assert!(west_gate.is_some());
        let npc = west_gate.unwrap();
        assert_eq!(npc.kind, EntityKind::Npc);
        assert!(!npc.display_title.is_empty());
        assert!(!npc.current_activity.is_empty());
        assert!(npc.vit > 0);

        // 2. 獵場深處環境生物
        // 2. 獵場深處環境生物（預設顯形尋常物種／小怪，不直接刷出鎮場霸主）
        let hunting_ground = ensure_ambient_npc_at_grid(-180, -195, "長風草甸・向陽原野");
        assert!(hunting_ground.is_some());
        let creature = hunting_ground.unwrap();
        assert!(!creature.display_title.is_empty());
        assert_ne!(creature.display_title, "長風四目水鹿王");

        // 3. 尋跡顯形後鎮場霸主破霧現身
        mark_apex_beast_tracked_at(-180, -195);
        let tracked_apex = ensure_ambient_npc_at_grid(-180, -195, "長風草甸・向陽原野").unwrap();
        assert_eq!(tracked_apex.display_title, "長風四目水鹿王");

        // 4. 深水不應生成環境生物
        let deep_water = ensure_ambient_npc_at_grid(-390, 0, "北冕海・深水");
        assert!(deep_water.is_none());
    }

    #[test]
    fn test_hunting_zone_apex_omen() {
        let all_19_zones = [
            "長風草甸", "古河巨木林", "赤石坡", "落日岬", "驚濤岩岸",
            "巨藻淺灣", "黑石灣", "翠冠巨木林", "清泉溪谷", "北坡雲杉林",
            "西麓林道", "淺澤泥口", "白茫洲", "貼山深潭", "青籠關",
            "西嶺礦谷", "東海峭壁", "雲腰雪林", "白塔雪巔",
        ];
        for z in all_19_zones {
            let omen = hunting_zone_apex_omen(z);
            assert!(omen.is_some(), "Zone {z} must have an apex omen");
            let text = omen.unwrap();
            assert!(!text.contains("【山海異兆】"), "Omen must not contain bracketed system header");
            assert!(text.chars().count() >= 20);
        }
        assert!(hunting_zone_apex_omen("文曜門前大街").is_none());
    }

    #[test]
    fn test_shanhai_item_use_effect() {
        // 護身珍品：不消耗
        let (cat, vit, qi, consume, narr) = shanhai_item_use_effect("wt_cat_0214").unwrap();
        assert_eq!(cat, "佩之・山海靈佩");
        assert_eq!(vit, 0);
        assert_eq!(qi, 10);
        assert!(!consume);
        assert!(!narr.is_empty());

        let (_, _, _, consume, _) = shanhai_item_use_effect("丹冠藍腹鷴彩羽").unwrap();
        assert!(!consume);

        // 消耗性靈藥／珍餚：消耗
        let (cat, vit, qi, consume, narr) = shanhai_item_use_effect("wt_cat_0205").unwrap();
        assert_eq!(cat, "食之・山海珍饈");
        assert_eq!(vit, 25);
        assert_eq!(qi, 10);
        assert!(consume);
        assert!(!narr.is_empty());

        let (_, _, _, consume, _) = shanhai_item_use_effect("百步蛇乾蛻").unwrap();
        assert!(consume);
        let (_, _, _, consume, _) = shanhai_item_use_effect("深山牛樟芝").unwrap();
        assert!(consume);

        assert!(shanhai_item_use_effect("wt_cat_unknown").is_none());
    }

    #[test]
    fn test_player_shanhai_talisman_bonus() {
        let inv_with_croc = r#"[{"item_id":"wt_cat_0176","quantity":1}]"#;
        let bonus = player_shanhai_talisman_bonus(inv_with_croc);
        assert!(bonus.is_some());
        let (name, vit, qi, desc) = bonus.unwrap();
        assert_eq!(name, "泥甲巨鱷厚麟皮");
        assert_eq!(vit, 12);
        assert_eq!(qi, 0);
        assert!(!desc.is_empty());

        let inv_with_pheasant = r#"[{"item_id":"wt_cat_0214","quantity":1}]"#;
        let bonus = player_shanhai_talisman_bonus(inv_with_pheasant);
        assert!(bonus.is_some());
        let (name, vit, qi, _) = bonus.unwrap();
        assert_eq!(name, "霧林帝雉長尾翎");
        assert_eq!(vit, 0);
        assert_eq!(qi, 10);

        let inv_empty = r#"[]"#;
        assert!(player_shanhai_talisman_bonus(inv_empty).is_none());
    }

    #[test]
    fn test_zone_ambient_word_source() {
        use crate::combat::wordplate::WordSource;
        let qi_zone = zone_ambient_word_source("白塔雪巔").unwrap();
        assert_eq!(qi_zone.0, WordSource::Qi);
        let water_zone = zone_ambient_word_source("貼山深潭").unwrap();
        assert_eq!(water_zone.0, WordSource::Water);
        let plant_zone = zone_ambient_word_source("翠冠巨木林").unwrap();
        assert_eq!(plant_zone.0, WordSource::Plant);
        let min_zone = zone_ambient_word_source("西嶺礦谷").unwrap();
        assert_eq!(min_zone.0, WordSource::Mineral);
        let fallback_zone = zone_ambient_word_source("文曜街市").unwrap();
        assert_eq!(fallback_zone.0, WordSource::Qi);
    }

    #[test]
    fn test_trophic_cascade_on_apex_slain() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let z_idx = 0; // 長風草甸
        let initial_st = get_zone_trophic_state(z_idx).expect("Zone state exists");
        assert!(initial_st.apex_presence >= 0.95);
        let slots_before = live_zone_shanhai_slots(z_idx, 100);
        assert!(slots_before.iter().any(|s| s.slot_idx == 0 && s.tier == 1));

        // Slay Tier 1 Apex
        let (_zone_name, cx, cy) = CANONICAL_HUNTING_ZONES[z_idx];
        record_wilderness_kill_at_coord(
            cx + 1,
            cy - 1,
            &format!("shanhai_{z_idx}_0_{}_{}", cx + 1, cy - 1),
            "長風四目水鹿王",
            "wt_cat_apex_loot",
            "水鹿王神角",
            "獵手",
            crate::game::now_unix(),
        );

        let st_slain = get_zone_trophic_state(z_idx).unwrap();
        assert_eq!(st_slain.apex_presence, 0.0);

        // Slot 0 should no longer be emitted in live_zone_shanhai_slots
        let slots_slain = live_zone_shanhai_slots(z_idx, 101);
        assert!(!slots_slain.iter().any(|s| s.slot_idx == 0));

        // Over 3 trophic steps: mesopredators surge (> 6.0), n_tier3 / n_tier4 dip but stay >= refuge
        for _ in 0..3 {
            step_zone_trophic_web_once(z_idx);
        }

        let st_after = get_zone_trophic_state(z_idx).unwrap();
        assert!(
            st_after.n_tier2 > 6.0,
            "Mesopredators should surge above 6.0, got {}",
            st_after.n_tier2
        );
        let ref4 = (st_after.k_tier4 * 0.18).max(6.0);
        let ref3 = (st_after.k_tier3 * 0.18).max(3.0);
        assert!(
            st_after.n_tier4 >= ref4,
            "Tier 4 count {} should stay above refuge floor {}",
            st_after.n_tier4, ref4
        );
        assert!(
            st_after.n_tier3 >= ref3,
            "Tier 3 count {} should stay above refuge floor {}",
            st_after.n_tier3, ref3
        );

        // Dynamic omen reflects apex vacancy in natural world prose
        let omen = dynamic_hunting_zone_ecology_omen("長風草甸");
        assert!(omen.is_some());
        assert!(omen.unwrap().contains("久未見巨獸蹤影"));
        // Test cleanup: restore apex_presence and n_tier2 for zone 0
        {
            RECENT_HUNT_KILL_SITES.write().remove(&(cx + 1, cy - 1));
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].apex_presence = 1.0;
            web[z_idx].n_tier2 = 6.0;
            web[z_idx].n_tier3 = web[z_idx].k_tier3;
            web[z_idx].n_tier4 = web[z_idx].k_tier4;
        }
    }

    #[test]
    fn test_detritus_recycling_and_flora_grazing_loop() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let z_idx = 1; // 古河巨木林
        let (_zone_name, cx, cy) = CANONICAL_HUNTING_ZONES[z_idx];
        let now = crate::game::now_unix();

        let old_time = now - 200;
        record_wilderness_kill_at_coord(
            cx,
            cy,
            &format!("shanhai_{z_idx}_1_{cx}_{cy}"),
            "錢紋石虎",
            "wt_cat_pelt",
            "石虎毛皮",
            "獵手",
            old_time,
        );

        {
            let sites = RECENT_HUNT_KILL_SITES.read();
            assert!(sites.contains_key(&(cx, cy)));
        }

        let detritus_before = get_zone_trophic_state(z_idx).unwrap().detritus_s;

        // Step trophic web: expired kill site should be swept and boost detritus_s by +18.0
        step_zone_trophic_web_once(z_idx);

        {
            let sites = RECENT_HUNT_KILL_SITES.read();
            assert!(!sites.contains_key(&(cx, cy)), "Expired kill site should be removed");
        }

        let st_stepped = get_zone_trophic_state(z_idx).unwrap();
        assert!(
            st_stepped.detritus_s > detritus_before,
            "Detritus should increase after kill site recycling"
        );

        // Set detritus_s >= 120.0 to verify fungi spawning and harvesting
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].detritus_s = 135.0;
        }

        let mut target_coord = None;
        for dy in -5..=5 {
            for dx in -5..=5 {
                let fx = cx + dx;
                let fy = cy + dy;
                if ((fx as i64 * 17389) ^ (fy as i64 * 49157)).unsigned_abs() % 8 == 0 {
                    target_coord = Some((fx, fy));
                    break;
                }
            }
            if target_coord.is_some() {
                break;
            }
        }
        let (fx, fy) = target_coord.expect("Found matching fungi coordinate");

        let view = get_grid_room_view("test_hunter", fx, fy, 12).unwrap().unwrap();
        let has_fungi = view.objects.iter().any(|o| o.id == format!("detritus_fungi_{fx}_{fy}"));
        assert!(has_fungi, "RoomView at ({fx}, {fy}) should contain detritus_fungi");

        let harvested = harvest_zone_detritus_fungi(fx, fy);
        assert!(harvested.is_some(), "Fungi should be harvestable when detritus_s >= 120.0");
        let (item_id, item_name) = harvested.unwrap();
        assert_eq!(item_id, "wt_cat_0064");
        assert_eq!(item_name, "深山牛樟芝");

        // Flora harvesting
        let harvest_units = harvest_hunting_zone_flora("古河巨木林", 25.0);
        assert_eq!(harvest_units, 25.0);

        // Test cleanup: restore detritus_s for zone 1
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].detritus_s = 100.0;
        }
    }

    #[test]
    fn test_spatial_taxis_and_starving_predator_road_spillover() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let z_idx = 2; // 赤石坡
        let (_zone_name, cx, cy) = CANONICAL_HUNTING_ZONES[z_idx];

        // 1. Spillover activation predicate test
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].apex_presence = 1.0;
            web[z_idx].n_tier2 = 6.0;
            web[z_idx].n_tier3 = web[z_idx].k_tier3;
        }
        assert!(!is_zone_predator_spillover_active(z_idx));
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].apex_presence = 0.0;
            web[z_idx].n_tier2 = 7.0; // > 6.0 * 1.08 = 6.48
        }
        assert!(is_zone_predator_spillover_active(z_idx));

        // 2. Spillover slot appearance in live_zone_shanhai_slots
        let slots = live_zone_shanhai_slots(z_idx, 100);
        let spillover_slot = slots
            .iter()
            .find(|s| s.tier_tag == "__tier2_overflow__");
        assert!(
            spillover_slot.is_some(),
            "Overflow slot should appear when spillover is active"
        );
        let sp = spillover_slot.unwrap();
        assert_eq!(sp.sub_geo, "古道外圍");
        assert!(
            sp.act_prefix.contains("因山中獵物匱乏而越界下山"),
            "Expected outskirt spillover prefix, got: {}",
            sp.act_prefix
        );

        // 3. Carcass taxis
        let (pred_bx, pred_by) = slots
            .iter()
            .find(|s| s.tier == 2 && s.tier_tag == "__tier2__")
            .map(|s| (s.base_x, s.base_y))
            .unwrap_or((cx, cy));
        let kill_site_x = pred_bx + 4;
        let kill_site_y = pred_by + 4;
        let now = crate::game::now_unix();
        {
            let mut sites = RECENT_HUNT_KILL_SITES.write();
            sites.insert(
                (kill_site_x, kill_site_y),
                RecentHuntKillSite {
                    prey_name: "林間野鹿".to_string(),
                    loot_id: "wt_cat_0203".to_string(),
                    loot_name: "野鹿肉".to_string(),
                    hunter_label: "老獵人".to_string(),
                    killed_at: now,
                },
            );
            // Restore apex so spillover is inactive
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].apex_presence = 1.0;
            web[z_idx].n_tier2 = 6.0;
        }

        let slots_carcass = live_zone_shanhai_slots(z_idx, 100);
        let carcass_feeder = slots_carcass
            .iter()
            .find(|s| s.tier == 2 && s.act_prefix.contains("循著林間血氣聚攏於殘骸周遭爭食"));
        assert!(
            carcass_feeder.is_some(),
            "Should have a predator displaced toward the carcass"
        );

        // Cleanup
        {
            RECENT_HUNT_KILL_SITES.write().remove(&(kill_site_x, kill_site_y));
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].apex_presence = 1.0;
            web[z_idx].n_tier2 = 6.0;
            web[z_idx].n_tier3 = web[z_idx].k_tier3;
            web[z_idx].n_tier4 = web[z_idx].k_tier4;
        }
    }

    #[test]
    fn test_climate_tidal_coupling_and_downhill_migration() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // 1. Geographic nearest neighbors
        let neighbors_0 = nearest_geographic_zones(0);
        assert_ne!(neighbors_0[0], 0);
        assert_ne!(neighbors_0[1], 0);
        assert_ne!(neighbors_0[0], neighbors_0[1]);

        // 2. Coastal and alpine predicate tests
        assert!(is_coastal_hunting_zone(3));
        assert!(is_coastal_hunting_zone(4));
        assert!(is_coastal_hunting_zone(5));
        assert!(is_coastal_hunting_zone(6));
        assert!(is_coastal_hunting_zone(9));
        assert!(is_coastal_hunting_zone(16));
        assert!(!is_coastal_hunting_zone(0));
        assert!(!is_coastal_hunting_zone(18));

        assert!(is_alpine_hunting_zone(14));
        assert!(is_alpine_hunting_zone(17));
        assert!(is_alpine_hunting_zone(18));
        assert!(!is_alpine_hunting_zone(0));
        assert!(!is_alpine_hunting_zone(3));

        // 3. Deterministic weather & tide
        let (w0, t0) = current_zone_weather_and_tide(0, 0);
        assert_eq!(w0, ZoneWeatherPhase::RainMist);
        assert_eq!(t0, ZoneTidePhase::FloodTide);

        let (w1, t1) = current_zone_weather_and_tide(0, 1);
        assert_eq!(w1, ZoneWeatherPhase::ClearBreeze);
        assert_eq!(t1, ZoneTidePhase::EbbTide);

        let (w3, t3) = current_zone_weather_and_tide(0, 3);
        assert_eq!(w3, ZoneWeatherPhase::ColdFrost);
        assert_eq!(t3, ZoneTidePhase::FloodTide);

        // 4. Weather & Tide coupling in trophic step
        let alpine_idx = 18; // 白塔雪巔
        let mid_idx = 0; // 長風草甸
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[alpine_idx].n_tier3 = web[alpine_idx].k_tier3;
            web[mid_idx].n_tier3 = web[mid_idx].k_tier3;
        }

        let alpine_tick = 1;
        assert_eq!(current_zone_weather_and_tide(alpine_idx, alpine_tick).0, ZoneWeatherPhase::ColdFrost);

        let pre_t3 = get_zone_trophic_state(alpine_idx).unwrap().n_tier3;
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            let [left_idx, right_idx] = nearest_geographic_zones(alpine_idx);
            let n4_left = web[left_idx].n_tier4;
            let n3_left = web[left_idx].n_tier3;
            let n2_left = web[left_idx].n_tier2;
            let n4_right = web[right_idx].n_tier4;
            let n3_right = web[right_idx].n_tier3;
            let n2_right = web[right_idx].n_tier2;
            step_zone_equations_internal(
                &mut web[alpine_idx],
                n4_left,
                n3_left,
                n2_left,
                n4_right,
                n3_right,
                n2_right,
                alpine_tick,
            );
        }
        let post_t3 = get_zone_trophic_state(alpine_idx).unwrap().n_tier3;
        assert!(post_t3 <= pre_t3 * 1.05);

        // Cleanup
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[alpine_idx].n_tier3 = web[alpine_idx].k_tier3;
            web[mid_idx].n_tier3 = web[mid_idx].k_tier3;
        }
    }

    #[test]
    fn test_gordon_schaefer_resource_pools_and_fallow_rotation() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let z_idx = 0; // 長風草甸
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].timber_w = 1000.0;
            web[z_idx].mineral_m = 1000.0;
            web[z_idx].fishery_f = 1000.0;
            web[z_idx].resource_fallow_mode = false;
        }

        // 1. Harvest when abundant
        let (actual, abundant, is_fallow) = harvest_zone_natural_resource("長風草甸", "timber", 3.0);
        assert_eq!(actual, 3.0);
        assert!(abundant, "Pool was at 1000.0, so abundant bonus should be true");
        assert!(!is_fallow);

        // 2. Trigger fallow mode by depleting pool below 350.0
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].timber_w = 340.0;
            web[z_idx].resource_fallow_mode = true;
        }

        assert_eq!(is_zone_in_fallow_stewardship("長風草甸"), Some("林木"));

        // 3. Harvesting while in fallow mode returns (0.0, false, true) and restores pool by +6.0
        let (fallow_harvest, fallow_abundant, fallow_restored) =
            harvest_zone_natural_resource("長風草甸", "timber", 3.0);
        assert_eq!(fallow_harvest, 0.0);
        assert!(!fallow_abundant);
        assert!(fallow_restored);
        let restored_w = get_zone_trophic_state(z_idx).unwrap().timber_w;
        assert_eq!(restored_w, 346.0);

        // 4. Artisan waypoint rotation during fallow mode
        let ((_px, _py), rotated_zone, _sub_geo) =
            pick_zone_artisan_waypoint_with_zone("長風草甸", 0, (0, 0));
        assert_ne!(
            rotated_zone, "長風草甸",
            "Artisan waypoint should rotate to neighbor zone during fallow mode on even seed"
        );

        // 5. Recovery clears fallow mode when all pools >= 500.0
        {
            let mut web = LIVE_ZONE_TROPHIC_WEB.write();
            web[z_idx].timber_w = 550.0;
            web[z_idx].mineral_m = 550.0;
            web[z_idx].fishery_f = 550.0;
            web[z_idx].resource_fallow_mode = false;
        }
        assert!(is_zone_in_fallow_stewardship("長風草甸").is_none());
    }

    #[test]
    fn no_design_jargon_in_hunting_zone_frontend_strings() {
        let _guard = TEST_TROPHIC_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        const FORBIDDEN: &[&str] = &[
            "五階平衡",
            "食物鏈運轉",
            "營養級聯",
            "霸主空窗",
            "霸主退位",
            "次級掠食",
            "鎮場霸主",
            "尋常中型獸群",
            "尋常小怪",
            "格內顯形",
            "尋跡顯形",
            "獵獲物質循環",
            "食腐分解閉環",
            "植被緊縮",
            "腐殖養分",
            "【山海異兆】",
            "\u{525b}\u{624d}",
            "\u{525b}\u{525b}",
            "\u{65b9}\u{624d}",
        ];
        for (z_idx, &(zone_name, _, _)) in CANONICAL_HUNTING_ZONES.iter().enumerate() {
            let omen = dynamic_hunting_zone_ecology_omen(zone_name).unwrap_or_default();
            for &bad in FORBIDDEN {
                assert!(
                    !omen.contains(bad),
                    "Zone {zone_name} dynamic omen leaked forbidden design term '{bad}': {omen}"
                );
            }
            let slots = live_zone_shanhai_slots(z_idx, 100);
            for sl in slots {
                for &bad in FORBIDDEN {
                    assert!(
                        !sl.tier_label.contains(bad),
                        "Zone {zone_name} slot tier_label leaked '{bad}': {}",
                        sl.tier_label
                    );
                    assert!(
                        !sl.act_prefix.contains(bad),
                        "Zone {zone_name} slot act_prefix leaked '{bad}': {}",
                        sl.act_prefix
                    );
                }
            }
        }

        for tick in 0..12 {
            let mut st = get_zone_trophic_state(0).unwrap();
            let [left_idx, right_idx] = nearest_geographic_zones(0);
            let l = get_zone_trophic_state(left_idx).unwrap();
            let r = get_zone_trophic_state(right_idx).unwrap();
            step_zone_equations_internal(
                &mut st,
                l.n_tier4,
                l.n_tier3,
                l.n_tier2,
                r.n_tier4,
                r.n_tier3,
                r.n_tier2,
                tick,
            );
            for &bad in FORBIDDEN {
                assert!(
                    !st.recent_trophic_event.contains(bad),
                    "Trophic event leaked '{bad}': {}",
                    st.recent_trophic_event
                );
            }
        }
    }
}

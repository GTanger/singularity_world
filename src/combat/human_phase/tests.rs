
// ==========================================
// 單元測試（涵蓋五大模組與代數守恆）
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::wordplate::{CarrierAffinity, PostureState, TacticalSlot, WordRegistry};

    #[test]
    fn test_module1_algebraic_conservation() {
        for seed in [12345i64, 20261028, 999999, -42, 777] {
            let bone = InnateBonePhase::from_seed(seed);

            // 1. 對數空間功率守恆
            assert!(bone.alpha >= 0.5 && bone.alpha <= 2.0);
            assert!(bone.beta >= 0.5 && bone.beta <= 2.0);
            let p_mult = bone.alpha * bone.beta;
            assert!(
                p_mult >= 0.860 && p_mult <= 1.165,
                "power product {p_mult} out of bounds"
            );

            // 2. 15 區運動鏈阻抗矩陣總和守恆
            let mut sum_r = 0.0f64;
            for u in 0..3 {
                for v in 0..5 {
                    let r = bone.r_matrix[u][v];
                    assert!(r >= 299.99 && r <= 1200.01, "r_val {r} out of bounds");
                    sum_r += r;
                }
            }
            assert!(
                (sum_r - 10000.0).abs() < 1e-4,
                "R sum {sum_r} != 10000.0"
            );

            // 3. 外顯三維守恆
            let total_stats = bone.vit_0 + bone.qi_0 + bone.dex_0;
            assert!(
                (total_stats - 310.0).abs() < 1e-4,
                "Stats sum {total_stats} != 310.0"
            );

            // 4. 順暢度界限
            for u in 0..3 {
                for v in 0..5 {
                    let s = bone.cell_smoothness(u, v);
                    assert!(s > 0.25 && s < 1.75);
                }
            }

            // 5. 白話文字不可包含阿拉伯數字或禁詞
            let arch_title = bone.bone_archetype_title();
            let desc_zh = bone.describe_bone_phase_zh();
            let five_zh = bone.describe_five_stage_fluency_zh();
            let auto_zh = bone.auto_trigger_summary_zh();

            for s in [&arch_title, &desc_zh, &five_zh, &auto_zh] {
                assert!(!s.contains("還有"));
                assert!(!s.contains("資料庫"));
                assert!(!s.contains("110V"));
                assert!(!s.contains("念場"));
                assert!(!s.chars().any(|c| c.is_ascii_digit()));
            }
        }
    }

    #[test]
    fn test_module2_and_3_viterbi_emergence() {
        let registry = WordRegistry::global();
        let bone = InnateBonePhase::from_seed(20261028);

        let routine = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Opening,
            CarrierAffinity::Blade,
            "青鋼長劍",
            0.5,
            10,
            registry,
        );

        assert_eq!(routine.domain, "martial");
        assert_eq!(routine.carrier, CarrierAffinity::Blade);
        assert_eq!(routine.main_motions.len(), 5);
        assert!(!routine.turn_branch_motion.is_empty());
        assert!(routine.continuity_c > 0.0);
        assert!(routine.max_gap_ticks >= 1);
        assert!(!routine.omp_roots.0.is_empty());
        assert!(!routine.omp_roots.1.is_empty());
        assert!(!routine.formal_name.is_empty());
    }

    #[test]
    fn test_module4_three_phase_eval() {
        let bone_heavy = InnateBonePhase::from_seed(9999);
        let bone_light = InnateBonePhase::from_seed(8888);

        // 跨量級硬撼：120kg 巨錘 vs 40kg 匕首
        let res_crush = evaluate_three_phase_bout(
            TacticalSlot::Opening,
            &bone_heavy,
            &bone_light,
            120.0,
            40.0,
            CarrierAffinity::Blunt,
            CarrierAffinity::Blade,
            PostureState::Steady,
            PostureState::Steady,
            5,
            5,
        );

        match res_crush.verdict {
            ThreePhaseVerdict::Phase1ImpulseCrush { winner_is_a, .. } => {
                assert!(winner_is_a);
                assert_eq!(res_crush.b_completed_motions, 1);
                assert!(!res_crush.b_manifested_as_skill);
                assert_eq!(res_crush.b_posture_after, PostureState::Shaken);
            }
            _ => panic!("Expected Phase1ImpulseCrush"),
        }

        // 庖丁解牛穿隙
        let seam = evaluate_seam_threading(
            &bone_light,
            CarrierAffinity::Blade,
            120.0,
            80.0,
            25,
        );
        assert!(seam.xi_index >= 0.0 && seam.xi_index <= 2.30);
        assert!(seam.hardness_bypass >= 0.0 && seam.hardness_bypass <= 0.92);
        assert!(seam.wear_multiplier >= 0.05);
    }

    #[test]
    fn test_determinism_soul_seed_and_limb_mass() {
        let seed1 = resolve_entity_soul_seed("entity_npc_tiger_99", None);
        let seed2 = resolve_entity_soul_seed("entity_npc_tiger_99", None);
        assert_eq!(seed1, seed2);
        assert_ne!(seed1, 0);
        assert_ne!(seed1, 100_003);

        let explicit = resolve_entity_soul_seed("entity_npc_tiger_99", Some(777));
        assert_eq!(explicit, 777);

        let bone = InnateBonePhase::from_seed(seed1);
        let m_unarmed = effective_carrier_mass(&bone, CarrierAffinity::Unarmed, 0.0);
        let m_blunt = effective_carrier_mass(&bone, CarrierAffinity::Blunt, 15.0);
        assert!(m_unarmed > 0.0);
        assert!(m_blunt > m_unarmed);
    }

    #[test]
    fn test_continuous_timings_and_read_adaptation() {
        let bone = InnateBonePhase::from_seed(42);
        let (tau_pre, tau_gap) =
            compute_continuous_timings(&bone, CarrierAffinity::Blade, 1.2, 0.6, 0.0);
        assert!(tau_pre >= 1.0);
        assert!(tau_gap >= 1.0);

        let r0 = read_adaptation_factor(0, 0.8, 0.5);
        assert_eq!(r0, 0.0);
        let r3 = read_adaptation_factor(3, 0.8, 0.5);
        assert!(r3 > 0.0 && r3 <= 0.75);

        let (pre_scar, gap_scar) =
            compute_continuous_timings(&bone, CarrierAffinity::Blade, 1.2, 0.6, 0.25);
        assert!(gap_scar > tau_gap);
        assert_eq!(pre_scar, tau_pre);
    }

    #[test]
    fn test_unbounded_practice_and_smoothness_scaling() {
        let p1 = practice_depth(25.0);
        assert!((p1 - 1.0).abs() < 1e-4);
        let p100 = practice_depth(100.0);
        assert!(p100 > p1);

        let budget1 = effective_time_budget(1000.0, 1, 0.2);
        let budget3 = effective_time_budget(1000.0, 3, 0.2);
        assert_eq!(budget1, 1000.0);
        assert!(budget3 < budget1);

        let decayed = apply_annual_idle_decay(100.0);
        assert!((decayed - 97.0).abs() < 1e-4);

        let delta_high_s = manual_comprehension_delta(1.2, 3.0);
        let delta_low_s = manual_comprehension_delta(0.4, 3.0);
        assert!(delta_high_s > delta_low_s);

        let h_high_s = mastery_factor_with_smoothness(0.5, 50, 1.5);
        let h_low_s = mastery_factor_with_smoothness(0.5, 50, 0.5);
        assert!(h_high_s > h_low_s);
    }

    #[test]
    fn test_seam_b_mirror_and_real_viterbi_coupling() {
        let bone_dexterous = InnateBonePhase::from_seed(8888);
        let _bone_stiff = InnateBonePhase::from_seed(9999);

        let (cont, var_v) = compute_viterbi_continuity_and_variation(
            &bone_dexterous,
            TacticalSlot::Midgame,
            CarrierAffinity::Blade,
            0.8,
            40,
        );
        assert!(cont > 0.0);
        assert!(var_v >= 0.40 && var_v <= 1.80);

        // 測試乙方 (defender) 穿隙勝出分支
        let res_seam_b = evaluate_three_phase_bout_ext(
            TacticalSlot::Midgame,
            &bone_dexterous,
            &bone_dexterous,
            60.0,
            60.0,
            CarrierAffinity::Blade,
            CarrierAffinity::Blade,
            PostureState::Shaken,
            PostureState::Steady,
            5,
            5,
            0,
            25,
            25,
            0,
            200,
            0.8,
            0.8,
            0.0,
            0.0,
        );

        match res_seam_b.verdict {
            ThreePhaseVerdict::Phase3SeamThreading { threader_is_a, xi_index, .. } => {
                assert!(!threader_is_a);
                assert!(xi_index >= 1.50);
                assert_eq!(res_seam_b.a_posture_after, PostureState::Broken);
                assert_eq!(res_seam_b.b_posture_after, PostureState::Steady);
                assert_eq!(res_seam_b.a_completed_motions, 2);
            }
            _ => panic!("Expected Phase3SeamThreading with threader_is_a == false, got {:?}", res_seam_b.verdict),
        }
    }

    #[test]
    fn test_module5_residual_omp_roots() {
        let registry = WordRegistry::global();
        let feat = [
            0.8, 0.2, 0.1, 0.4, 0.0, 0.0, 0.5, 0.1, 0.2, 0.3, 0.0, 0.0, 0.2, 0.7, 0.1, 0.0,
        ];

        let ((id1, id2), (ch1, ch2)) = residual_omp_select_roots(&feat, registry);
        assert_ne!(id1, id2);
        assert_ne!(ch1, ch2);
        assert!(!CLASSICAL_PARTICLES.contains(&ch1.as_str()));
        assert!(!CLASSICAL_PARTICLES.contains(&ch2.as_str()));
    }

    #[test]
    fn test_tactical_slots_diversity_and_craft_isolation() {
        let registry = WordRegistry::global();
        let bone = InnateBonePhase::from_seed(20261028);

        // 1. 驗證同一 bone 下，4 個 TacticalSlot 產生 4 個不同的 formal_name
        let r_open = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Opening,
            CarrierAffinity::Blade,
            "青鋼長劍",
            0.5,
            10,
            registry,
        );
        let r_mid = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Midgame,
            CarrierAffinity::Blade,
            "青鋼長劍",
            0.5,
            10,
            registry,
        );
        let r_fin = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Finisher,
            CarrierAffinity::Blade,
            "青鋼長劍",
            0.5,
            10,
            registry,
        );
        let r_def = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Defense,
            CarrierAffinity::Blade,
            "青鋼長劍",
            0.5,
            10,
            registry,
        );

        let names = [
            &r_open.formal_name,
            &r_mid.formal_name,
            &r_fin.formal_name,
            &r_def.formal_name,
        ];
        let mut unique_names = std::collections::HashSet::new();
        for name in names {
            unique_names.insert(name);
        }
        assert_eq!(unique_names.len(), 4, "Expected 4 distinct names for 4 slots, got: {:?}", names);

        // 2. 驗證 Blade 武相動作中絕不混入百工動作
        let catalog = build_elementary_motion_catalog();
        let craft_names: std::collections::HashSet<String> = catalog
            .iter()
            .filter(|m| m.craft_domain.is_some())
            .map(|m| m.name.clone())
            .collect();

        for routine in [&r_open, &r_mid, &r_fin, &r_def] {
            for m_name in &routine.main_motions {
                assert!(
                    !craft_names.contains(m_name),
                    "Martial routine contains craft motion: {m_name}"
                );
            }
            assert!(!routine.learned_and_familiar_zh.contains("\\to"));
        }

        // 3. 驗證 wayfare 行旅藝相有 5 個各不相同的階段碎招，且零 "\\to"
        let r_wayfare = derive_personal_craft_routine(
            &bone,
            "wayfare",
            CarrierAffinity::Footwork,
            "羅盤與行囊",
            10,
            10,
            registry,
        );
        let mut wayfare_motion_set = std::collections::HashSet::new();
        for m_name in &r_wayfare.main_motions {
            wayfare_motion_set.insert(m_name);
        }
        assert_eq!(
            wayfare_motion_set.len(),
            5,
            "Wayfare craft routine must have 5 distinct stage motions, got: {:?}",
            r_wayfare.main_motions
        );
        assert!(!r_wayfare.learned_and_familiar_zh.contains("\\to"));
        assert!(!r_wayfare.auto_trigger_desc_zh.contains("\\to"));
    }
}

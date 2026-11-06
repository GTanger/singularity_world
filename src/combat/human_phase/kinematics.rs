
impl InnateBonePhase {
    /// 依唯一 soul_seed 初始化確定性骨相
    pub fn from_seed(seed: i64) -> Self {
        let mut rng_state = seed as u64;

        // 輔助函式：自 splitmix64 取 [0.0, 1.0) 浮點數
        let next_f64 = |state: &mut u64| -> f64 {
            let val = crate::combat::wordplate::splitmix64(*state);
            *state = state.wrapping_add(0x9E3779B97F4A7C15);
            (val >> 11) as f64 / ((1u64 << 53) as f64)
        };

        let ln_2 = 2.0f64.ln();
        let u_alpha = next_f64(&mut rng_state);
        let ln_alpha = -ln_2 + u_alpha * (2.0 * ln_2);

        let u_eps = next_f64(&mut rng_state);
        let eps = -0.15 + u_eps * 0.30;

        let raw_ln_beta = -ln_alpha + eps;
        let ln_beta = raw_ln_beta.clamp(-ln_2, ln_2);

        let u_gamma = next_f64(&mut rng_state);
        let gamma = -1.0 + u_gamma * 2.0;

        let alpha = ln_alpha.exp();
        let beta = ln_beta.exp();

        // 抽樣 15 個 Gamma(18, 1) 變量
        let mut r_flat = [0.0f64; 15];
        let mut sum_raw = 0.0f64;
        for i in 0..15 {
            let mut g_val = 0.0f64;
            for _ in 0..18 {
                let u = next_f64(&mut rng_state).clamp(1e-12, 1.0);
                g_val += -u.ln();
            }
            r_flat[i] = g_val;
            sum_raw += g_val;
        }

        if sum_raw <= 0.0 {
            sum_raw = 1.0;
        }
        for i in 0..15 {
            r_flat[i] = (r_flat[i] / sum_raw) * 10000.0;
        }

        // 帶界單純形水位投影 (Bounded Simplex Water-Filling Projection) 12 次迭代
        for _ in 0..12 {
            for v in r_flat.iter_mut() {
                *v = v.clamp(300.0, 1200.0);
            }
            let cur_sum: f64 = r_flat.iter().sum();
            let deficit = 10000.0 - cur_sum;
            let mut free_sum = 0.0f64;
            let mut free_mask = [false; 15];
            for i in 0..15 {
                let val = r_flat[i];
                if (deficit > 0.0 && val < 1199.99) || (deficit < 0.0 && val > 300.01) {
                    free_mask[i] = true;
                    free_sum += 1.0;
                }
            }
            if free_sum > 0.0 {
                let step = deficit / free_sum;
                for i in 0..15 {
                    if free_mask[i] {
                        r_flat[i] += step;
                    }
                }
            } else {
                break;
            }
        }

        // 最後確切 clamp [300.0, 1200.0] 與精確再歸一化，防止浮點微小誤差飄移
        for v in r_flat.iter_mut() {
            *v = v.clamp(300.0, 1200.0);
        }
        let final_sum: f64 = r_flat.iter().sum();
        if final_sum > 0.0 && (final_sum - 10000.0).abs() > 1e-9 {
            let factor = 10000.0 / final_sum;
            for v in r_flat.iter_mut() {
                *v = (*v * factor).clamp(300.0, 1200.0);
            }
            let post_sum: f64 = r_flat.iter().sum();
            let diff = 10000.0 - post_sum;
            r_flat[0] = (r_flat[0] + diff).clamp(300.0, 1200.0);
        }

        let mut r_matrix = [[0.0f64; 5]; 3];
        for u in 0..3 {
            for v in 0..5 {
                r_matrix[u][v] = r_flat[u * 5 + v];
            }
        }

        // 外顯三維計算
        let sum_vit: f64 = r_matrix[0].iter().sum();
        let sum_qi: f64 = r_matrix[1].iter().sum();
        let sum_dex: f64 = r_matrix[2].iter().sum();

        let vit_0 = 50.0 + 0.02 * (6000.0 - sum_vit);
        let qi_0 = 50.0 + 0.02 * (6000.0 - sum_qi);
        let dex_0 = 50.0 + 0.02 * (6000.0 - sum_dex);

        Self {
            seed,
            alpha,
            beta,
            gamma,
            r_matrix,
            vit_0,
            qi_0,
            dex_0,
        }
    }

    /// 爆發力矩放大係數
    pub fn impact_factor(&self) -> f64 {
        self.alpha.sqrt()
    }

    /// 有界個人身體順暢因子 S_e(u, v) in (0.25, 1.75)
    pub fn cell_smoothness(&self, axis: usize, stage: usize) -> f64 {
        let r_val = self.r_matrix[axis][stage];
        0.25 + 1.50 / (1.0 + ((r_val - R_BAR) / TAU_R).exp())
    }

    /// 該發力階段阻抗最低（最順暢）之主幹屬性 (0:體, 1:氣, 2:敏)
    pub fn stage_best_axis(&self, stage: usize) -> usize {
        let mut best_axis = 0;
        let mut min_r = self.r_matrix[0][stage];
        for axis in 1..3 {
            if self.r_matrix[axis][stage] < min_r {
                min_r = self.r_matrix[axis][stage];
                best_axis = axis;
            }
        }
        best_axis
    }

    /// 階段發力流暢度評語
    pub fn stage_fluency_label(&self, stage: usize) -> &'static str {
        let best_s = self.cell_smoothness(self.stage_best_axis(stage), stage);
        if best_s >= 1.30 {
            "渾然天成"
        } else if best_s >= 1.00 {
            "順暢自如"
        } else if best_s >= 0.70 {
            "平穩守中"
        } else {
            "略需借勢"
        }
    }

    /// 天生體魄稱號（零數字、純正體白話）
    pub fn bone_archetype_title(&self) -> String {
        let prefix = if self.alpha >= 1.35 {
            "沉腰重岳"
        } else if self.alpha <= 0.75 {
            "輕靈矯捷"
        } else {
            "骨肉勻稱"
        };

        let suffix = if self.gamma >= 0.35 {
            "氣脈沉厚"
        } else if self.gamma <= -0.35 {
            "氣機靈動"
        } else {
            "氣息平和"
        };

        format!("{prefix}・{suffix}")
    }
    /// 完整白話骨相描述（零數字、零 N001/G01）
    pub fn describe_bone_phase_zh(&self) -> String {
        let power_desc = if self.alpha >= 1.25 {
            "天生筋骨粗壯，下盤沉實，落步發力帶有重岳崩山之勢；出招雖稍顯沉穩，但每一擊皆力透筋骨。"
        } else if self.alpha <= 0.80 {
            "天生筋絡纖長，動靜靈動輕捷，呼吸換氣與步履銜接極為迅疾；雖難以蠻力撼動重甲，但出招連綿如穿花引線。"
        } else {
            "氣血與筋骨協調勻稱，發力剛柔並濟，既能正面立足抗衡，亦能從容調息換勢。"
        };

        let temperament_desc = if self.gamma >= 0.25 {
            "其人氣息沉穩厚實，舉止遵循法度，最善於在長線拆解之中尋找對手的破綻與銜接空隙。"
        } else if self.gamma <= -0.25 {
            "其人性喜靈動求變，氣機周流不滯，遭遇強敵沉重壓制時極擅長側身偏轉、借勢卸力求生。"
        } else {
            "周身氣脈平穩中和，守勢與攻勢流轉隨心，不偏執於剛猛亦不滯於巧變。"
        };

        let stage_names = ["起勢", "承接", "轉折", "協調", "合擊"];
        let axis_names = ["筋骨發力", "內息吐納", "手腕步法"];
        let mut min_r = f64::MAX;
        let mut max_r = f64::MIN;
        let mut best_pair = (0usize, 0usize);
        let mut tight_pair = (0usize, 2usize);
        for u in 0..3 {
            for v in 0..5 {
                let r = self.r_matrix[u][v];
                if r < min_r {
                    min_r = r;
                    best_pair = (u, v);
                }
                if r > max_r {
                    max_r = r;
                    tight_pair = (u, v);
                }
            }
        }
        let chain_desc = format!(
            "交手發力之際，尤以{}時之{}最為順暢；若遇{}時{}稍滯，身形亦能自然順勢卸轉、借力換步。",
            stage_names[best_pair.1],
            axis_names[best_pair.0],
            stage_names[tight_pair.1],
            axis_names[tight_pair.0],
        );

        format!("{power_desc}{temperament_desc}{chain_desc}")
    }

    /// 五常發力順暢白話說明
    pub fn describe_five_stage_fluency_zh(&self) -> String {
        let labels = [
            self.stage_fluency_label(0),
            self.stage_fluency_label(1),
            self.stage_fluency_label(2),
            self.stage_fluency_label(3),
            self.stage_fluency_label(4),
        ];

        let axes_names = ["筋骨體魄", "氣血周流", "靈敏身形"];
        let best_0 = axes_names[self.stage_best_axis(0)];
        let best_2 = axes_names[self.stage_best_axis(2)];
        let best_4 = axes_names[self.stage_best_axis(4)];

        format!(
            "出招節奏自起手之{}、中盤承轉之{}，一路延續至收勢之{}；起手憑{}爭先，中盤仗{}周旋，收勢以{}定局。",
            labels[0], labels[2], labels[4], best_0, best_2, best_4
        )
    }

    /// 三相自發適配傾向總結（白話）
    pub fn auto_trigger_summary_zh(&self) -> String {
        let crush_prop = if self.alpha >= 1.15 {
            "正面交鋒時佔盡千鈞優勢，極易以渾厚衝量逼散敵方架勢；"
        } else {
            "正面硬撼需審慎借器物分量立足，不宜與沉猛之勢直面死磕；"
        };

        let intercept_prop = if self.gamma >= 0.15 {
            "對換招縫隙與對手破綻極度敏銳，擅長以截擊瓦解連招；"
        } else {
            "遭遇交鋒多以變招游移周旋，借勢轉向以尋良機；"
        };

        let threading_prop = if self.dex_0 >= 100.0 || self.beta >= 1.10 {
            "刃口落點細微精準，在百工解剖與實戰破綻切入時皆能直尋骨隙筋節。"
        } else {
            "出手偏重連貫平穩，講究力道透徹而非純粹求巧。"
        };

        format!("{crush_prop}{intercept_prop}{threading_prop}")
    }
}

// ==========================================
// 模組二：碎招運動學庫與物理計算
// ==========================================

/// 四維姿態距離平方
pub fn posture_distance_sq(p1: &[f64; 4], p2: &[f64; 4]) -> f64 {
    let dz = p1[0] - p2[0];
    let d_omega = p1[1] - p2[1];
    let dv = p1[2] - p2[2];
    let d_theta = p1[3] - p2[3];

    let cos_val = d_theta.cos();
    let theta_term = 2.0 * (1.0 - cos_val).clamp(0.0, 2.0);

    // 等量綱權重均為 1.0
    dz * dz + d_omega * d_omega + dv * dv + theta_term
}

/// 非交換運動學匹配核 K(m_i -> m_{i+1}) in (0, 1]
pub fn kinematic_kernel(m1: &ElementaryMotionDef, m2: &ElementaryMotionDef) -> f64 {
    // 強制起 -> 承 -> 轉 -> 協 -> 合 階段推進
    if m2.stage != m1.stage + 1 {
        return 0.0;
    }

    let self_penalty = if m1.name == m2.name { RHO_SELF } else { 1.0 };
    let d_sq = posture_distance_sq(&m1.p_out, &m2.p_in);
    let exp_term = (-d_sq / (2.0 * SIGMA_P_SQ)).exp();

    self_penalty * exp_term
}

/// 個人身體順暢因子 S_e(m)
pub fn motion_smoothness(bone: &InnateBonePhase, motion: &ElementaryMotionDef) -> f64 {
    let stage = motion.stage.min(4);
    let r_eff = motion.w_3d[0] * bone.r_matrix[0][stage]
        + motion.w_3d[1] * bone.r_matrix[1][stage]
        + motion.w_3d[2] * bone.r_matrix[2][stage];

    0.25 + 1.50 / (1.0 + ((r_eff - R_BAR) / TAU_R).exp())
}

/// 無上限練習冪律深度 m_e(f_eff) = (f_eff / F_0)^0.45
pub fn practice_depth(effective_f: f64) -> f64 {
    (effective_f.max(0.0) / F_0).powf(P_PRACTICE)
}

/// 年度有效時間預算分流：隨修習門數廣度單調遞減
pub fn effective_time_budget(total_time: f64, active_routines: usize, switch_cost: f64) -> f64 {
    let penalty = switch_cost.max(0.0) * (active_routines.saturating_sub(1) as f64);
    total_time.max(0.0) / (1.0 + penalty)
}

/// 閒置用進廢退衰減：每年按固定比例衰減練習量
pub fn apply_annual_idle_decay(f: f64) -> f64 {
    f.max(0.0) * (1.0 - ANNUAL_IDLE_DECAY)
}

/// 手札研讀領悟增量 Delta L = 1.0 - exp(-s * units / 3.0)
pub fn manual_comprehension_delta(routine_smoothness: f64, study_units: f64) -> f64 {
    1.0 - (-routine_smoothness.max(0.25) * study_units.max(0.0) / 3.0).exp()
}

/// 結合骨相順暢度之無上限熟練因子 H_e（順性放大練習深度，拔除人為封頂）
pub fn mastery_factor_with_smoothness(
    learned_delta: f64,
    familiarity_count: u32,
    smoothness_s: f64,
) -> f64 {
    let f_eff = smoothness_s.max(0.25) * (familiarity_count as f64);
    1.0 + ETA_L * learned_delta.clamp(0.0, 1.0) + ETA_F_POWER * practice_depth(f_eff)
}

/// 後天所學與熟練因子 H_e（預設順暢度 1.0）
pub fn mastery_factor(learned_delta: f64, familiarity_count: u32) -> f64 {
    mastery_factor_with_smoothness(learned_delta, familiarity_count, 1.0)
}

/// 綜合銜接流暢度 C_e(m_i -> m_{i+1})
pub fn transition_continuity(
    bone: &InnateBonePhase,
    m1: &ElementaryMotionDef,
    m2: &ElementaryMotionDef,
    learned_delta: f64,
    familiarity_count: u32,
) -> f64 {
    let k = kinematic_kernel(m1, m2);
    if k <= 0.0 {
        return 0.0;
    }
    let s = motion_smoothness(bone, m2);
    let h = mastery_factor_with_smoothness(learned_delta, familiarity_count, s);

    k * s * h
}
/// 換招空檔破綻窗 tau^gap in Ticks (>= 1)
pub fn transition_gap_ticks(
    bone: &InnateBonePhase,
    m1: &ElementaryMotionDef,
    m2: &ElementaryMotionDef,
    c_val: f64,
) -> u8 {
    let base = (m1.timing.2 as f64) + (m2.timing.0 as f64);
    let denom = 1.0 + 0.55 * bone.beta.sqrt() * c_val;
    let ticks = (base / denom).ceil() as u8;
    ticks.max(1)
}

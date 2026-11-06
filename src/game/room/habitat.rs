/// 正典 19 座山海獵場與其深處地脈核心錨點。
pub const CANONICAL_HUNTING_ZONES: &[(&str, i32, i32)] = &[
    ("長風草甸", -180, -195),
    ("古河巨木林", -55, -80),
    ("赤石坡", -85, -180),
    ("落日岬", -320, -180),
    ("驚濤岩岸", -355, 115),
    ("巨藻淺灣", -280, 200),
    ("黑石灣", -280, 5),
    ("白茫洲", -180, -320),
    ("貼山深潭", 120, -260),
    ("淺澤泥口", 20, -180),
    ("青籠關", 0, -450),
    ("西麓林道", -120, 80),
    ("清泉溪谷", 60, 160),
    ("翠冠巨木林", -60, 220),
    ("北坡雲杉林", 80, 280),
    ("西嶺礦谷", 180, 40),
    ("東海峭壁", 320, 200),
    ("雲腰雪林", 180, 320),
    ("白塔雪巔", 0, 420),
];

/// 全島全域自然棲地生靈點位（消除人工同心圓，按 19 獵場真實疆域進行藍噪聲 Poisson-disc 與獸群聚落分布）。
#[derive(Debug, Clone)]
pub struct ShanhaiHabitatSlot {
    pub slot_idx: usize,
    pub base_x: i32,
    pub base_y: i32,
    pub x: i32,
    pub y: i32,
    pub sub_geo: &'static str,
    pub tier: u8,
    pub tier_tag: &'static str,
    pub kind_str: &'static str,
    pub tier_label: &'static str,
    pub act_prefix: &'static str,
}

pub fn is_inside_settlement_urban_core(x: i32, y: i32) -> bool {
    const SETTLEMENT_CORES: &[(i32, i32, i32)] = &[
        (0, 0, 20),         // 文曜 [-17..17]
        (-260, 160, 14),    // 風舶角 [-270..-250, 150..170]
        (-330, 160, 11),    // 風舶嶼 [-337..-323, 153..167]
        (-180, -140, 11),   // 大圳頭 [-187..-173, -147..-133]
        (160, 210, 11),     // 半嶺鎮 [153..167, 203..217]
        (0, -220, 11),      // 南哨 [-7..7, -227..-213]
        (0, 125, 9),        // 老堰頭 [-6..6, 119..131]
        (110, 115, 9),      // 北嶺腳 [104..116, 109..121]
        (190, 290, 9),      // 霜林口 [184..196, 284..296]
        (-360, 95, 9),      // 風櫃尾 [-366..-354, 89..101]
        (-260, 35, 9),      // 烏石鼻 [-266..-254, 29..41]
        (-180, -245, 9),    // 牛埔 [-186..-174, -250..-240]
        (-85, -140, 9),     // 紅土崁 [-91..-79, -146..-134]
        (-285, -140, 9),    // 三號坑口 [-291..-279, -146..-134]
        (-75, -280, 9),     // 水尾寮 [-81..-69, -286..-274]
    ];
    SETTLEMENT_CORES
        .iter()
        .any(|&(sx, sy, r)| (x - sx).abs() <= r && (y - sy).abs() <= r)
}

fn habitat_hash(a: i64, b: i64, c: i64) -> u64 {
    let mut h = (a as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((b as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((c as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

fn is_valid_wilderness_habitat_cell(x: i32, y: i32, expected_zone: &str) -> Option<&'static str> {
    if !(-392..=392).contains(&x) || !(-492..=492).contains(&y) {
        return None;
    }
    if is_inside_settlement_urban_core(x, y) {
        return None;
    }
    let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(x, y);
    if zone != expected_zone {
        return None;
    }
    if matches!(
        terrain,
        crate::grid::Terrain::WaterDeep | crate::grid::Terrain::Wall | crate::grid::Terrain::Water
    ) {
        return None;
    }
    Some(sub_geo)
}

type BaseHabitatEntry = (i32, i32, &'static str, u8, &'static str, usize);

static ZONE_HABITAT_LAYOUTS: std::sync::LazyLock<Vec<Vec<BaseHabitatEntry>>> =
    std::sync::LazyLock::new(|| {
        let mut zone_candidates: Vec<Vec<(i32, i32, &'static str)>> =
            vec![Vec::new(); CANONICAL_HUNTING_ZONES.len()];

        // 1. 掃描全島 800x1000 疆域（4 格抖動網格），按 classify_wilderness_cell 歸入 19 獵場真實版圖
        for by in (-488..=468).step_by(4) {
            for bx in (-388..=358).step_by(4) {
                let h = habitat_hash(bx as i64, by as i64, 17);
                let jx = ((h % 3) as i32) - 1;
                let jy = (((h >> 8) % 3) as i32) - 1;
                let gx = bx + jx;
                let gy = by + jy;
                if is_inside_settlement_urban_core(gx, gy) {
                    continue;
                }
                let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(gx, gy);
                if matches!(
                    terrain,
                    crate::grid::Terrain::WaterDeep
                        | crate::grid::Terrain::Wall
                        | crate::grid::Terrain::Water
                ) {
                    continue;
                }
                if let Some(z_idx) = CANONICAL_HUNTING_ZONES
                    .iter()
                    .position(|(zn, _, _)| *zn == zone)
                {
                    zone_candidates[z_idx].push((gx, gy, sub_geo));
                }
            }
        }

        // 針對風舶外島「驚濤岩岸」與狹長海岸帶加密採樣（2 格步距），確保外島與岬角棲地充沛
        for by in (-270..=260).step_by(2) {
            for bx in (-376..=-225).step_by(2) {
                if is_inside_settlement_urban_core(bx, by) {
                    continue;
                }
                let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(bx, by);
                if !matches!(zone, "驚濤岩岸" | "落日岬" | "巨藻淺灣" | "黑石灣") {
                    continue;
                }
                if matches!(
                    terrain,
                    crate::grid::Terrain::WaterDeep
                        | crate::grid::Terrain::Wall
                        | crate::grid::Terrain::Water
                ) {
                    continue;
                }
                if let Some(z_idx) = CANONICAL_HUNTING_ZONES
                    .iter()
                    .position(|(zn, _, _)| *zn == zone)
                {
                    if !zone_candidates[z_idx]
                        .iter()
                        .any(|&(ex, ey, _)| ex == bx && ey == by)
                    {
                        zone_candidates[z_idx].push((bx, by, sub_geo));
                    }
                }
            }
        }

        // 按 19 座獵場真實陸塊面積（候選棲地格數）動態配賦 Tier-3（共 380 隻）與 Tier-4（共 1,292 隻）名額，
        // 使廣袤大獵場（翠冠巨木林、西麓林道、西嶺礦谷、白茫洲等）腹地飽滿，狹小岬角不過度擁擠，總數嚴格維持 1,805 隻！
        let total_cands: usize = zone_candidates.iter().map(|v| v.len().max(1)).sum();
        let mut tier3_counts = vec![12usize; CANONICAL_HUNTING_ZONES.len()]; // 保底 12*19 = 228，餘 152 按面積分配
        let rem_t3 = 380usize.saturating_sub(12 * CANONICAL_HUNTING_ZONES.len());
        let mut assigned_t3 = 0usize;
        for z_idx in 0..CANONICAL_HUNTING_ZONES.len() {
            let add = (rem_t3 * zone_candidates[z_idx].len().max(1)) / total_cands.max(1);
            tier3_counts[z_idx] += add;
            assigned_t3 += add;
        }
        let mut order_by_size: Vec<usize> = (0..CANONICAL_HUNTING_ZONES.len()).collect();
        order_by_size.sort_by_key(|&i| std::cmp::Reverse(zone_candidates[i].len()));
        for i in 0..(rem_t3.saturating_sub(assigned_t3)) {
            tier3_counts[order_by_size[i % order_by_size.len()]] += 1;
        }

        let mut tier4_counts = vec![28usize; CANONICAL_HUNTING_ZONES.len()]; // 保底 28*19 = 532，餘 760 按面積分配
        let rem_t4 = 1292usize.saturating_sub(28 * CANONICAL_HUNTING_ZONES.len());
        let mut assigned_t4 = 0usize;
        for z_idx in 0..CANONICAL_HUNTING_ZONES.len() {
            let add = (rem_t4 * zone_candidates[z_idx].len().max(1)) / total_cands.max(1);
            tier4_counts[z_idx] += add;
            assigned_t4 += add;
        }
        for i in 0..(rem_t4.saturating_sub(assigned_t4)) {
            tier4_counts[order_by_size[i % order_by_size.len()]] += 1;
        }

        let mut layouts = Vec::with_capacity(CANONICAL_HUNTING_ZONES.len());
        let mut global_chosen: Vec<(i32, i32)> = Vec::with_capacity(1900);

        for (z_idx, &(zone_name, cx, cy)) in CANONICAL_HUNTING_ZONES.iter().enumerate() {
            let cands = &zone_candidates[z_idx];
            let t3_quota = tier3_counts[z_idx];
            let t4_quota = tier4_counts[z_idx];
            let mut entries: Vec<BaseHabitatEntry> = Vec::with_capacity(1 + 6 + t3_quota + t4_quota + 3);

            // 預先計算每個候選格距獵場邊界／海岸線／城鎮核心的邊界權重（3..=10），不封頂腹地距離，徹底消除大獵場中央空洞與邊界排隊
            let cand_boundary_weights: Vec<i64> = cands
                .iter()
                .map(|&(px, py, _)| {
                    const DIRS: &[(i32, i32)] = &[
                        (1, 0),
                        (-1, 0),
                        (0, 1),
                        (0, -1),
                        (1, 1),
                        (-1, 1),
                        (1, -1),
                        (-1, -1),
                    ];
                    for &(r, w) in &[(5i32, 3i64), (10, 6), (16, 8)] {
                        for &(dx, dy) in DIRS {
                            let tx = px + dx * r;
                            let ty = py + dy * r;
                            if is_inside_settlement_urban_core(tx, ty) {
                                return w;
                            }
                            let (tz, _, tt) =
                                crate::grid::r#gen::classify_wilderness_cell(tx, ty);
                            if tz != zone_name
                                || matches!(
                                    tt,
                                    crate::grid::Terrain::WaterDeep | crate::grid::Terrain::Wall
                                )
                            {
                                return w;
                            }
                        }
                    }
                    10i64
                })
                .collect();

            // (1) Tier 1 鎮場霸主（1 頭）：駐於獵場深處核心地脈，周圍 2 格設「百鳥屏息圈」
            let (apex_x, apex_y, apex_sub) = if let Some(sub) =
                is_valid_wilderness_habitat_cell(cx + 1, cy - 1, zone_name)
            {
                (cx + 1, cy - 1, sub)
            } else {
                cands
                    .iter()
                    .copied()
                    .min_by_key(|&(x, y, _)| (x - cx).pow(2) + (y - cy).pow(2))
                    .unwrap_or((cx + 1, cy - 1, "深谷"))
            };
            entries.push((apex_x, apex_y, apex_sub, 1, "__apex__", 0));

            let mut chosen_points: Vec<(i32, i32)> = vec![(apex_x, apex_y)];
            global_chosen.push((apex_x, apex_y));

            let pick_next_spread_point =
                |chosen: &mut Vec<(i32, i32)>,
                 all_chosen: &mut Vec<(i32, i32)>,
                 salt: i64|
                 -> (i32, i32, &'static str) {
                    let mut best = (cx + 5, cy - 5, "林地");
                    let mut best_score = -1i64;
                    for (c_idx, &(px, py, psub)) in cands.iter().enumerate() {
                        if (px - apex_x).abs() <= 2 && (py - apex_y).abs() <= 2 {
                            continue;
                        }
                        let min_local_d2 = chosen
                            .iter()
                            .map(|&(ox, oy)| {
                                let dx = (px - ox) as i64;
                                let dy = (py - oy) as i64;
                                dx * dx + dy * dy
                            })
                            .min()
                            .unwrap_or(0);
                        if min_local_d2 == 0 {
                            continue;
                        }
                        let min_global_d2 = all_chosen
                            .iter()
                            .map(|&(ox, oy)| {
                                let dx = (px - ox) as i64;
                                let dy = (py - oy) as i64;
                                dx * dx + dy * dy
                            })
                            .min()
                            .unwrap_or(min_local_d2);
                        let effective_d2 = min_local_d2.min(min_global_d2);
                        let bw = cand_boundary_weights[c_idx];
                        let jitter_w = 9 + (habitat_hash(px as i64, py as i64, salt) % 7) as i64;
                        let score = effective_d2 * bw * jitter_w
                            + (habitat_hash(py as i64, px as i64, salt + 99) % 31) as i64;
                        if score > best_score {
                            best_score = score;
                            best = (px, py, psub);
                        }
                    }
                    chosen.push((best.0, best.1));
                    all_chosen.push((best.0, best.1));
                    best
                };

            // (2) Tier 2 次級掠食猛獸／珍獸（6 隻 / 獵場，全島 114 隻）：分據獵場全境 6 大獨立巡獵領地
            for p_idx in 0..6usize {
                let (px, py, psub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    100 + p_idx as i64,
                );
                entries.push((px, py, psub, 2, "__tier2__", 1 + p_idx));
            }

            // (3) Tier 3 尋常中型獸群（全島 380 隻，按獵場面積配賦 t3_quota）：以每群 2~3 隻結群放牧於全區草甸溪谷
            const HERD_MEMBER_DELTAS: &[(i32, i32)] = &[
                (0, 0),
                (2, 1),
                (-1, 2),
                (1, -2),
                (-2, -1),
                (2, -1),
                (-2, 2),
            ];
            let mut rem_herd_members = t3_quota;
            let mut herd_idx = 0usize;
            while rem_herd_members > 0 {
                let herd_size = if rem_herd_members >= 3 {
                    3
                } else {
                    rem_herd_members
                };
                rem_herd_members -= herd_size;
                let (hx, hy, hsub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    300 + herd_idx as i64,
                );
                for m in 0..herd_size {
                    let (mut mx, mut my, mut msub) = (hx, hy, hsub);
                    if m > 0 {
                        for attempt in 0..HERD_MEMBER_DELTAS.len() {
                            let (dx, dy) = HERD_MEMBER_DELTAS
                                [(m + herd_idx + attempt) % HERD_MEMBER_DELTAS.len()];
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let tx = hx + dx;
                            let ty = hy + dy;
                            if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
                                && !chosen_points.contains(&(tx, ty))
                                && let Some(valid_sub) =
                                    is_valid_wilderness_habitat_cell(tx, ty, zone_name)
                            {
                                mx = tx;
                                my = ty;
                                msub = valid_sub;
                                chosen_points.push((mx, my));
                                global_chosen.push((mx, my));
                                break;
                            }
                        }
                    }
                    entries.push((mx, my, msub, 3, "__tier3__", 7 + herd_idx));
                }
                herd_idx += 1;
            }

            // (4) Tier 4 尋常小怪與山海靈禽（全島 1,292 隻，按獵場面積配賦 t4_quota）：以藍噪聲漫山散佈於獵場全境腹地
            for k in 0..t4_quota {
                let (fx, fy, fsub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    700 + k as i64,
                );
                let tier_tag = if k % 2 == 0 {
                    "__tier4a__"
                } else {
                    "__tier4b__"
                };
                entries.push((fx, fy, fsub, 4, tier_tag, 30 + k));
            }

            // (5) 次級掠食者過載溢出槽（3 隻 / 獵場）：供霸主退位時次級掠食者釋放（Mesopredator Release）湧現
            for ofs in 0..3usize {
                let (px, py, psub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    900 + ofs as i64,
                );
                entries.push((px, py, psub, 2, "__tier2_overflow__", 1000 + ofs));
            }

            layouts.push(entries);
        }
        layouts
    });

fn resolve_displaced_habitat_cell(
    bx: i32,
    by: i32,
    base_sub: &'static str,
    target_x: i32,
    target_y: i32,
    max_step: i32,
    apex_x: i32,
    apex_y: i32,
    zone_name: &str,
) -> (i32, i32, &'static str) {
    let dx = target_x - bx;
    let dy = target_y - by;
    let dist = dx.abs() + dy.abs();
    if dist == 0 || max_step <= 0 {
        return (bx, by, base_sub);
    }
    let step = max_step.min(dist);
    let step_x = ((dx as f32 / dist as f32) * (step as f32)).round() as i32;
    let step_y = ((dy as f32 / dist as f32) * (step as f32)).round() as i32;
    let cand_x = bx + step_x;
    let cand_y = by + step_y;

    const PROBE_OFFSETS: &[(i32, i32)] = &[(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)];
    for &(ox, oy) in PROBE_OFFSETS {
        let tx = cand_x + ox;
        let ty = cand_y + oy;
        if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
            && let Some(vsub) = is_valid_wilderness_habitat_cell(tx, ty, zone_name)
        {
            return (tx, ty, vsub);
        }
    }
    (bx, by, base_sub)
}

fn pick_zone_outskirt_road_spillover_cell(
    z_idx: usize,
    slot_idx: usize,
    time_tick: i64,
) -> Option<(i32, i32)> {
    let &(_, cx, cy) = CANONICAL_HUNTING_ZONES.get(z_idx)?;
    let milestones = get_island_road_milestones();
    if !milestones.is_empty() {
        let candidates: Vec<(i32, i32)> = milestones
            .iter()
            .copied()
            .filter_map(|(rx, ry, _)| {
                let dist = (rx - cx).abs() + (ry - cy).abs();
                if (15..=85).contains(&dist) && !is_inside_settlement_urban_core(rx, ry) {
                    Some((rx, ry))
                } else {
                    None
                }
            })
            .collect();
        if !candidates.is_empty() {
            let seed = (z_idx as i64 * 31)
                ^ (slot_idx as i64 * 71)
                ^ (time_tick * 101);
            let idx = (seed.unsigned_abs() as usize) % candidates.len();
            return Some(candidates[idx]);
        }
    }

    const SETTLEMENT_ANCHORS: &[(i32, i32, i32)] = &[
        (0, 0, 20),         // 文曜
        (-260, 160, 14),    // 風舶角
        (-330, 160, 11),    // 風舶嶼
        (-180, -140, 11),   // 大圳頭
        (160, 210, 11),     // 半嶺鎮
        (0, -220, 11),      // 南哨
        (0, 125, 9),        // 老堰頭
        (110, 115, 9),      // 北嶺腳
        (190, 290, 9),      // 霜林口
        (-360, 95, 9),      // 風櫃尾
        (-260, 35, 9),      // 烏石鼻
        (-180, -245, 9),    // 牛埔
        (-85, -140, 9),     // 紅土崁
        (-285, -140, 9),    // 三號坑口
        (-75, -280, 9),     // 水尾寮
    ];

    let nearest_anchor = SETTLEMENT_ANCHORS
        .iter()
        .min_by_key(|&(sx, sy, _)| (sx - cx).abs() + (sy - cy).abs())?;

    let (sx, sy, radius) = *nearest_anchor;
    let dx = sx - cx;
    let dy = sy - cy;
    let full_dist = (dx.abs() + dy.abs()).max(1);

    for t in (15..=85).step_by(3) {
        let step_ratio = (t as f32) / (full_dist as f32);
        let px = cx + ((dx as f32) * step_ratio).round() as i32;
        let py = cy + ((dy as f32) * step_ratio).round() as i32;
        let dist = (px - cx).abs() + (py - cy).abs();
        if (15..=85).contains(&dist)
            && !is_inside_settlement_urban_core(px, py)
            && (px - sx).abs() > radius + 1
            && (py - sy).abs() > radius + 1
        {
            return Some((px, py));
        }
    }

    let px = cx + 20;
    let py = cy + 20;
    if !is_inside_settlement_urban_core(px, py) {
        Some((px, py))
    } else {
        Some((cx + 25, cy + 25))
    }
}

/// 取得指定獵場（`z_idx in 0..19`）在當前時間片之《白塔山海誌》生靈即時棲地座標（個體與獸群各自獨立漫遊）。
pub fn live_zone_shanhai_slots(z_idx: usize, time_tick: i64) -> Vec<ShanhaiHabitatSlot> {
    step_all_zones_trophic_web_if_needed(time_tick);

    let Some(&(zone_name, _, _)) = CANONICAL_HUNTING_ZONES.get(z_idx) else {
        return Vec::new();
    };
    let base_list = &ZONE_HABITAT_LAYOUTS[z_idx];
    let (apex_x, apex_y) = base_list
        .first()
        .map(|&(x, y, ..)| (x, y))
        .unwrap_or((0, 0));

    let st = get_zone_trophic_state(z_idx);
    let apex_present = st.as_ref().map(|s| s.apex_presence >= 0.95).unwrap_or(true);
    let spillover_active = is_zone_predator_spillover_active(z_idx);
    let max_tier2 = st
        .as_ref()
        .map(|s| {
            let base = (s.n_tier2.round() as usize).clamp(1, 9);
            if spillover_active {
                base.max(7)
            } else {
                base
            }
        })
        .unwrap_or(6);
    let max_tier3 = st.as_ref().map(|s| (s.n_tier3.round() as usize).max(2)).unwrap_or(30);
    let base_max_tier4 = st.as_ref().map(|s| (s.n_tier4.round() as usize).max(4)).unwrap_or(70);

    let (_, tide) = current_zone_weather_and_tide(z_idx, time_tick);
    let max_tier4 = if is_coastal_hunting_zone(z_idx) && tide == ZoneTidePhase::EbbTide {
        ((base_max_tier4 as f32) * 1.15).round() as usize
    } else {
        base_max_tier4
    };

    let mut tier2_count = 0usize;
    let mut tier3_count = 0usize;
    let mut tier4_count = 0usize;

    let [n1, n2] = nearest_geographic_zones(z_idx);
    let rich_neighbor_idx = {
        let f1 = get_zone_trophic_state(n1).map(|s| s.flora_p).unwrap_or(1000.0);
        let f2 = get_zone_trophic_state(n2).map(|s| s.flora_p).unwrap_or(1000.0);
        if f1 >= f2 { n1 } else { n2 }
    };
    let &(_, rich_cx, rich_cy) = &CANONICAL_HUNTING_ZONES[rich_neighbor_idx];

    let mut out = Vec::with_capacity(base_list.len());
    let mut tier3_coords = Vec::new();

    // Pass 1: Tier 1, Tier 3, Tier 4
    for (slot_idx, &(bx, by, base_sub, tier, tier_tag, roam_group)) in
        base_list.iter().enumerate()
    {
        if tier == 2 {
            continue;
        }

        if tier == 1 {
            if !apex_present {
                continue;
            }
            out.push(ShanhaiHabitatSlot {
                slot_idx,
                base_x: bx,
                base_y: by,
                x: bx,
                y: by,
                sub_geo: base_sub,
                tier: 1,
                tier_tag,
                kind_str: "shanhai_apex",
                tier_label: "山海異獸",
                act_prefix: "潛伏於深林霧氣間，周遭百鳥屏息，正",
            });
            continue;
        }

        if tier == 3 {
            if tier3_count >= max_tier3 {
                continue;
            }
            tier3_count += 1;

            let respawn_gen = SHANHAI_SLOT_RESPAWN_GEN
                .read()
                .get(&(z_idx, slot_idx))
                .copied()
                .unwrap_or(0);

            let (gx, gy, sub_geo) = if st.as_ref().map(|s| s.flora_p < 500.0).unwrap_or(false) {
                resolve_displaced_habitat_cell(
                    bx,
                    by,
                    base_sub,
                    rich_cx,
                    rich_cy,
                    6,
                    apex_x,
                    apex_y,
                    zone_name,
                )
            } else {
                let effective_tick = time_tick.wrapping_add(respawn_gen.wrapping_mul(17) as i64);
                let rh = habitat_hash(
                    (z_idx as i64).wrapping_add(respawn_gen.wrapping_mul(7) as i64),
                    roam_group as i64,
                    effective_tick,
                );
                let rdx = ((rh % 5) as i32) - 2;
                let rdy = (((rh >> 8) % 5) as i32) - 2;
                let tx = bx + rdx;
                let ty = by + rdy;
                if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
                    && let Some(vsub) = is_valid_wilderness_habitat_cell(tx, ty, zone_name)
                {
                    (tx, ty, vsub)
                } else {
                    (bx, by, base_sub)
                }
            };

            tier3_coords.push((gx, gy));

            let act = if let Some(s) = &st {
                if s.flora_p >= 800.0 {
                    "正低首悠然啃食肥美嫩草、"
                } else if s.flora_p < 500.0 {
                    "因周遭嫩草漸稀，正往草木豐茂處游走遷徙、"
                } else {
                    "正結伴於林間游走覓食、"
                }
            } else {
                "正結伴於林間游走覓食、"
            };

            out.push(ShanhaiHabitatSlot {
                slot_idx,
                base_x: bx,
                base_y: by,
                x: gx,
                y: gy,
                sub_geo,
                tier: 3,
                tier_tag,
                kind_str: "shanhai_herd",
                tier_label: "山林獸群",
                act_prefix: act,
            });
            continue;
        }

        if tier == 4 {
            if tier4_count >= max_tier4 {
                continue;
            }
            tier4_count += 1;

            let respawn_gen = SHANHAI_SLOT_RESPAWN_GEN
                .read()
                .get(&(z_idx, slot_idx))
                .copied()
                .unwrap_or(0);

            let effective_tick = time_tick.wrapping_add(respawn_gen.wrapping_mul(17) as i64);
            let rh = habitat_hash(
                (z_idx as i64).wrapping_add(respawn_gen.wrapping_mul(7) as i64),
                roam_group as i64,
                effective_tick,
            );
            let rdx = ((rh % 5) as i32) - 2;
            let rdy = (((rh >> 8) % 5) as i32) - 2;
            let tx = bx + rdx;
            let ty = by + rdy;
            let (gx, gy, sub_geo) = if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
                && let Some(vsub) = is_valid_wilderness_habitat_cell(tx, ty, zone_name)
            {
                (tx, ty, vsub)
            } else {
                (bx, by, base_sub)
            };

            let lbl = if tier_tag == "__tier4a__" {
                "山野走獸"
            } else {
                "山海靈禽"
            };

            out.push(ShanhaiHabitatSlot {
                slot_idx,
                base_x: bx,
                base_y: by,
                x: gx,
                y: gy,
                sub_geo,
                tier: 4,
                tier_tag,
                kind_str: "shanhai_beast",
                tier_label: lbl,
                act_prefix: "",
            });
        }
    }

    // Pass 2: Tier 2 slots
    let now_unix = crate::game::now_unix();
    let sites_lock = RECENT_HUNT_KILL_SITES.read();
    let active_carcasses: Vec<(i32, i32)> = sites_lock
        .iter()
        .filter(|(_, site)| now_unix.saturating_sub(site.killed_at) <= 180)
        .map(|(&(cx, cy), _)| (cx, cy))
        .collect();
    drop(sites_lock);

    for (slot_idx, &(bx, by, base_sub, tier, tier_tag, roam_group)) in
        base_list.iter().enumerate()
    {
        if tier != 2 {
            continue;
        }

        if tier_tag == "__tier2_overflow__" {
            if !spillover_active {
                continue;
            }
            if tier2_count >= max_tier2 {
                continue;
            }
            tier2_count += 1;

            let (gx, gy, sub_geo) = if let Some((rx, ry)) =
                pick_zone_outskirt_road_spillover_cell(z_idx, slot_idx, time_tick)
            {
                (rx, ry, "古道外圍")
            } else {
                (bx, by, base_sub)
            };

            out.push(ShanhaiHabitatSlot {
                slot_idx,
                base_x: bx,
                base_y: by,
                x: gx,
                y: gy,
                sub_geo,
                tier: 2,
                tier_tag,
                kind_str: "shanhai_predator",
                tier_label: "山林猛獸",
                act_prefix: "因山中獵物匱乏而越界下山，正於古道草莽間伏行窺伺、",
            });
            continue;
        }

        if tier2_count >= max_tier2 {
            continue;
        }
        tier2_count += 1;

        let nearest_carcass = active_carcasses
            .iter()
            .filter_map(|&(cx, cy)| {
                let dist = (cx - bx).abs() + (cy - by).abs();
                if dist <= 65 {
                    Some((dist, cx, cy))
                } else {
                    None
                }
            })
            .min_by_key(|&(dist, ..)| dist);

        let (gx, gy, sub_geo, act_prefix) = if let Some((_, kx, ky)) = nearest_carcass {
            let (dx, dy, dsub) = resolve_displaced_habitat_cell(
                bx,
                by,
                base_sub,
                kx,
                ky,
                8,
                apex_x,
                apex_y,
                zone_name,
            );
            (dx, dy, dsub, "循著林間血氣聚攏於殘骸周遭爭食、")
        } else if (!apex_present || st.as_ref().map(|s| s.n_tier3 < s.k_tier3 * 0.70).unwrap_or(false))
            && !tier3_coords.is_empty()
        {
            let &(hx, hy) = tier3_coords
                .iter()
                .min_by_key(|&&(x, y)| (x - bx).abs() + (y - by).abs())
                .unwrap();
            let (dx, dy, dsub) = resolve_displaced_habitat_cell(
                bx,
                by,
                base_sub,
                hx,
                hy,
                7,
                apex_x,
                apex_y,
                zone_name,
            );
            let act = if !apex_present {
                "趁山中無巨獸出沒，正結伴四處巡獵、"
            } else {
                "腹中飢餓，正緊緊尾隨前方獸群伺機撲擊、"
            };
            (dx, dy, dsub, act)
        } else {
            let respawn_gen = SHANHAI_SLOT_RESPAWN_GEN
                .read()
                .get(&(z_idx, slot_idx))
                .copied()
                .unwrap_or(0);
            let effective_tick = time_tick.wrapping_add(respawn_gen.wrapping_mul(17) as i64);
            let rh = habitat_hash(
                (z_idx as i64).wrapping_add(respawn_gen.wrapping_mul(7) as i64),
                roam_group as i64,
                effective_tick,
            );
            let rdx = ((rh % 5) as i32) - 2;
            let rdy = (((rh >> 8) % 5) as i32) - 2;
            let tx = bx + rdx;
            let ty = by + rdy;
            let (dx, dy, dsub) = if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
                && let Some(vsub) = is_valid_wilderness_habitat_cell(tx, ty, zone_name)
            {
                (tx, ty, vsub)
            } else {
                (bx, by, base_sub)
            };
            (dx, dy, dsub, "正壓低身形於草木間潛步巡獵、")
        };

        out.push(ShanhaiHabitatSlot {
            slot_idx,
            base_x: bx,
            base_y: by,
            x: gx,
            y: gy,
            sub_geo,
            tier: 2,
            tier_tag,
            kind_str: "shanhai_predator",
            tier_label: "山林猛獸",
            act_prefix,
        });
    }

    out.sort_by_key(|s| s.slot_idx);
    out
}

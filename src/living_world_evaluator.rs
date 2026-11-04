#![forbid(unsafe_code)]
//! 奇點世界（Singularity World）三大品質、防撞衫與需求路由驗算器
//!
//! 借鑑 `NVIDIA/SkillEvaluator` 之確定性守門、雙連詞（2-gram）重疊檢測、
//! 正負例路由模擬（Positive/Negative Trigger Routing Simulation）與反事實增益（Counterfactual Lift）：
//! 1. `audit_living_world_anti_template_overlap`：萬人萬相居民日常與文曜地標防撞衫／防重複檢測器
//! 2. `simulate_room_tag_and_assignment_routing`：`Room Tags` + `Assignment` 需求決策正負例路由驗算器
//! 3. `evaluate_singularity_world_quality_suite`：整合三大驗算器（含 Data-to-Text 交鋒敘事四維計分與反事實增益）

use std::collections::{HashMap, HashSet};

use anyhow::{Result, ensure};

use crate::ai::narrative::{
    CombatNarrativeCounterfactualReport, evaluate_combat_narrative_counterfactual_lift,
};
use crate::grid::{
    SquareCoord, Terrain, derive_room_tags, generate_white_tower_grid,
    resolve_any_room_venue_object,
};
use crate::store::households::{WENYAO_HOUSEHOLD_SLOTS, WenyaoHouseholdSlot};
use crate::store::outer_households::OUTER_HOUSEHOLD_SLOTS;

/// 萬人萬相居民與地標防撞衫重疊檢測報告（Evaluator 1）
#[derive(Debug, Clone, PartialEq)]
pub struct AntiTemplateOverlapReport {
    pub wenyao_slot_count: usize,
    pub outer_slot_count: usize,
    pub wenyao_commercial_landmark_count: usize,
    pub wenyao_residential_landmark_count: usize,
    pub max_wenyao_resident_jaccard: f64,
    pub max_landmark_desc_jaccard: f64,
    pub resident_overlap_violations: Vec<String>,
    pub landmark_overlap_violations: Vec<String>,
    pub venue_talk_line_count: usize,
    pub unique_venue_talk_line_count: usize,
    pub unique_talk_line_ratio: f64,
    pub duplicate_talk_lines: Vec<String>,
}

/// `Room Tags` + `Assignment` 需求決策正負例路由模擬報告（Evaluator 2）
#[derive(Debug, Clone, PartialEq)]
pub struct DemandRoutingSimulationReport {
    pub positive_total: usize,
    pub positive_pass_count: usize,
    pub negative_total: usize,
    pub negative_false_positive_count: usize,
    pub routing_accuracy: f64,
    pub positive_failures: Vec<String>,
    pub negative_false_positives: Vec<String>,
}

/// 全倉防鬼打牆與正典恆定契約審計報告（Evaluator 4）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodebaseAntiLoopingAuditReport {
    pub audited_text_surfaces: usize,
    pub audited_core_modules: usize,
    pub violations: Vec<String>,
}

/// 奇點世界四大驗算器總成報告
#[derive(Debug, Clone, PartialEq)]
pub struct SingularityWorldEvaluationSuite {
    pub overlap_report: AntiTemplateOverlapReport,
    pub routing_report: DemandRoutingSimulationReport,
    pub narrative_report: CombatNarrativeCounterfactualReport,
    pub anti_looping_report: CodebaseAntiLoopingAuditReport,
}

fn cjk_char_bigrams(text: &str) -> HashSet<(char, char)> {
    let chars: Vec<char> = text
        .chars()
        .filter(|c| {
            !c.is_whitespace()
                && !matches!(
                    c,
                    '，' | '。' | '！' | '？' | '、' | '；' | '：' | '「' | '」' | '・' | '（' | '）'
                )
        })
        .collect();
    if chars.len() < 2 {
        return HashSet::new();
    }
    chars.windows(2).map(|w| (w[0], w[1])).collect()
}

fn bigram_jaccard(a: &HashSet<(char, char)>, b: &HashSet<(char, char)>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    if inter == 0 {
        return 0.0;
    }
    let union = a.len() + b.len() - inter;
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

fn slot_composite_signature(slot: &WenyaoHouseholdSlot) -> String {
    format!(
        "{}|{}|{}|{}",
        slot.home_stay_activity,
        slot.work_stay_activity,
        slot.errand_stay_activity,
        slot.attire_line
    )
}

/// 執行 Evaluator 1：萬人萬相居民作息、地標描述與當值台詞防撞衫／防重複檢測
pub fn audit_living_world_anti_template_overlap() -> AntiTemplateOverlapReport {
    let mut resident_overlap_violations = Vec::new();
    let mut max_wenyao_resident_jaccard: f64 = 0.0;

    let wenyao_grams: Vec<(usize, &WenyaoHouseholdSlot, HashSet<(char, char)>)> =
        WENYAO_HOUSEHOLD_SLOTS
            .iter()
            .enumerate()
            .map(|(idx, slot)| (idx, slot, cjk_char_bigrams(&slot_composite_signature(slot))))
            .collect();

    for i in 0..wenyao_grams.len() {
        for j in (i + 1)..wenyao_grams.len() {
            let sim = bigram_jaccard(&wenyao_grams[i].2, &wenyao_grams[j].2);
            if sim > max_wenyao_resident_jaccard {
                max_wenyao_resident_jaccard = sim;
            }
            if sim >= 0.85 {
                resident_overlap_violations.push(format!(
                    "文曜居民槽位 #{i} ({}) 與 #{j} ({}) 複合人設高度重疊 (Jaccard={sim:.2})",
                    wenyao_grams[i].1.home_name, wenyao_grams[j].1.home_name
                ));
            }
        }
    }

    // 檢查外圍聚落同聚落居民槽位是否出現模板複製 (Jaccard >= 0.85)
    let mut outer_by_settlement: HashMap<&str, Vec<(usize, &WenyaoHouseholdSlot, HashSet<(char, char)>)>> =
        HashMap::new();
    for (idx, slot) in OUTER_HOUSEHOLD_SLOTS.iter().enumerate() {
        let settlement = slot.home_name.split('・').next().unwrap_or(slot.home_name);
        outer_by_settlement.entry(settlement).or_default().push((
            idx,
            slot,
            cjk_char_bigrams(&slot_composite_signature(slot)),
        ));
    }
    for (settlement, group) in &outer_by_settlement {
        for i in 0..group.len() {
            for j in (i + 1)..group.len() {
                let sim = bigram_jaccard(&group[i].2, &group[j].2);
                if sim >= 0.85 {
                    resident_overlap_violations.push(format!(
                        "外圍聚落 {settlement} 槽位 #{} ({}) 與 #{} ({}) 複合人設高度重疊 (Jaccard={sim:.2})",
                        group[i].0, group[i].1.home_name, group[j].0, group[j].1.home_name
                    ));
                }
            }
        }
    }

    // 檢查文曜 245 棟地標（95 商業／功能地標 + 150 民居住宅）描述重疊與疊字瑕疵
    let grid = generate_white_tower_grid();
    let mut commercial_landmarks: Vec<(&str, &str, HashSet<(char, char)>)> = Vec::new();
    let mut wenyao_residential_landmark_count = 0usize;
    let mut landmark_overlap_violations = Vec::new();

    for dx in -17..=17 {
        for dy in -17..=17 {
            if let Some(cell) = grid.get(SquareCoord::new(dx, dy))
                && cell.terrain.category() == "landmark"
            {
                if cell.description.contains("擺著教案上的") || cell.description.contains("細密密") {
                    landmark_overlap_violations.push(format!(
                        "文曜地標「{}」描述含有贅字瑕疵：{}",
                        cell.name, cell.description
                    ));
                }
                if cell.terrain == Terrain::House {
                    wenyao_residential_landmark_count += 1;
                } else {
                    commercial_landmarks.push((
                        cell.name.as_str(),
                        cell.description.as_str(),
                        cjk_char_bigrams(&cell.description),
                    ));
                }
            }
        }
    }
    commercial_landmarks.sort_by(|a, b| a.0.cmp(&b.0));

    let mut max_landmark_desc_jaccard: f64 = 0.0;
    for i in 0..commercial_landmarks.len() {
        for j in (i + 1)..commercial_landmarks.len() {
            let sim = bigram_jaccard(&commercial_landmarks[i].2, &commercial_landmarks[j].2);
            if sim > max_landmark_desc_jaccard {
                max_landmark_desc_jaccard = sim;
            }
            if sim >= 0.85 {
                landmark_overlap_violations.push(format!(
                    "文曜商業功能地標「{}」與「{}」描述高度重疊 (Jaccard={sim:.2})",
                    commercial_landmarks[i].0, commercial_landmarks[j].0
                ));
            }
        }
    }

    // 檢查場域物件（RoomVenueObject）當值對話語錄唯一率
    let sample_venue_probes = [
        ("半島百草診所", ""), ("巧木居所傢俱坊", ""), ("分家巷・精工火磚匠廬", ""),
        ("岑氏第一鍛造工坊", ""), ("三水大食堂", ""), ("泉香溫水浴場", ""),
        ("東橋旅社與山風茶屋", ""), ("學院主校區", ""), ("藏書閣與印行館", ""),
        ("百工商算學舍", ""), ("文曜蒙學堂講舍", ""), ("三水總商行總部", ""),
        ("傳訊鐘樓與信使總站", ""), ("分家巷口雜貨柑仔店", ""), ("三水口乾貨海味行", ""),
        ("分家巷習武堂試招院", ""), ("公議堂", ""), ("公議堂房地契書局", ""),
        ("雲錦織造與服飾坊", ""), ("西門車馬聯運總站", ""), ("西門修傘補靴攤", ""),
        ("老堰頭水輪石磨", ""), ("早市鮮魚拍賣場", "魚販"), ("陳記醬園", ""),
        ("沈家老酒坊", ""), ("西門麵食鋪", ""), ("水手茶飯館", ""),
        ("通濟河運總倉", ""), ("竹綿造紙精廬", ""), ("端溪石硯坊", ""),
        ("活字排版印書局", ""), ("格物輿圖外院", ""), ("文曜棋院", ""),
        ("杏林百草醫館", ""), ("溫氏百草烘藥房", ""), ("重鐵水錘大鍛坊", ""),
        ("五金農具總商行", ""), ("護商武備鋪", ""), ("精銅鐘錶儀器坊", ""),
        ("連軸鋸木坊", ""), ("高山木炭窯", ""), ("染纈織染居", ""),
        ("內河造船塢", ""), ("萬里駝隊大驛站", ""), ("西門皮毛大市", ""),
        ("重載雙輪大車坊", ""), ("車馬獸醫館", ""), ("川流貨船大埠", ""), ("西門街", ""),
    ];

    let mut seen_venue_kinds = HashSet::new();
    let mut seen_talk_lines = HashSet::new();
    let mut duplicate_talk_lines = Vec::new();
    let mut venue_talk_line_count = 0usize;

    for (room_name, hint) in sample_venue_probes {
        if let Some(venue) = resolve_any_room_venue_object(room_name, hint)
            && seen_venue_kinds.insert(venue.venue_kind)
        {
            for &line in venue.on_duty_talk_lines {
                venue_talk_line_count += 1;
                if !seen_talk_lines.insert(line.to_string()) {
                    duplicate_talk_lines.push(format!("{}: {}", venue.venue_kind, line));
                }
            }
        }
    }

    let unique_venue_talk_line_count = seen_talk_lines.len();
    let unique_talk_line_ratio = if venue_talk_line_count == 0 {
        1.0
    } else {
        ((unique_venue_talk_line_count as f64 / venue_talk_line_count as f64) * 1000.0).round()
            / 1000.0
    };

    AntiTemplateOverlapReport {
        wenyao_slot_count: WENYAO_HOUSEHOLD_SLOTS.len(),
        outer_slot_count: OUTER_HOUSEHOLD_SLOTS.len(),
        wenyao_commercial_landmark_count: commercial_landmarks.len(),
        wenyao_residential_landmark_count,
        max_wenyao_resident_jaccard: (max_wenyao_resident_jaccard * 100.0).round() / 100.0,
        max_landmark_desc_jaccard: (max_landmark_desc_jaccard * 100.0).round() / 100.0,
        resident_overlap_violations,
        landmark_overlap_violations,
        venue_talk_line_count,
        unique_venue_talk_line_count,
        unique_talk_line_ratio,
        duplicate_talk_lines,
    }
}

/// 執行 Evaluator 2：`Room Tags` + `Assignment` 需求決策正負例路由驗算器
pub fn simulate_room_tag_and_assignment_routing() -> DemandRoutingSimulationReport {
    let grid = generate_white_tower_grid();
    let mut positive_total = 0usize;
    let mut positive_pass_count = 0usize;
    let mut positive_failures = Vec::new();

    // 1. 正例需求路由驗算：從文曜中心與各聚落搜尋核心生活／百工需求標籤，必須命中非住宅地標並具備對應標籤
    let positive_demand_queries: &[(&str, (i32, i32), i32, &[&str], bool)] = &[
        ("熱食膳食需求", (0, 0), 15, &["食堂", "熱食", "現煮飲品"], true),
        ("醫館藥材需求", (0, 0), 15, &["藥圃", "藥材"], true),
        ("鍛造鐵器需求", (0, 0), 15, &["鐵匠鋪", "工坊"], true),
        ("布匹織染需求", (0, 0), 15, &["織坊", "布料"], true),
        ("商鋪市集需求", (0, 0), 15, &["商鋪", "市集"], true),
        ("溫水沐浴需求", (0, 0), 15, &["浴場"], false),
        ("茶屋旅社歇宿需求", (0, 0), 15, &["茶屋", "旅社"], false),
        ("車馬聯運碼頭需求", (0, 0), 15, &["聯運", "碼頭"], false),
        ("大圳頭糧倉儲糧需求", (-180, -140), 15, &["糧倉", "穀物"], true),
        ("霜林口林場木材需求", (190, 290), 15, &["林場", "木材"], false),
        ("風舶角漁港海鹽需求", (-260, 160), 15, &["漁港", "海鹽"], true),
    ];

    for &(label, origin, radius, tags, require_counter_zone) in positive_demand_queries {
        positive_total += 1;
        match grid.find_nearest_room_by_tags(origin, radius, tags, 0) {
            Some((coord, matched_name)) => {
                let cell = grid.get(SquareCoord::new(coord.0, coord.1));
                let is_house = cell.is_some_and(|c| c.terrain == Terrain::House);
                let has_counter = cell.is_some_and(|c| {
                    c.has_room_tag("櫃檯") && c.has_room_tag("非本店人員禁止進入區")
                });
                if is_house {
                    positive_failures.push(format!(
                        "{label} 誤路由至私人住宅「{matched_name}」({coord:?})"
                    ));
                } else if require_counter_zone && !has_counter {
                    positive_failures.push(format!(
                        "{label} 命中「{matched_name}」但缺少櫃檯／非本店人員禁止進入區標籤"
                    ));
                } else {
                    positive_pass_count += 1;
                }
            }
            None => {
                positive_failures.push(format!("{label} 於半徑 {radius} 內找不到任何匹配場域"));
            }
        }
    }

    // 2. 正例場域物件契約解析（特別驗證曾遭「分家巷」或「講堂街」遮蔽之場域）
    let positive_venue_expectations: &[(&str, &str, bool)] = &[
        ("文曜・分家巷習武堂試招院", "演武堂", false),
        ("文曜・分家巷口雜貨柑仔店", "街角雜貨鋪", true),
        ("文曜・分家巷・染纈織染居", "織造染坊", true),
        ("文曜・分家巷・磨刀利刃鋪", "大鍛鐵匠鍛造坊", true),
        ("文曜・分家巷・精工火磚匠廬", "導能器脈工坊", true),
        ("文曜・溫氏脈絡診所", "醫館藥局", true),
        ("文曜・三水大食堂", "大食堂", true),
        ("文曜・三水總商行總部", "商行通兌總處", true),
        ("分家巷", "街巷橋樑公共空地", false),
        ("講堂街", "街巷橋樑公共空地", false),
        ("西門街", "街巷橋樑公共空地", false),
    ];

    for &(room_name, expected_kind, expected_counter_zone) in positive_venue_expectations {
        positive_total += 1;
        match resolve_any_room_venue_object(room_name, "") {
            Some(v)
                if v.venue_kind == expected_kind
                    && v.has_counter_restricted_zone == expected_counter_zone =>
            {
                positive_pass_count += 1;
            }
            Some(v) => {
                positive_failures.push(format!(
                    "場域「{room_name}」解析為 ({}, counter={})，預期為 ({expected_kind}, counter={expected_counter_zone})",
                    v.venue_kind, v.has_counter_restricted_zone
                ));
            }
            None => {
                positive_failures.push(format!(
                    "場域「{room_name}」未能解析出預期物件契約 {expected_kind}"
                ));
            }
        }
    }

    // 3. 負例反證驗算（Negative Counterexamples）：
    //    (a) 文曜全部 150 棟 Terrain::House 住宅與私人內室絕不可誤掛「櫃檯」、「非本店人員禁止進入區」或商業供給標籤
    let mut negative_total = 0usize;
    let mut negative_false_positive_count = 0usize;
    let mut negative_false_positives = Vec::new();

    let forbidden_private_tags = [
        "櫃檯", "非本店人員禁止進入區", "商鋪", "市集", "鐵匠鋪", "醫館", "藥圃", "織坊", "糧倉", "食堂",
    ];

    for dx in -17..=17 {
        for dy in -17..=17 {
            if let Some(cell) = grid.get(SquareCoord::new(dx, dy))
                && cell.terrain == Terrain::House
            {
                negative_total += 1;
                let leaked_tag = forbidden_private_tags
                    .iter()
                    .copied()
                    .find(|t| cell.has_room_tag(t));
                let leaked_venue = cell
                    .venue_object()
                    .filter(|v| v.has_counter_restricted_zone);
                if let Some(tag) = leaked_tag {
                    negative_false_positive_count += 1;
                    negative_false_positives.push(format!(
                        "文曜民居「{}」誤掛商業／櫃檯標籤「{tag}」",
                        cell.name
                    ));
                } else if let Some(v) = leaked_venue {
                    negative_false_positive_count += 1;
                    negative_false_positives.push(format!(
                        "文曜民居「{}」誤解析為管制櫃檯場域「{}」",
                        cell.name, v.venue_kind
                    ));
                }
            }
        }
    }

    //    (b) 顯式私人臥房／內室與易混淆匠師居所名稱（傳入 Terrain::Road 時亦不可誤判為商業櫃檯）
    let explicit_private_rooms = [
        "三水口・陸家宅邸・內室", "講堂街・溫家藥圃居・內室", "講堂街・溫家藥圃居",
        "分家巷・岑氏鍛藝居", "分家巷・蘇氏繡坊居", "分家巷・柯氏木作居",
        "西門・遠行行商棧舍", "西門・行道車馬行舍", "西門・草料客商居",
        "西門・皮靴油布匠居", "三水口・白氏湯屋居", "三水口・陳氏庖廚居",
        "大圳頭・修耙補犁鋪居", "風舶角・海風麵點鋪居",
    ];

    for room_name in explicit_private_rooms {
        negative_total += 1;
        let tags = derive_room_tags(Terrain::Road, room_name, "文曜");
        let venue = resolve_any_room_venue_object(room_name, "");
        if tags
            .iter()
            .any(|t| forbidden_private_tags.contains(&t.as_str()))
            || venue.is_some_and(|v| v.has_counter_restricted_zone)
        {
            negative_false_positive_count += 1;
            negative_false_positives.push(format!(
                "私人居所／內室「{room_name}」誤觸發商業標籤或櫃檯管制：{tags:?}"
            ));
        }
    }

    //    (c) 公共街道名稱絕不可誤掛「櫃檯」、「非本店人員禁止進入區」或「商鋪」
    let public_street_rooms = [
        "文曜・三水口商號街", "文曜・西門貨棧街", "文曜・西門聯運大道",
        "文曜・講堂街南口", "文曜・分家巷百工石道", "分家巷", "講堂街", "西門街",
    ];
    for street_name in public_street_rooms {
        negative_total += 1;
        let tags = derive_room_tags(Terrain::Road, street_name, "文曜");
        let venue = resolve_any_room_venue_object(street_name, "");
        if tags
            .iter()
            .any(|t| matches!(t.as_str(), "櫃檯" | "非本店人員禁止進入區" | "商鋪"))
            || venue.is_some_and(|v| v.has_counter_restricted_zone)
        {
            negative_false_positive_count += 1;
            negative_false_positives.push(format!(
                "公共街道「{street_name}」誤掛商業櫃檯標籤：{tags:?}"
            ));
        }
    }

    let total_cases = positive_total + negative_total;
    let passed_cases = positive_pass_count + (negative_total - negative_false_positive_count);
    let routing_accuracy = if total_cases == 0 {
        1.0
    } else {
        ((passed_cases as f64 / total_cases as f64) * 10000.0).round() / 10000.0
    };

    DemandRoutingSimulationReport {
        positive_total,
        positive_pass_count,
        negative_total,
        negative_false_positive_count,
        routing_accuracy,
        positive_failures,
        negative_false_positives,
    }
}

/// 審計全島 1,713 戶家戶、49 類場域、千物總典、事件／劇本包與跨模組接線恆定契約（Evaluator 4）
pub fn audit_codebase_anti_looping_invariants() -> CodebaseAntiLoopingAuditReport {
    let forbidden_tokens = [
        "剛才", "剛剛", "方才", "適才", "正典人物", "正典NPC", "一般居民", "普通居民",
        "處方", "本坊", "wt_cat_", "系統提示",
    ];
    let mut audited_text_surfaces = 0usize;
    let mut violations = Vec::new();
    let mut check_surface = |label: &str, text: &str| {
        audited_text_surfaces += 1;
        for token in forbidden_tokens {
            if text.contains(token) {
                violations.push(format!("{label} 含禁用詞「{token}」"));
            }
        }
    };
    for s in WENYAO_HOUSEHOLD_SLOTS.iter().chain(OUTER_HOUSEHOLD_SLOTS.iter()) {
        check_surface(s.home_name, s.home_stay_activity);
        check_surface(s.home_name, s.work_stay_activity);
        check_surface(s.home_name, s.errand_stay_activity);
        check_surface(s.home_name, s.attire_line);
    }
    for item in crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS {
        check_surface(item.id, item.name);
        check_surface(item.id, item.look_text);
        check_surface(item.id, item.action_text);
    }
    for script in crate::game::peninsula_scripts::PENINSULA_DAILY_SCRIPTS {
        check_surface(script.id, script.title);
        check_surface(script.id, script.trigger_and_causal_chain);
        check_surface(script.id, script.player_actions);
        check_surface(script.id, script.outcome_and_world_change);
    }
    let grid = generate_white_tower_grid();
    for cell in grid.cells() {
        check_surface(&cell.name, &cell.description);
    }
    let dlg_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/templates/dialogues");
    if let Ok(entries) = std::fs::read_dir(dlg_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "json")
                && let Ok(raw) = std::fs::read_to_string(entry.path())
            {
                check_surface(&entry.file_name().to_string_lossy(), &raw);
            }
        }
    }
    let mut script_ids = HashSet::new();
    for script in crate::game::peninsula_scripts::PENINSULA_DAILY_SCRIPTS {
        if !script_ids.insert(script.id) {
            violations.push(format!("日常劇本編號重複：{}", script.id));
        }
        let (ev, sp) = crate::game::peninsula_scripts::decompose_into_event_and_script_pack(
            script,
            "白塔半島",
            None,
            None,
        );
        if ev.room_tags.is_empty() || sp.host_slot_actor.is_empty() {
            violations.push(format!("劇本 {} 拆解後缺少場域標籤或當值槽位", script.id));
        }
        for &exemplar in crate::game::peninsula_scripts::EXEMPLAR_NAMES {
            if ev.event_title.contains(exemplar)
                || ev.trigger_and_causal_chain.contains(exemplar)
                || sp.player_actions.contains(exemplar)
                || sp.outcome_and_world_change.contains(exemplar)
            {
                violations.push(format!("劇本 {} 動態拆解後殘留寫死人名「{exemplar}」", script.id));
            }
        }
    }
    let module_contracts: [(&str, &str, &[&str]); 6] = [
        (
            "src/pg/living_world.rs",
            include_str!("pg/living_world.rs"),
            &["is_recognizable_room_staff_or_counter_holder", "resolve_nearest_room_by_tag", "ALL_RESIDENTS_SEEDED.call_once"],
        ),
        (
            "src/npc/causal_core.rs",
            include_str!("npc/causal_core.rs"),
            &["is_recognizable_room_staff_or_counter_holder_entity", "resolve_nearest_room_by_tag"],
        ),
        (
            "src/server/simulation_loop/idle.rs",
            include_str!("server/simulation_loop/idle.rs"),
            &["co_enact_script_pack", "resolve_any_room_venue_object"],
        ),
        (
            "src/grid/grid.rs",
            include_str!("grid/grid.rs"),
            &["find_nearest_room_by_tags", "is_private_residence_room_name", "is_street_or_gate_room_name"],
        ),
        (
            "src/pg/world_work.rs",
            include_str!("pg/world_work.rs"),
            &["ensure_schema"],
        ),
        (
            "src/pg/world_trade.rs",
            include_str!("pg/world_trade.rs"),
            &["ensure_schema"],
        ),
    ];
    for (mod_name, src, required_anchors) in module_contracts {
        for anchor in required_anchors {
            if !src.contains(anchor) {
                violations.push(format!("跨模組防退化契約缺口：{mod_name} 缺少關鍵接線 `{anchor}`"));
            }
        }
    }
    CodebaseAntiLoopingAuditReport {
        audited_text_surfaces,
        audited_core_modules: module_contracts.len(),
        violations,
    }
}

/// 執行奇點世界四大驗算器並強制檢核所有品質與防退化門檻
pub fn evaluate_singularity_world_quality_suite() -> Result<SingularityWorldEvaluationSuite> {
    let overlap_report = audit_living_world_anti_template_overlap();
    ensure!(overlap_report.resident_overlap_violations.is_empty(), "發現居民萬人萬相模板重疊違規：{:?}", overlap_report.resident_overlap_violations);
    ensure!(overlap_report.landmark_overlap_violations.is_empty(), "發現文曜地標描述重疊或贅字違規：{:?}", overlap_report.landmark_overlap_violations);
    ensure!(overlap_report.duplicate_talk_lines.is_empty(), "發現場域當值台詞重複：{:?}", overlap_report.duplicate_talk_lines);

    let routing_report = simulate_room_tag_and_assignment_routing();
    ensure!(routing_report.positive_failures.is_empty(), "發現需求決策正例路由失敗：{:?}", routing_report.positive_failures);
    ensure!(routing_report.negative_false_positives.is_empty(), "發現需求決策負例誤判（私宅或街道誤掛櫃檯／商業標籤）：{:?}", routing_report.negative_false_positives);

    let narrative_report = evaluate_combat_narrative_counterfactual_lift();
    ensure!(
        narrative_report.all_baseline_passed && narrative_report.all_defective_rejected && narrative_report.mean_lift >= 45.0,
        "Data-to-Text 交鋒敘事四維品質或反事實增益未達標：{narrative_report:?}"
    );
    let anti_looping_report = audit_codebase_anti_looping_invariants();
    ensure!(
        anti_looping_report.violations.is_empty(),
        "發現全倉防鬼打牆或跨模組接線退化違規：{:?}",
        anti_looping_report.violations
    );
    Ok(SingularityWorldEvaluationSuite { overlap_report, routing_report, narrative_report, anti_looping_report })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singularity_world_three_evaluators_pass_all_gates() {
        let suite = evaluate_singularity_world_quality_suite()
            .expect("奇點世界三大驗算器必須全數通過");
        assert_eq!(suite.overlap_report.wenyao_slot_count, 324);
        assert_eq!(suite.overlap_report.outer_slot_count, 1389);
        assert_eq!(suite.overlap_report.wenyao_commercial_landmark_count, 95);
        assert_eq!(suite.overlap_report.wenyao_residential_landmark_count, 150);
        assert!(suite.overlap_report.max_wenyao_resident_jaccard < 0.85);
        assert!(suite.overlap_report.max_landmark_desc_jaccard < 0.85);
        assert_eq!(suite.overlap_report.unique_talk_line_ratio, 1.0);
        assert_eq!(suite.routing_report.negative_false_positive_count, 0);
        assert_eq!(suite.routing_report.routing_accuracy, 1.0);
        assert!(suite.narrative_report.baseline_mean_score >= 90.0);
        assert!(suite.narrative_report.mean_lift >= 45.0);
    }
}

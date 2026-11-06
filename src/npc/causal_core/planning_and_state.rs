/// 依據住民生理與處境需求，由內向外推進決策意象並依真實有效鎂幣擇定行為動作
pub fn plan_next_causal_action(
    entity_id: &str,
    role_hint: &str,
    settlement: &str,
    seed: u64,
    prev_dest_name: &str,
    streaks: u8,
    has_affliction: bool,
    vessel_worn: bool,
    real_magnesium: i32,
    hour: u32,
    scarcity_focus: Option<&str>,
) -> (NpcCausalState, &'static ActionOptionDef) {
    let already_existed = LIVE_NPC_CAUSAL_STATES
        .read()
        .ok()
        .is_some_and(|m| m.contains_key(entity_id));
    let mut state = get_or_init_causal_state(entity_id, role_hint, settlement, seed);

    // 先結算上一輪已完成動作對生理狀態之舒緩與預計鎂幣收支，再推進下一輪時間刻度與需求
    let mut effective_mg = real_magnesium;
    if already_existed
        && let Some(prev_opt) = lookup_action_option(&state.chosen_action_code)
    {
        let dyn_cost = dynamic_action_cost_mg(settlement, prev_opt);
        let dyn_earn = dynamic_action_earn_mg(settlement, prev_opt);
        let est_spent = dyn_cost.min(real_magnesium.max(0));
        effective_mg = (real_magnesium - est_spent + dyn_earn).max(0);
        match prev_opt.image {
            DecisionImageKind::Eat => {
                let red = if dyn_cost > 0 { 55 } else { 35 };
                state.hunger = state.hunger.saturating_sub(red);
            }
            DecisionImageKind::Drink => {
                state.thirst = state.thirst.saturating_sub(55);
            }
            DecisionImageKind::SleepOrRest => {
                state.fatigue = state.fatigue.saturating_sub(60);
                state.stress = state.stress.saturating_sub(18);
                if state.stress < 72 {
                    state.stress_break_active = false;
                }
            }
            DecisionImageKind::WorkOrTrade => {
                state.hunger = state.hunger.saturating_add(8).min(100);
                state.thirst = state.thirst.saturating_add(8).min(100);
                state.fatigue = state.fatigue.saturating_add(10).min(100);
            }
            _ => {}
        }
    }

    state.life_step_tick = state.life_step_tick.saturating_add(1);
    state.same_task_repetitions = state.same_task_repetitions.saturating_add(1);
    state.hunger = state.hunger.saturating_add(12).min(100);
    state.thirst = state.thirst.saturating_add(14).min(100);
    state.fatigue = state.fatigue.saturating_add(10).min(100);
    let vessel = resolve_vocational_vessel(role_hint);
    let s_clean = if settlement.trim().is_empty() { "文曜" } else { clean_settlement_name(settlement) };
    let is_rest_or_heal = prev_dest_name.contains("浴")
        || prev_dest_name.contains("湯")
        || prev_dest_name.contains("食堂")
        || prev_dest_name.contains("茶")
        || prev_dest_name.contains("診")
        || prev_dest_name.contains("居")
        || prev_dest_name.contains("宅");
    if is_rest_or_heal {
        state.stress = state.stress.saturating_sub(16);
        if state.stress < 72 {
            state.stress_break_active = false;
        }
    } else {
        let mut stress_delta: u8 = 4 + streaks.min(6).saturating_mul(3);
        if has_affliction {
            stress_delta = stress_delta.saturating_add(12);
        }
        if vessel_worn && state.cornerstone == Cornerstone::Craft {
            stress_delta = stress_delta.saturating_add(12);
        }
        if effective_mg <= 4 {
            stress_delta = stress_delta.saturating_add(8);
        }
        state.stress = state.stress.saturating_add(stress_delta).min(100);
    }

    if hour == 6 {
        state.carryover_from_yesterday = format!(
            "延續昨日第 {} 回工序手感與生計步調，起床接續今日生計",
            state.same_task_repetitions
        );
    }

    let live_scarcity = crate::game::object_ecology::settlement_scarcity_signal(settlement);
    let (scarce_label, scarce_ratio) = if let Some((_, ref name, r)) = live_scarcity {
        (Some(name.as_str()), r)
    } else {
        (scarcity_focus, 1.0)
    };

    if vessel_worn {
        state.last_surprise_delta = Some(format!(
            "察覺手中「{}」分寸微鈍，先改道{}修整器況再續原訂差事",
            vessel.display_name, vessel.maintenance_venue_keyword
        ));
        state.plan_steps = [
            format!("先赴{}修整「{}」", vessel.maintenance_venue_keyword, vessel.display_name),
            format!("重返{s_clean}完成手頭耽擱的活計"),
            format!("收工後與【{}】核對今日人情帳", state.hooks.first().map(|h| h.target_name.as_str()).unwrap_or("街坊")),
        ];
        state.plan_cursor = 0;
    } else if let Some(actual_item) = scarce_label
        && !actual_item.is_empty()
        && actual_item != state.belief_scarcity_item
    {
        let old_item = state.belief_scarcity_item.clone();
        state.belief_scarcity_item = actual_item.to_string();
        state.last_surprise_delta = Some(format!(
            "抵埠始知{s_clean}行情已由「{old_item}」轉缺「{actual_item}」，當場改換採辦與趕製標的"
        ));
        state.plan_steps = derive_default_3step_plan(
            role_hint,
            s_clean,
            vessel.display_name,
            state.cornerstone,
            actual_item,
        );
        state.plan_cursor = 1;
    } else if !state.plan_steps.is_empty() {
        state.plan_cursor = (state.plan_cursor + 1) % (state.plan_steps.len() as u8);
        if state.plan_cursor == 0 {
            state.last_surprise_delta = None;
        }
    }

    if state.stress >= 72 {
        state.stress_break_active = true;
    }
    let image = if has_affliction {
        DecisionImageKind::HealOrTend
    } else if state.hunger >= 64 || (matches!(hour, 7 | 12 | 18) && state.hunger >= 35) {
        DecisionImageKind::Eat
    } else if state.thirst >= 62 || (matches!(hour, 10 | 15) && state.thirst >= 38) {
        DecisionImageKind::Drink
    } else if state.fatigue >= 68 || state.stress >= 76 || !(6..=21).contains(&hour) {
        DecisionImageKind::SleepOrRest
    } else if vessel_worn {
        DecisionImageKind::MendOrEquip
    } else if (scarce_ratio >= 1.25 || scarce_label.is_some())
        && state.hunger < 68
        && state.thirst < 68
        && state.fatigue < 70
        && (7..=19).contains(&hour)
    {
        DecisionImageKind::WorkOrTrade
    } else if effective_mg <= 4 || (8..=17).contains(&hour) {
        DecisionImageKind::WorkOrTrade
    } else {
        DecisionImageKind::InteractOrUnwind
    };

    let mut base_eligible = eligible_actions_for_image(image, effective_mg);
    if image == DecisionImageKind::WorkOrTrade && !base_eligible.iter().any(|opt| opt.code == "work_perform_steady_daily_vocation") {
        if let Some(vocation_def) = PREDEFINED_ACTION_CATALOG.iter().find(|opt| opt.code == "work_perform_steady_daily_vocation") {
            base_eligible.push(vocation_def);
        }
    }
    let affordable_in_band: Vec<&'static ActionOptionDef> = base_eligible
        .iter()
        .copied()
        .filter(|opt| dynamic_action_cost_mg(settlement, opt) <= effective_mg.max(0))
        .collect();
    let eligible: Vec<&'static ActionOptionDef> = if !affordable_in_band.is_empty() {
        affordable_in_band
    } else {
        PREDEFINED_ACTION_CATALOG
            .iter()
            .filter(|opt| {
                opt.image == image
                    && effective_mg.max(0) >= opt.min_mg
                    && dynamic_action_cost_mg(settlement, opt) <= effective_mg.max(0)
            })
            .collect()
    };
    let margin = MarginBand::from_magnesium(effective_mg);

    let chosen = if image == DecisionImageKind::WorkOrTrade && scarce_label.is_some() {
        let preferred_code = match margin {
            MarginBand::Scarcity => {
                if matches!(state.cornerstone, Cornerstone::Order | Cornerstone::Profit)
                    || !(state.life_step_tick + seed).is_multiple_of(3)
                {
                    "work_perform_steady_daily_vocation"
                } else {
                    "work_scavenge_wild_firewood_and_scraps"
                }
            }
            MarginBand::Subsistence => {
                if (state.life_step_tick + seed).is_multiple_of(2) {
                    "work_peddle_gathered_goods_at_market"
                } else {
                    "work_restock_needed_raw_materials"
                }
            }
            MarginBand::Abundance => "work_cross_settlement_scarcity_arbitrage",
        };
        eligible
            .iter()
            .copied()
            .find(|opt| opt.code == preferred_code)
            .unwrap_or_else(|| {
                choose_action_from_eligible(
                    &eligible,
                    state.cornerstone,
                    state.life_step_tick,
                    seed,
                )
            })
    } else {
        choose_action_from_eligible(
            &eligible,
            state.cornerstone,
            state.life_step_tick,
            seed,
        )
    };

    if let Some(item_name) = scarce_label {
        state.belief_scarcity_item = item_name.to_string();
        state.belief_summary = if scarce_ratio >= 1.5 {
            format!("聽聞坊間「{item_name}」頗為緊俏，價錢比往常貴上不少")
        } else {
            format!("留意到街市上「{item_name}」銷路甚快，行情隱約看漲")
        };
    } else {
        state.belief_summary = "坊間各色物料供需平穩，行情如常".to_string();
    }

    state.decision_image_code = image.code().to_string();
    state.eligible_option_count = base_eligible.len() as u8;
    state.chosen_action_code = chosen.code.to_string();
    state.active_need = format!(
        "{}（懷揣 {} 枚鎂幣，{}）",
        image.label(),
        effective_mg.max(0),
        margin.label()
    );
    state.image_decision = format!(
        "衡量{}的處境，承接第 {} 回手頭活計，決定先「{}」",
        margin.label(),
        state.same_task_repetitions,
        chosen.action_title
    );
    state.situated_action = format!(
        "承接第 {} 回熟稔手感，{}",
        state.same_task_repetitions,
        chosen.action_behavior
    );

    if let Ok(mut map) = LIVE_NPC_CAUSAL_STATES.write() {
        map.insert(entity_id.to_string(), state.clone());
    }

    (state, chosen)
}

/// 確保因果狀態中之可選動作數與擇定動作嚴格對齊當前有效鎂幣（匱乏 2 項、溫飽 3 項、有餘裕 5~6 項）
pub fn sync_causal_state_with_magnesium(
    state: &mut NpcCausalState,
    magnesium: i32,
    seed: u64,
) {
    let image = match state.decision_image_code.as_str() {
        "eat" => DecisionImageKind::Eat,
        "drink" => DecisionImageKind::Drink,
        "sleep_or_rest" => DecisionImageKind::SleepOrRest,
        "heal_or_tend" => DecisionImageKind::HealOrTend,
        "mend_or_equip" => DecisionImageKind::MendOrEquip,
        "interact_or_unwind" => DecisionImageKind::InteractOrUnwind,
        _ => DecisionImageKind::WorkOrTrade,
    };
    let margin = MarginBand::from_magnesium(magnesium);
    let eligible = eligible_actions_for_image(image, magnesium);
    let chosen = eligible
        .iter()
        .copied()
        .find(|opt| opt.code == state.chosen_action_code)
        .unwrap_or_else(|| {
            choose_action_from_eligible(
                &eligible,
                state.cornerstone,
                state.life_step_tick,
                seed,
            )
        });
    state.decision_image_code = image.code().to_string();
    state.eligible_option_count = eligible.len() as u8;
    state.chosen_action_code = chosen.code.to_string();
    state.active_need = format!(
        "{}（懷揣 {} 枚鎂幣，{}）",
        image.label(),
        magnesium.max(0),
        margin.label()
    );
    state.image_decision = format!(
        "衡量{}的處境，承接第 {} 回手頭活計，決定先「{}」",
        margin.label(),
        state.same_task_repetitions,
        chosen.action_title
    );
    state.situated_action = format!(
        "承接第 {} 回熟稔手感，{}",
        state.same_task_repetitions,
        chosen.action_behavior
    );
}

/// 依代碼查閱預擬行為選項定義
pub fn lookup_action_option(code: &str) -> Option<&'static ActionOptionDef> {
    PREDEFINED_ACTION_CATALOG.iter().find(|opt| opt.code == code)
}

fn find_real_settlement_venue(settlement: &str, keywords: &[char]) -> Option<String> {
    for slot in crate::store::OUTER_HOUSEHOLD_SLOTS {
        for candidate in [slot.work_name, slot.errand_name] {
            if candidate.starts_with(settlement)
                && let Some((_, leaf)) = candidate.split_once('・')
                && keywords.iter().any(|&kw| leaf.contains(kw))
            {
                return Some(leaf.to_string());
            }
        }
    }
    None
}

pub fn coherent_place_for_venue_req(venue_req: VenueRequirement, dest_name: &str) -> String {
    let settlement = clean_settlement_name(dest_name);
    let dest_leaf = dest_name.rsplit('・').next().unwrap_or(dest_name).trim();
    match venue_req {
        VenueRequirement::HomeOrKitchen => {
            let is_home = (dest_leaf.contains('宅')
                || dest_leaf.contains('居')
                || dest_leaf.contains('屋')
                || dest_leaf.contains('家')
                || dest_leaf.contains('宿')
                || dest_leaf.contains('寮')
                || dest_leaf.contains('舍'))
                && !dest_leaf.contains("食堂")
                && !dest_leaf.contains("飯")
                && !dest_leaf.contains("茶屋")
                && !dest_leaf.contains("湯屋")
                && !dest_leaf.contains("商");
            if is_home {
                dest_leaf.to_string()
            } else {
                format!("{settlement}自家屋舍廚下")
            }
        }
        VenueRequirement::WildOrOutskirts => {
            let is_wild = (dest_leaf.contains('林')
                || dest_leaf.contains('溪')
                || dest_leaf.contains('谷')
                || dest_leaf.contains('山')
                || dest_leaf.contains('坡')
                || dest_leaf.contains('岸')
                || dest_leaf.contains('甸')
                || dest_leaf.contains('道')
                || dest_leaf.contains('嶺')
                || dest_leaf.contains('野')
                || dest_leaf.contains('灣')
                || dest_leaf.contains('潭'))
                && !dest_leaf.contains("商行")
                && !dest_leaf.contains("貨運")
                && !dest_leaf.contains("工坊")
                && !dest_leaf.contains("食堂")
                && !dest_leaf.contains("茶屋");
            if is_wild {
                dest_leaf.to_string()
            } else {
                format!("{settlement}外圍山林近郊")
            }
        }
        VenueRequirement::CanteenOrStall => {
            if dest_leaf.contains("食堂")
                || dest_leaf.contains('飯')
                || dest_leaf.contains('麵')
                || dest_leaf.contains('攤')
                || dest_leaf.contains("熱食")
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "三水大食堂".to_string()
            } else {
                format!("{settlement}街角熱食麵攤")
            }
        }
        VenueRequirement::TeahouseOrTavern => {
            if dest_leaf.contains('茶')
                || dest_leaf.contains('酒')
                || dest_leaf.contains("旅社")
                || dest_leaf.contains("茶寮")
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "東橋旅社與山風茶屋".to_string()
            } else {
                find_real_settlement_venue(settlement, &['茶', '食', '飯', '湯', '旅'])
                    .unwrap_or_else(|| format!("{settlement}路邊茶寮"))
            }
        }
        VenueRequirement::WellOrStream => {
            if dest_leaf.contains('井')
                || dest_leaf.contains('泉')
                || dest_leaf.contains('溪')
                || dest_leaf.contains('水')
                || dest_leaf.contains('圳')
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "三水口橋頭".to_string()
            } else {
                find_real_settlement_venue(settlement, &['井', '泉', '溪', '水', '圳', '渡'])
                    .unwrap_or_else(|| format!("{settlement}街角山泉公井"))
            }
        }
        VenueRequirement::Bathhouse => {
            if dest_leaf.contains('浴') || dest_leaf.contains('湯') {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "泉香溫水浴場".to_string()
            } else {
                find_real_settlement_venue(settlement, &['浴', '湯', '泉', '旅'])
                    .unwrap_or_else(|| format!("{settlement}溫水石湯屋"))
            }
        }
        VenueRequirement::InnOrHome => {
            if dest_leaf.contains('旅')
                || dest_leaf.contains('宿')
                || dest_leaf.contains("旅社")
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "東橋旅社".to_string()
            } else {
                find_real_settlement_venue(settlement, &['旅', '宿', '棧', '寮', '居'])
                    .unwrap_or_else(|| format!("{settlement}旅社通鋪"))
            }
        }
        VenueRequirement::Clinic => {
            if dest_leaf.contains('診') || dest_leaf.contains('醫') || dest_leaf.contains('藥') {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "溫氏百草診所".to_string()
            } else {
                find_real_settlement_venue(settlement, &['診', '醫', '藥', '堂'])
                    .unwrap_or_else(|| format!("{settlement}草木藥鋪診所"))
            }
        }
        VenueRequirement::ForgeOrWorkshop => {
            if dest_leaf.contains('坊')
                || dest_leaf.contains('鍛')
                || dest_leaf.contains('鐵')
                || dest_leaf.contains('鋪')
                || dest_leaf.contains('室')
                || dest_leaf.contains('所')
                || dest_leaf.contains('廠')
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "岑氏第一鍛造工坊".to_string()
            } else {
                find_real_settlement_venue(settlement, &['坊', '鍛', '鐵', '木', '窯', '作', '鋪'])
                    .unwrap_or_else(|| format!("{settlement}百工修繕坊"))
            }
        }
        VenueRequirement::DockOrStation => {
            if dest_leaf.contains('站')
                || dest_leaf.contains('運')
                || dest_leaf.contains("碼頭")
                || dest_leaf.contains('港')
                || dest_leaf.contains('埠')
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "西門車馬聯運總站".to_string()
            } else {
                find_real_settlement_venue(settlement, &['站', '運', '碼', '港', '埠', '渡'])
                    .unwrap_or_else(|| format!("{settlement}車馬聯運貨站"))
            }
        }
        VenueRequirement::Market => {
            if dest_leaf.contains('市')
                || dest_leaf.contains('商')
                || dest_leaf.contains('集')
                || dest_leaf.contains('攤')
                || dest_leaf.contains('店')
                || dest_leaf.contains('行')
                || dest_leaf.contains('鋪')
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "三水總商行市集".to_string()
            } else {
                find_real_settlement_venue(settlement, &['市', '商', '集', '攤', '店', '行', '鋪'])
                    .unwrap_or_else(|| format!("{settlement}街心市集攤區"))
            }
        }
        VenueRequirement::BridgeOrPlaza => {
            if dest_leaf.contains('橋')
                || dest_leaf.contains('坪')
                || dest_leaf.contains("廣場")
                || dest_leaf.contains('廊')
                || dest_leaf.contains('岬')
                || dest_leaf.contains('閣')
            {
                dest_leaf.to_string()
            } else if settlement == "文曜" {
                "三水匯流大石橋".to_string()
            } else {
                format!("{settlement}街心石坪廊下")
            }
        }
        VenueRequirement::OwnWorkplace => {
            if dest_leaf.is_empty() {
                format!("{settlement}案頭崗位")
            } else {
                dest_leaf.to_string()
            }
        }
    }
}

/// 格式化預擬定點停留活動（在前端與狀態表中真實反應依現況與選項表擇定的自然人話活動描述）
pub fn format_predefined_stay_activity(
    _dest_name: &str,
    causal: &NpcCausalState,
    _magnesium: i32,
) -> String {
    let opt = lookup_action_option(&causal.chosen_action_code).unwrap_or(&PREDEFINED_ACTION_CATALOG[0]);
    let beh = opt.action_behavior.trim();
    if beh.starts_with('正') {
        beh.to_string()
    } else {
        format!("正{}", beh)
    }
}

pub static LIVE_NPC_CAUSAL_STATES: LazyLock<RwLock<HashMap<String, NpcCausalState>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

const HOOK_PEER_NAMES: &[&str] = &[
    "謝宇翔", "林雅筑", "許家豪", "陳冠宇", "黃詩涵", "張哲銘", "李佳穎", "王俊傑",
    "吳佩珊", "劉建宏", "蔡欣怡", "楊承翰", "鄭雅婷", "郭志豪", "洪淑芬", "邱冠廷",
];

pub fn clean_settlement_name(raw: &str) -> &'static str {
    const PLACES: &[(&str, &str)] = &[
        ("風舶角", "風舶角"),
        ("風舶嶼", "風舶嶼"),
        ("大圳頭", "大圳頭"),
        ("半嶺鎮", "半嶺鎮"),
        ("南哨", "南哨"),
        ("老堰頭", "老堰頭"),
        ("北嶺腳", "北嶺腳"),
        ("霜林口", "霜林口"),
        ("風櫃尾", "風櫃尾"),
        ("烏石鼻", "烏石鼻"),
        ("牛埔", "牛埔"),
        ("紅土崁", "紅土崁"),
        ("紅土", "紅土崁"),
        ("三號坑口", "三號坑口"),
        ("水尾寮", "水尾寮"),
        ("長風草甸", "長風草甸"),
        ("草甸", "長風草甸"),
        ("古河巨木林", "古河巨木林"),
        ("赤石坡", "赤石坡"),
        ("落日岬", "落日岬"),
        ("驚濤岩岸", "驚濤岩岸"),
        ("巨藻淺灣", "巨藻淺灣"),
        ("黑石灣", "黑石灣"),
        ("西麓林道", "西麓林道"),
        ("清泉溪谷", "清泉溪谷"),
        ("翠冠巨木林", "翠冠巨木林"),
        ("北坡雲杉林", "北坡雲杉林"),
        ("淺澤泥口", "淺澤泥口"),
        ("白茫洲", "白茫洲"),
        ("貼山深潭", "貼山深潭"),
        ("青籠關", "青籠關"),
        ("西嶺礦谷", "西嶺礦谷"),
        ("東海峭壁", "東海峭壁"),
        ("雲腰雪林", "雲腰雪林"),
        ("白塔雪巔", "白塔雪巔"),
        ("講堂街", "文曜"),
        ("三水", "文曜"),
        ("文曜", "文曜"),
    ];
    for &(kw, canonical) in PLACES {
        if raw.contains(kw) {
            return canonical;
        }
    }
    "文曜"
}

fn seed_hooks_for_resident(
    role_hint: &str,
    settlement: &str,
    seed: u64,
) -> Vec<RelationalHook> {
    let s = clean_settlement_name(settlement);
    let peer_a = HOOK_PEER_NAMES[(seed as usize) % HOOK_PEER_NAMES.len()];
    let peer_b = HOOK_PEER_NAMES[((seed as usize) + 5) % HOOK_PEER_NAMES.len()];

    let primary = if role_hint.contains("獵") || role_hint.contains("採") || role_hint.contains("巡") {
        RelationalHook {
            kind_label: "恩情".to_string(),
            target_name: peer_a.to_string(),
            summary: format!("欠【{peer_a}】一回山道解圍之恩，遇其合社召集必優先助陣"),
            intensity: 4,
            target_id: String::new(),
            updated_at_unix: 0,
        }
    } else if role_hint.contains("鐵") || role_hint.contains("木") || role_hint.contains("學徒") {
        RelationalHook {
            kind_label: "師徒代償".to_string(),
            target_name: peer_a.to_string(),
            summary: format!("與同門【{peer_a}】共擔{s}工坊損耗，見其器鈍必出手幫襯"),
            intensity: 4,
            target_id: String::new(),
            updated_at_unix: 0,
        }
    } else if role_hint.contains("商") || role_hint.contains("主事") || role_hint.contains("帳房") {
        RelationalHook {
            kind_label: "直供紅契".to_string(),
            target_name: peer_a.to_string(),
            summary: format!("與【{peer_a}】訂有{s}大宗物料直供契，貨到優先交割"),
            intensity: 3,
            target_id: String::new(),
            updated_at_unix: 0,
        }
    } else if role_hint.contains("家眷") || role_hint.contains("內當家") || role_hint.contains("食堂") {
        RelationalHook {
            kind_label: "熟客欠單".to_string(),
            target_name: peer_a.to_string(),
            summary: format!("記著街坊【{peer_a}】在{s}留下的熟客人情帳，逢集優先留貨"),
            intensity: 3,
            target_id: String::new(),
            updated_at_unix: 0,
        }
    } else {
        RelationalHook {
            kind_label: "同袍信約".to_string(),
            target_name: peer_a.to_string(),
            summary: format!("與{s}【{peer_a}】約定互通坊間行情與山海異兆"),
            intensity: 3,
            target_id: String::new(),
            updated_at_unix: 0,
        }
    };

    let secondary = match (seed / 7) % 3 {
        0 => RelationalHook {
            kind_label: "宿怨壓價".to_string(),
            target_name: peer_b.to_string(),
            summary: format!("曾遭【{peer_b}】在{s}市集壓價挑剔，交手時格外留心成色"),
            intensity: 2,
            target_id: String::new(),
            updated_at_unix: 0,
        },
        1 => RelationalHook {
            kind_label: "熟客欠單".to_string(),
            target_name: peer_b.to_string(),
            summary: format!("手頭還記著與【{peer_b}】待結的半筆物料尾款"),
            intensity: 2,
            target_id: String::new(),
            updated_at_unix: 0,
        },
        _ => RelationalHook {
            kind_label: "恩情".to_string(),
            target_name: peer_b.to_string(),
            summary: format!("雨夜曾受【{peer_b}】借火避雨之惠，心裡一直記掛著還禮"),
            intensity: 3,
            target_id: String::new(),
            updated_at_unix: 0,
        },
    };

    vec![primary, secondary]
}

pub fn derive_default_3step_plan(
    role_hint: &str,
    settlement: &str,
    vessel_name: &str,
    cornerstone: Cornerstone,
    belief_item: &str,
) -> [String; 3] {
    let s = clean_settlement_name(settlement);
    let item = if belief_item.trim().is_empty() { "山海常備物料" } else { belief_item.trim() };
    let is_blade = vessel_name.contains('刀') || vessel_name.contains('鎬') || vessel_name.contains('剪');

    if role_hint.contains("獵") || role_hint.contains("採") || role_hint.contains("樵") {
        let prep = if is_blade {
            format!("先檢視「{vessel_name}」刃口與行囊繩索")
        } else {
            format!("先收整隨身「{vessel_name}」與採集行囊")
        };
        [
            prep,
            format!("入山勘查獸徑並採集「{item}」與山林原材"),
            format!("返{s}市集交割換取鎂幣並歇腳"),
        ]
    } else if role_hint.contains("商") || role_hint.contains("主事") || role_hint.contains("執事") || role_hint.contains("帳房") {
        [
            format!("撥校「{vessel_name}」盤點{s}庫房現貨底數"),
            format!("收攏坊間急缺的「{item}」並調撥價差"),
            format!("結清今日熟客欠單後赴{s}食堂用膳"),
        ]
    } else if (role_hint.contains("書") || role_hint.contains("書院") || role_hint.contains("講") || role_hint.contains("學者") || role_hint.contains("算")) && !role_hint.contains("學徒") {
        [
            format!("展平「{vessel_name}」核對{s}今日風物與卷宗"),
            format!("於{s}講堂書齋纂錄「{item}」行情與山海考據"),
            format!("散學後巡看{s}街坊並與同道印證筆記"),
        ]
    } else if role_hint.contains("醫") || role_hint.contains("藥") || role_hint.contains("診") {
        [
            format!("拭淨「{vessel_name}」並清點{s}診所草藥存量"),
            format!("搗製應時傷藥並為{s}傷患切脈調理"),
            format!("赴{s}市集探問「{item}」與短缺藥引行情"),
        ]
    } else if role_hint.contains("水輪") || role_hint.contains("石磨") || role_hint.contains("磨坊") || role_hint.contains("碾") {
        [
            format!("開啟{s}引水閘門，以「{vessel_name}」校準上下磨盤齒距與銅網篩眼"),
            format!("藉湍急水輪分動大磨，將大圳頭新穀與「{item}」細碾分篩並熬製溫米漿"),
            format!("封妥米漿保溫桶發往{s}早市，晚間赴{s}堰渠水畔食堂與鄰近農戶核對明日水門時刻"),
        ]
    } else if role_hint.contains("鐵") || role_hint.contains("鍛") || role_hint.contains("木工") || role_hint.contains("木匠") || role_hint.contains("木器") || role_hint.contains("竹") || role_hint.contains("工坊") {
        [
            format!("先於{s}工坊預熱爐火並校準「{vessel_name}」"),
            format!("趕製坊間急需的「{item}」器物批次"),
            format!("核對工坊成色後赴{s}茶寮舒緩筋骨"),
        ]
    } else if role_hint.contains("家眷") || role_hint.contains("內當家") || role_hint.contains("持家") {
        [
            format!("挽起「{vessel_name}」清點家中米糧與柴薪"),
            format!("赴{s}市集置辦「{item}」與今日鮮蔬"),
            format!("返家備妥熱飯並料理家宅內外整潔"),
        ]
    } else {
        [
            format!("依【{}】本色整頓隨身「{vessel_name}」", cornerstone.label()),
            format!("於{s}按部就班完成「{item}」相關差事"),
            format!("收工結清荷包後赴{s}街坊歇息敘話"),
        ]
    }
}

/// 格式化生命直線時間刻度標記（證明生命是一條直線，時間永不歸零）
pub fn format_time_scale_mark(state: &NpcCausalState, now_unix: i64) -> String {
    let day = if now_unix > 0 {
        ((now_unix / 3600) % 1000).max(1)
    } else {
        1
    };
    format!(
        "入島第 {} 日・歷練第 {} 步（同業累計第 {} 回）",
        day, state.life_step_tick, state.same_task_repetitions
    )
}


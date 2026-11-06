/// 取得或初始化單一住民的因果狀態
pub fn get_or_init_causal_state(
    entity_id: &str,
    role_hint: &str,
    settlement: &str,
    seed: u64,
) -> NpcCausalState {
    if let Ok(guard) = LIVE_NPC_CAUSAL_STATES.read()
        && let Some(existing) = guard.get(entity_id).cloned()
    {
        return existing;
    }
    let s = clean_settlement_name(settlement);
    let vessel = resolve_vocational_vessel(role_hint);
    let cornerstone = Cornerstone::from_role_and_seed(role_hint, seed);
    let belief_item = crate::game::object_ecology::scarcest_item_for_settlement(s)
        .map(|(_id, nm, _cur, _max, _pr)| nm)
        .unwrap_or_else(|| "山野鮮藥草".to_string());
    let plan_steps = derive_default_3step_plan(
        role_hint,
        s,
        vessel.display_name,
        cornerstone,
        &belief_item,
    );
    let now_unix = crate::game::now_unix();
    let life_step_tick = ((now_unix.max(0) as u64) / 60) + (seed % 900) + 120;
    let same_task_repetitions = ((seed % 180) + 24) as u32;
    let carryover_from_yesterday = format!(
        "昨日第 {} 回工序留下的器物手感與未竟尾款延續至今晨，晨起後順著昨日步調繼續忙活",
        same_task_repetitions
    );
    let initial_mg = compute_starting_wallet(seed, role_hint);
    let init_image = resolve_decision_image("", false, false, (12 + (seed % 28)) as u8, life_step_tick);
    let eligible = eligible_actions_for_image(init_image, initial_mg);
    let chosen = choose_action_from_eligible(&eligible, cornerstone, life_step_tick, seed);
    let margin = MarginBand::from_magnesium(initial_mg);
    let active_need = format!(
        "{}（懷揣 {} 枚鎂幣，{}）",
        init_image.label(),
        initial_mg,
        margin.label()
    );
    let image_decision = format!(
        "衡量{}的處境，依【{}】脾性決定先「{}」",
        margin.label(),
        cornerstone.label(),
        chosen.action_title
    );
    let situated_action = format!(
        "身在{}，{}",
        s,
        chosen.action_behavior
    );
    let state = NpcCausalState {
        cornerstone,
        stress: (12 + (seed % 28)) as u8,
        stress_break_active: false,
        hunger: ((seed % 35) + 25) as u8,
        thirst: (((seed / 3) % 40) + 15) as u8,
        fatigue: (((seed / 7) % 40) + 10) as u8,
        plan_steps,
        plan_cursor: ((seed / 3) % 3) as u8,
        belief_scarcity_item: belief_item.clone(),
        last_surprise_delta: None,
        hooks: seed_hooks_for_resident(role_hint, settlement, seed),
        life_step_tick,
        same_task_repetitions,
        carryover_from_yesterday,
        decision_image_code: init_image.code().to_string(),
        eligible_option_count: eligible.len() as u8,
        chosen_action_code: chosen.code.to_string(),
        active_need,
        image_decision,
        situated_action,
        belief_summary: format!("「{belief_item}」為當前聚落代表物資"),
        active_ripples: Vec::new(),
        immune_fact_ids: Vec::new(),
    };
    if let Ok(mut map) = LIVE_NPC_CAUSAL_STATES.write() {
        map.entry(entity_id.to_string())
            .or_insert_with(|| state.clone());
    }
    state
}

/// 住民抵達目標場所時，推進三步因果計畫鏈、信念落差（ΔE）調適與壓力閾值轉折，
/// 並將「需求產生意象決策，決策依現況產生行為」及不可逆之「時間刻度」推進至下一階段。
pub fn advance_causal_state_on_arrival(
    entity_id: &str,
    role_hint: &str,
    settlement: &str,
    seed: u64,
    arrived_dest_name: &str,
    consecutive_work_streaks: u8,
    has_affliction: bool,
    vessel_worn: bool,
    magnesium: i32,
    actual_scarcity_item: Option<&str>,
) -> NpcCausalState {
    let mut current = get_or_init_causal_state(entity_id, role_hint, settlement, seed);
    let vessel = resolve_vocational_vessel(role_hint);
    let s = if settlement.trim().is_empty() { "文曜" } else { settlement.trim() };

    // 0. 生命是一條直線：時間刻度嚴格單調遞增（永不取模歸零）
    current.life_step_tick = current.life_step_tick.saturating_add(1);

    // 1. 更新壓力（Stress 0..=100）：連軸工作、傷勢、荷包見底或器物磨損會推升壓力；洗沐、用膳、就醫則舒緩
    let is_rest_or_heal = arrived_dest_name.contains("浴")
        || arrived_dest_name.contains("湯")
        || arrived_dest_name.contains("食堂")
        || arrived_dest_name.contains("茶")
        || arrived_dest_name.contains("診")
        || arrived_dest_name.contains("居")
        || arrived_dest_name.contains("宅");

    if is_rest_or_heal {
        current.stress = current.stress.saturating_sub(24);
        current.stress_break_active = false;
        // 每個人每天起床不是歸零重整，生命是一條直線，記錄休歇與晨起未竟之延續
        current.carryover_from_yesterday = format!(
            "昨日第 {} 回工序留下的器物手感與未竟尾款延續至今晨，歇息調養後繼續推進手頭活計",
            current.same_task_repetitions
        );
    } else {
        let mut delta: u8 = 6 + consecutive_work_streaks.saturating_mul(5);
        if has_affliction {
            delta = delta.saturating_add(14);
        }
        if vessel_worn && current.cornerstone == Cornerstone::Craft {
            delta = delta.saturating_add(15);
        }
        if magnesium < 12 {
            delta = delta.saturating_add(10);
        }
        current.stress = current.stress.saturating_add(delta).min(100);

        // 同業日課與工序累計次數增加（即便每天做的都是同樣的事，時間刻度仍然是不同的）
        let is_work_or_forage = arrived_dest_name.contains("工坊")
            || arrived_dest_name.contains("獵")
            || arrived_dest_name.contains("礦")
            || arrived_dest_name.contains("林")
            || arrived_dest_name.contains("市集")
            || arrived_dest_name.contains("商")
            || arrived_dest_name.contains("書院")
            || arrived_dest_name.contains("講堂")
            || arrived_dest_name.contains("草甸")
            || arrived_dest_name.contains("道")
            || arrived_dest_name.contains("坑口")
            || consecutive_work_streaks > 0;
        if is_work_or_forage {
            current.same_task_repetitions = current.same_task_repetitions.saturating_add(1);
        }
        current.carryover_from_yesterday = format!(
            "昨日第 {} 回工序留下的器物手感與未竟尾款延續至今晨，晨起後順著昨日步調繼續忙活",
            current.same_task_repetitions
        );
    }

    // 2. 檢查信念落差（Belief Surprise ΔE）：若實際急缺物資與原先預期不同，或器物突發磨損，立即動態改寫三步計畫鏈
    if vessel_worn {
        current.last_surprise_delta = Some(format!(
            "察覺手中「{}」分寸微鈍，先改道{}修整器況再續原訂差事",
            vessel.display_name, vessel.maintenance_venue_keyword
        ));
        current.plan_steps = [
            format!("先赴{}修整「{}」", vessel.maintenance_venue_keyword, vessel.display_name),
            format!("重返{s}完成手頭耽擱的活計"),
            format!("收工後與【{}】核對今日人情帳", current.hooks.first().map(|h| h.target_name.as_str()).unwrap_or("街坊")),
        ];
        current.plan_cursor = 0;
    } else if let Some(actual_item) = actual_scarcity_item
        && !actual_item.is_empty()
        && actual_item != current.belief_scarcity_item
    {
        let old_item = current.belief_scarcity_item.clone();
        current.belief_scarcity_item = actual_item.to_string();
        current.belief_summary = format!("聽聞坊間「{actual_item}」緊缺看漲，當場改換趕製標的");
        current.last_surprise_delta = Some(format!(
            "抵埠始知{s}行情已由「{old_item}」轉缺「{actual_item}」，當場改換採辦與趕製標的"
        ));
        current.plan_steps = derive_default_3step_plan(
            role_hint,
            s,
            vessel.display_name,
            current.cornerstone,
            actual_item,
        );
        current.plan_cursor = 1;
    } else {
        current.plan_cursor = (current.plan_cursor + 1) % 3;
        if current.plan_cursor == 0 {
            current.last_surprise_delta = None;
            // 進入下一段直線刻度時，計畫鏈帶著最新累計回數向前推進，而非回到原點死循環
            let base_steps = derive_default_3step_plan(
                role_hint,
                s,
                vessel.display_name,
                current.cornerstone,
                &current.belief_scarcity_item,
            );
            current.plan_steps = [
                format!("承接第 {} 回手感，{}", current.same_task_repetitions, base_steps[0]),
                base_steps[1].clone(),
                base_steps[2].clone(),
            ];
        }
    }

    // 3. 壓力破防判定（Stress >= 72）
    if current.stress >= 72 {
        current.stress_break_active = true;
        let (break_title, break_step, _) =
            current.cornerstone.stress_break_directive(s, vessel.display_name);
        current.last_surprise_delta = Some(format!(
            "連番負荷觸發【{}】本色轉折：{}",
            current.cornerstone.label(),
            break_title
        ));
        current.plan_steps[current.plan_cursor as usize] = break_step;
    }

    // 4. 事先擬好「決策意象」與「行為動作」選項，等待條件（有錢沒錢？有多少錢？現況如何？）符合時做出選擇
    let image = resolve_decision_image(
        arrived_dest_name,
        has_affliction,
        vessel_worn,
        current.stress,
        current.life_step_tick,
    );
    let margin = MarginBand::from_magnesium(magnesium);
    let eligible = eligible_actions_for_image(image, magnesium);
    let chosen = choose_action_from_eligible(
        &eligible,
        current.cornerstone,
        current.life_step_tick,
        seed,
    );
    current.decision_image_code = image.code().to_string();
    current.eligible_option_count = eligible.len() as u8;
    current.chosen_action_code = chosen.code.to_string();

    current.active_need = format!(
        "{}（懷揣 {} 枚鎂幣，{}）",
        image.label(),
        magnesium.max(0),
        margin.label()
    );

    current.image_decision = format!(
        "衡量{}的處境，承接第 {} 回手頭活計，決定先「{}」",
        margin.label(),
        current.same_task_repetitions,
        chosen.action_title
    );

    let vessel_desc = if vessel_worn {
        format!("隨身「{}」微有磨損", vessel.display_name)
    } else {
        format!("隨身「{}」利索稱手", vessel.display_name)
    };
    current.situated_action = format!(
        "身處{}（{}），{}",
        arrived_dest_name,
        vessel_desc,
        chosen.action_behavior
    );
    if !is_rest_or_heal {
        let actor_short = crate::store::resolve_npc_person_name(entity_id, role_hint, None);
        let host_opt = if !actor_short.is_empty() && actor_short != "陌生人" {
            Some(actor_short.as_str())
        } else {
            None
        };
        if let Some((ev, sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
            arrived_dest_name,
            host_opt,
            crate::game::now_unix(),
        ) {
            current.situated_action = format!(
                "{}（當值事件：【{}】，與{}協力推進）",
                current.situated_action,
                ev.event_title,
                sp.partner_slot_actor,
            );
        }
    }
    if let Ok(mut map) = LIVE_NPC_CAUSAL_STATES.write() {
        map.insert(entity_id.to_string(), current.clone());
    }
    current
}

/// 按需十層體感微動作與內心念頭合成器（僅於 API 或觀測時即時合成，背景零浪費）
#[expect(
    clippy::too_many_arguments,
    reason = "10-layer somatic renderer explicitly takes the 10 live state dimensions on demand"
)]
pub fn render_somatic_10layer_thought(
    entity_id: &str,
    role_hint: &str,
    settlement: &str,
    seed: u64,
    activity: &str,
    held_vessel: &str,
    breath_habit: &str,
    afflictions: &[String],
    hunting_party: &str,
    magnesium: i32,
) -> String {
    let causal = get_or_init_causal_state(entity_id, role_hint, settlement, seed);
    let vessel_spec = resolve_vocational_vessel(role_hint);
    let vessel_short = if held_vessel.trim().is_empty() {
        vessel_spec.display_name
    } else {
        held_vessel.split('（').next().unwrap_or(vessel_spec.display_name)
    };

    // Layer 1 & 2: 若正處於性格錨點高壓破防，直接呈現鮮明的性格轉折體感
    if causal.stress_break_active {
        let (_, _, somatic_action) = causal
            .cornerstone
            .stress_break_directive(settlement, vessel_short);
        return format!(
            "{}；連日操勞下心頭頗感吃緊，承接第 {} 回累積的疲憊，心裡盤算先「{}」",
            somatic_action,
            causal.same_task_repetitions,
            causal.plan_steps[(causal.plan_cursor as usize) % 3]
        );
    }

    // Layer 3: 若正參與合社圍獵小隊，呈現小隊戰術協同與呼吸手勢
    if !hunting_party.is_empty() {
        let hook_peer = causal
            .hooks
            .first()
            .map(|h| h.target_name.as_str())
            .unwrap_or("同袍");
        return format!(
            "運轉「{}」壓住腳步聲，憑第 {} 回入山累積的老練手感扣緊「{}」，一面配合獵團合圍方位，一面留意同袍【{}】側翼動靜",
            breath_habit,
            causal.same_task_repetitions,
            vessel_short,
            hook_peer
        );
    }

    // Layer 4: 若身上帶有部位傷勢，體感微動作優先反映傷處代償與護具動作
    if let Some(first_aff) = afflictions.first() {
        return format!(
            "因「{}」隱隱作痛而微調「{}」吐納節奏，換手穩住「{}」，盤算著把第 {} 回手頭「{}」收尾便去敷藥歇息",
            first_aff,
            breath_habit,
            vessel_short,
            causal.same_task_repetitions,
            causal.plan_steps[(causal.plan_cursor as usize) % 3]
        );
    }

    // Layer 5: 若新發生信念驚奇（ΔE 預期落差），呈現隨機應變的眼神與手勢
    if let Some(surprise) = &causal.last_surprise_delta {
        return format!(
            "{}；遂一面運轉「{}」穩住心神，一面憑第 {} 回熟稔手感握緊「{}」，改按現況推進「{}」（下一步：{}）",
            surprise,
            breath_habit,
            causal.same_task_repetitions,
            vessel_short,
            causal.plan_steps[(causal.plan_cursor as usize) % 3],
            causal.plan_steps[((causal.plan_cursor as usize) + 1) % 3]
        );
    }

    // Layer 6..10: 結合器物微動作 + 呼吸節律 + 三步因果計畫鏈 + 人情鉤子 + 荷包成色 + 時間刻度
    let hand_kinematic = match (seed as usize + causal.plan_cursor as usize) % 5 {
        0 => format!("指節輕叩手中的「{vessel_short}」確核分寸"),
        1 => format!("順著「{breath_habit}」吐納節律，穩穩託住隨身「{vessel_short}」"),
        2 => format!("低頭拂去「{vessel_short}」上的浮塵，目光掃過四周動靜"),
        3 => format!("手腕微沉、將「{vessel_short}」收攏稱手位置，摸了摸荷包裡的 {magnesium} 枚鎂幣"),
        _ => format!("依【{}】習慣先將「{vessel_short}」理順，神色篤定", causal.cornerstone.label()),
    };

    let current_step = &causal.plan_steps[(causal.plan_cursor as usize) % 3];
    let next_step = &causal.plan_steps[((causal.plan_cursor as usize) + 1) % 3];
    let hook_note = causal
        .hooks
        .first()
        .map(|h| format!("心底同時記掛著「{}」", h.summary))
        .unwrap_or_default();

    if activity.contains("安歇") || activity.contains("熟睡") || activity.contains("就寢") {
        let sleep_pose = match (seed as usize) % 4 {
            0 => format!("將「{vessel_short}」妥善置於榻畔，綿長運轉「{breath_habit}」"),
            1 => format!("仔細收好「{vessel_short}」與荷包（{magnesium} 枚鎂幣），伴隨「{breath_habit}」安然歇下"),
            2 => format!("依【{}】習慣把「{vessel_short}」擺在伸手可及處，調勻「{breath_habit}」養神", causal.cornerstone.label()),
            _ => format!("吹熄燈火、撫平衣角，將「{vessel_short}」收攏枕側，在「{breath_habit}」節律中沉沉睡去"),
        };
        let continuity = format!(
            "日間第 {} 回工序的手感與帳目已收妥，明早醒來接著往下推進「{}」",
            causal.same_task_repetitions, next_step
        );
        if !hook_note.is_empty() && seed % 2 == 0 {
            format!("{sleep_pose}，{continuity}，{hook_note}")
        } else {
            format!("{sleep_pose}，{continuity}")
        }
    } else if !hook_note.is_empty() {
        format!(
            "{hand_kinematic}，承接昨日第 {} 回工序底數推進「{current_step}」（下一步：{next_step}），{hook_note}",
            causal.same_task_repetitions
        )
    } else {
        format!(
            "{hand_kinematic}，承接昨日第 {} 回工序底數專注於「{current_step}」，隨後準備「{next_step}」",
            causal.same_task_repetitions
        )
    }
}

/// 將一筆跨系統因果漣漪注入指定住民的因果大腦（SIR 傳播免疫、突顯度排序、性格錨點計畫鏈驅動）
pub fn inject_ripple_fact_into_npc(entity_id: &str, fact: WorldRippleFact, now_unix: i64) -> bool {
    let mut states = LIVE_NPC_CAUSAL_STATES.write().unwrap_or_else(|e| e.into_inner());
    let state = states.entry(entity_id.to_string()).or_insert_with(|| {
        let s = clean_settlement_name(&fact.subject_zone_or_settlement);
        NpcCausalState {
            cornerstone: Cornerstone::Pride,
            stress: 18,
            stress_break_active: false,
            hunger: 30,
            thirst: 25,
            fatigue: 20,
            plan_steps: ["先檢點隨身傢伙與行囊繩索".to_string(), "巡看周遭動靜".to_string(), "如常操持生計".to_string()],
            plan_cursor: 0,
            belief_scarcity_item: "常備物料".to_string(),
            last_surprise_delta: None,
            hooks: Vec::new(),
            life_step_tick: 100,
            same_task_repetitions: 10,
            carryover_from_yesterday: "昨日工序延續".to_string(),
            decision_image_code: "work_or_trade".to_string(),
            eligible_option_count: 3,
            chosen_action_code: "work_scavenge_wild_firewood_and_scraps".to_string(),
            active_need: "操持常業".to_string(),
            image_decision: "前往工坊".to_string(),
            situated_action: "操持常業".to_string(),
            belief_summary: format!("{s}周遭生計"),
            active_ripples: Vec::new(),
            immune_fact_ids: Vec::new(),
        }
    });

    state.immune_fact_ids.retain(|(_, exp)| *exp >= now_unix);
    if state.immune_fact_ids.iter().any(|(id, _)| id == &fact.fact_id) {
        return false;
    }
    if fact.hop_count > fact.max_hops {
        return false;
    }

    state.immune_fact_ids.push((fact.fact_id.clone(), now_unix + 5400));
    if state.immune_fact_ids.len() > 24 {
        state.immune_fact_ids.remove(0);
    }

    let eff_salience = |f: &WorldRippleFact| -> f32 { f.severity * (0.65_f32).powi(f.hop_count as i32) };
    state.active_ripples.retain(|f| {
        if f.fact_id == fact.fact_id {
            return false;
        }
        if fact.kind == RippleFactKind::ApexSlainOrRescued
            && f.kind == RippleFactKind::StrandedCargo
            && fact.order_id.is_some()
            && f.order_id == fact.order_id
        {
            return false;
        }
        true
    });
    state.active_ripples.push(fact.clone());
    state.active_ripples.sort_by(|a, b| {
        eff_salience(b)
            .partial_cmp(&eff_salience(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    state.active_ripples.truncate(4);

    state.belief_summary = fact.prose_summary.clone();
    state.last_surprise_delta = Some(fact.prose_summary.clone());

    match fact.kind {
        RippleFactKind::StrandedCargo | RippleFactKind::PredatorSpillover => {
            match state.cornerstone {
                Cornerstone::Honor | Cornerstone::Profit | Cornerstone::Pride => {
                    state.plan_steps = [
                        "先檢點隨身傢伙與行囊繩索".into(),
                        format!("趕往{}接應受阻貨單或助陣除獸", fact.subject_zone_or_settlement),
                        "護送物資直抵市集交割領賞".into(),
                    ];
                }
                Cornerstone::Order | Cornerstone::Care | Cornerstone::Craft => {
                    state.plan_steps = [
                        format!("聽聞{}風聲緊俏，先清點鋪內存貨", fact.subject_zone_or_settlement),
                        "避開險路改走穩妥熟道採辦".into(),
                        "安頓手頭器物與街坊所需".into(),
                    ];
                }
            }
            state.plan_cursor = 0;
        }
        RippleFactKind::MarketScarcity => {
            state.belief_scarcity_item = fact.item_or_actor.clone();
            state.plan_steps = [
                format!("得知{}急缺「{}」，先備齊手頭物料", fact.subject_zone_or_settlement, fact.item_or_actor),
                format!("趕製或調集「{}」運往市集", fact.item_or_actor),
                "趁行情看俏交割換取鎂幣".into(),
            ];
            state.plan_cursor = 0;
        }
        RippleFactKind::ApexSlainOrRescued => {
            state.plan_steps = [
                format!("欣聞{}險情解圍、貨途重新通暢", fact.subject_zone_or_settlement),
                "打點行囊重整商隊路引".into(),
                "如常往來交割各路物料".into(),
            ];
            state.plan_cursor = 0;
        }
        RippleFactKind::WeatherRouteHazard | RippleFactKind::ResourceFallow => {
            state.plan_steps = [
                format!("留心{}天候地勢與途程異動", fact.subject_zone_or_settlement),
                "放緩步調備妥防護用度".into(),
                "守住常業靜候路況平復".into(),
            ];
            state.plan_cursor = 0;
        }
        RippleFactKind::PlayerDeed => {
            state.plan_steps = [
                format!("得知義士在{}現身行事", fact.subject_zone_or_settlement),
                "留心坊間口碑與風聲流向".into(),
                "秉持本分相待往來豪客".into(),
            ];
            state.plan_cursor = 0;
        }
    }

    true
}

/// 兩位住民相遇時進行口耳相傳（SIR 傳播、衰減衰退、人情鉤子建立）
pub fn propagate_ripple_between_npcs(
    speaker_id: &str,
    speaker_name: &str,
    listener_id: &str,
    _listener_name: &str,
    now_unix: i64,
) -> Option<WorldRippleFact> {
    let eff_salience = |f: &WorldRippleFact| -> f32 { f.severity * (0.65_f32).powi(f.hop_count as i32) };
    let best_fact = {
        let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
        let speaker = states.get(speaker_id)?;
        let listener_opt = states.get(listener_id);
        speaker
            .active_ripples
            .iter()
            .filter(|f| {
                f.hop_count < f.max_hops
                    && (f.origin_tick + 5400 >= now_unix)
                    && listener_opt
                        .map(|l| !l.immune_fact_ids.iter().any(|(id, exp)| id == &f.fact_id && *exp >= now_unix))
                        .unwrap_or(true)
            })
            .max_by(|a, b| {
                eff_salience(a)
                    .partial_cmp(&eff_salience(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    };

    let fact = best_fact?;
    let mut next_fact = fact.clone();
    next_fact.hop_count += 1;
    next_fact.prose_summary = if fact.hop_count == 0 && !fact.prose_summary.contains(&speaker_name) {
        format!("聽{}提起，{}", speaker_name, fact.prose_summary)
    } else {
        fact.prose_summary.clone()
    };

    let injected = inject_ripple_fact_into_npc(listener_id, next_fact.clone(), now_unix);
    if injected {
        upsert_relational_hook(
            listener_id,
            speaker_id,
            speaker_name,
            "互通風聲",
            format!(
                "承蒙【{}】告知{}的實況風聲，得以提早因應",
                speaker_name, next_fact.subject_zone_or_settlement
            ),
            2,
            now_unix,
        );
        Some(next_fact)
    } else {
        None
    }
}

/// 查詢指定住民當前最突顯之因果漣漪
pub fn top_active_ripple_for_npc(entity_id: &str, now_unix: i64) -> Option<WorldRippleFact> {
    let eff_salience = |f: &WorldRippleFact| -> f32 { f.severity * (0.65_f32).powi(f.hop_count as i32) };
    let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
    let state = states.get(entity_id)?;
    state
        .active_ripples
        .iter()
        .filter(|f| f.origin_tick + 5400 >= now_unix)
        .max_by(|a, b| {
            eff_salience(a)
                .partial_cmp(&eff_salience(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}


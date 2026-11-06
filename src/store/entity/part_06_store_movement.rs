impl Store {
    pub fn step_npc_grid_coord_with_grid(
        &self,
        entity_id: &str,
        grid: &crate::grid::SquareGrid,
    ) -> Option<((i32, i32), (i32, i32))> {
        let e = self.entities.get(entity_id)?;
        let (old_x, old_y) = self.effective_entity_grid_with_grid(e, Some(grid))?;
        if is_npc_paused_for_interaction(entity_id) {
            return Some(((old_x, old_y), (old_x, old_y)));
        }

        let hour = current_game_hour();
        let slot = wenyao_household_slot_for(entity_id);

        let _is_expedition_slot = slot.brief_tag.starts_with("合社獵團・")
            || slot.brief_tag.starts_with("入山百工・")
            || slot.brief_tag.starts_with("古道行旅・");
        let in_hunting_party = LIVE_HUNTING_PARTIES
            .try_read()
            .map(|parties| {
                parties
                    .values()
                    .any(|p| p.member_ids.iter().any(|id| id == entity_id))
            })
            .unwrap_or(false);

        let mut arrived_for_ripple: Option<((i32, i32), String, i64)> = None;
        let mut states = LIVE_NPC_GRID_STATES.write();
        let state = states.entry(entity_id.to_string()).or_insert_with(|| {
            let (dest, dest_name, purpose, stay_activity, stay_ticks) =
                resolve_wenyao_npc_schedule(slot, hour, false, slot.home_name);
            NpcGridState {
                current: (old_x, old_y),
                dest,
                dest_name,
                origin_name: slot.home_name.to_string(),
                stay_activity,
                purpose,
                stay_ticks_remaining: stay_ticks,
                work_errand_toggle: false,
                consecutive_work_streaks: 0,
            }
        });

        if e.grid_x.is_some() && e.grid_y.is_some() {
            return None;
        }
        // 作息與日間／夜間返家均由下方正向因果引擎（plan_next_causal_action）統一驅動
        let game_hour = crate::game::current_game_hour();
        if !(6..=21).contains(&game_hour)
            && state.current != (slot.home_x, slot.home_y)
            && (state.purpose.contains("拾柴") || state.purpose.contains("野草") || state.dest_name.contains("西麓林道"))
        {
            state.dest = (slot.home_x, slot.home_y);
            state.dest_name = slot.home_name.to_string();
            state.purpose = "回居所木榻安歇養神".to_string();
            state.stay_ticks_remaining = 0;
        }
        if state.current == state.dest {
            if state.stay_ticks_remaining > 0 {
                state.stay_ticks_remaining -= 1;
                if state.stay_ticks_remaining == 3 {
                    let now_unix = crate::game::now_unix();
                    let spk_name = resolve_npc_person_name(entity_id, &e.display_title, Some(e.gender.as_str()));
                    arrived_for_ripple = Some((state.current, spk_name, now_unix));
                } else {
                    return Some(((old_x, old_y), (old_x, old_y)));
                }
            } else {
            let now_unix = crate::game::now_unix();
            let ent_id = entity_id.to_string();
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in entity_id.as_bytes() {
                h ^= u64::from(*b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
            // 1. 評估已完成的目標行為並寫入真實記憶心聲、經濟收益、器物磨合與傷病處置
            let dest_name_appraisal = state.dest_name.clone();
            let purpose_appraisal = state.purpose.clone();
            let is_expedition_role = slot.brief_tag.starts_with("合社獵團・")
                || slot.brief_tag.starts_with("入山百工・")
                || slot.brief_tag.starts_with("古道行旅・");
            let role_hint = format!("{} {} {}", slot.brief_tag, slot.work_name, crate::npc::causal_core::strip_trailing_person_name_from_role(&e.display_title));
            let vessel_spec = crate::npc::causal_core::resolve_vocational_vessel(&role_hint);
            let matched_opt = crate::npc::causal_core::find_predefined_action_by_title(&purpose_appraisal);
            let is_bath = matched_opt
                .map(|opt| opt.code == "rest_soak_in_hot_stone_bathhouse")
                .unwrap_or_else(|| {
                    dest_name_appraisal.contains("浴場")
                        || dest_name_appraisal.contains("湯屋")
                        || purpose_appraisal.contains("泡湯")
                        || purpose_appraisal.contains("沐浴")
                });
            let is_dining = !is_bath
                && matched_opt
                    .map(|opt| {
                        matches!(
                            opt.image,
                            crate::npc::causal_core::DecisionImageKind::Eat
                                | crate::npc::causal_core::DecisionImageKind::Drink
                        )
                    })
                    .unwrap_or_else(|| {
                        dest_name_appraisal.contains("食堂")
                            || dest_name_appraisal.contains("茶屋")
                            || dest_name_appraisal.contains("茶湯鋪")
                            || [
                                "乾糧", "果腹", "粗麵", "麥餅", "自炊", "熱粥", "滷肉", "大菜",
                                "吃熱食", "藥膳", "解渴", "冷開水", "涼水", "粗茶", "水筒", "涼茶",
                                "清茗", "共飲", "米酒", "品茶",
                            ]
                            .iter()
                            .any(|kw| purpose_appraisal.contains(kw))
                    });
            let is_clinic = matched_opt
                .map(|opt| opt.image == crate::npc::causal_core::DecisionImageKind::HealOrTend)
                .unwrap_or_else(|| {
                    dest_name_appraisal.contains("診所")
                        || dest_name_appraisal.contains("藥居")
                        || dest_name_appraisal.contains("藥鋪")
                        || [
                            "敷藥", "調理", "止血", "膏藥", "包紮", "煎湯", "熱敷", "施針",
                            "養傷", "靜養", "傷藥",
                        ]
                        .iter()
                        .any(|kw| purpose_appraisal.contains(kw))
                });
            let is_forge = matched_opt
                .map(|opt| opt.image == crate::npc::causal_core::DecisionImageKind::MendOrEquip)
                .unwrap_or_else(|| {
                    dest_name_appraisal.contains("鍛造")
                        || dest_name_appraisal.contains("鐵坊")
                        || dest_name_appraisal.contains("鐵器")
                        || purpose_appraisal.contains("修整器物")
                        || purpose_appraisal.contains("磨利")
                });
            let is_rest_stop = !is_bath
                && matched_opt
                    .map(|opt| opt.image == crate::npc::causal_core::DecisionImageKind::SleepOrRest)
                    .unwrap_or_else(|| {
                        [
                            "歇息", "小憩", "便當", "喘口氣", "閉目", "納涼", "用膳", "吃熱食",
                            "睡一覺", "熟睡", "調息", "擦汗歇腿",
                        ]
                        .iter()
                        .any(|kw| purpose_appraisal.contains(kw))
                    });
            let is_hunting_or_gathering = !is_clinic
                && !is_forge
                && !is_dining
                && !is_bath
                && !is_rest_stop
                && matched_opt
                    .map(|opt| {
                        opt.image == crate::npc::causal_core::DecisionImageKind::WorkOrTrade
                            && (opt.venue_req
                                == crate::npc::causal_core::VenueRequirement::WildOrOutskirts
                                || (is_expedition_role
                                    && state.current != (slot.errand_x, slot.errand_y)
                                    && state.current != (slot.home_x, slot.home_y)))
                    })
                    .unwrap_or_else(|| {
                        is_expedition_role
                            && state.current != (slot.errand_x, slot.errand_y)
                            && state.current != (slot.home_x, slot.home_y)
                    });
            let is_work = !is_rest_stop
                && !is_dining
                && !is_bath
                && !is_clinic
                && !is_forge
                && !is_hunting_or_gathering
                && matched_opt
                    .map(|opt| opt.image == crate::npc::causal_core::DecisionImageKind::WorkOrTrade)
                    .unwrap_or_else(|| {
                        [
                            "工坊", "鍛", "木", "商行", "埠", "站", "院", "堂", "館", "閣", "所",
                            "市集", "交易", "交割",
                        ]
                        .iter()
                        .any(|kw| dest_name_appraisal.contains(kw) || purpose_appraisal.contains(kw))
                    });
            if is_clinic {
                let _ = crate::pg::living_world::clear_entity_afflictions(&ent_id, true, now_unix);
                crate::npc::causal_core::enqueue_arrival_effect(
                    crate::npc::causal_core::ArrivalEffect::ClinicRecovered {
                        entity_id: ent_id.clone(),
                        dest_name: dest_name_appraisal.clone(),
                        recovered_label: "勞損與暗傷".to_string(),
                        now_unix,
                    },
                );
            } else if is_forge {
                let _ = crate::pg::living_world::temper_vessel_at_forge(
                    &ent_id,
                    vessel_spec.item_id,
                    now_unix,
                );
                crate::npc::causal_core::enqueue_arrival_effect(
                    crate::npc::causal_core::ArrivalEffect::VesselMaintained {
                        entity_id: ent_id.clone(),
                        dest_name: dest_name_appraisal.clone(),
                        vessel_name: vessel_spec.display_name.to_string(),
                        now_unix,
                    },
                );
            } else if is_hunting_or_gathering {
                state.consecutive_work_streaks = state.consecutive_work_streaks.saturating_add(1);
                let _ = crate::pg::living_world::record_breath_activity(
                    &ent_id,
                    crate::pg::living_world::BreathHabitKind::ForestLong,
                    2,
                    now_unix,
                );
                let _ = crate::pg::living_world::record_vessel_use(
                    &ent_id,
                    vessel_spec.item_id,
                    1,
                    1,
                    now_unix,
                );
                crate::npc::causal_core::enqueue_arrival_effect(
                    crate::npc::causal_core::ArrivalEffect::WildernessForaged {
                        entity_id: ent_id.clone(),
                        dest_name: dest_name_appraisal.clone(),
                        purpose: purpose_appraisal.clone(),
                        streak: state.consecutive_work_streaks,
                        now_unix,
                    },
                );
            } else if is_dining {
                crate::npc::causal_core::enqueue_arrival_effect(
                    crate::npc::causal_core::ArrivalEffect::DinedAtVenue {
                        entity_id: ent_id.clone(),
                        dest_name: dest_name_appraisal.clone(),
                        purpose: purpose_appraisal.clone(),
                        now_unix,
                    },
                );
                // consecutive_work_streaks 僅作物理連軸工數統計；生命是一條直線，大腦不可逆刻度與昨日延續由 advance_causal_state_on_arrival 完整繼承
                state.consecutive_work_streaks = 0;
            } else if is_bath {
                crate::npc::causal_core::enqueue_arrival_effect(
                    crate::npc::causal_core::ArrivalEffect::BathedAtVenue {
                        entity_id: ent_id.clone(),
                        now_unix,
                    },
                );
                // consecutive_work_streaks 僅作物理連軸工數統計；生命是一條直線，大腦不可逆刻度與昨日延續由 advance_causal_state_on_arrival 完整繼承
                state.consecutive_work_streaks = 0;
            } else if is_rest_stop {
                state.consecutive_work_streaks = 0;
                crate::npc::causal_core::enqueue_arrival_effect(
                    crate::npc::causal_core::ArrivalEffect::CompletedWorkShift {
                        entity_id: ent_id.clone(),
                        dest_name: dest_name_appraisal.clone(),
                        purpose: purpose_appraisal.clone(),
                        earn: 0,
                        streaks: 0,
                        now_unix,
                    },
                );
            } else if is_work {
                let streaks = state.consecutive_work_streaks;
                state.consecutive_work_streaks = state.consecutive_work_streaks.saturating_add(1);
                let _ = crate::pg::living_world::record_vessel_use(
                    &ent_id,
                    vessel_spec.item_id,
                    1,
                    usize::from(streaks % 3 == 2) as i32,
                    now_unix,
                );
                crate::npc::causal_core::enqueue_arrival_effect(
                    crate::npc::causal_core::ArrivalEffect::CompletedWorkShift {
                        entity_id: ent_id.clone(),
                        dest_name: dest_name_appraisal.clone(),
                        purpose: purpose_appraisal.clone(),
                        earn: 0,
                        streaks,
                        now_unix,
                    },
                );
            } else {
                // consecutive_work_streaks 僅作物理連軸工數統計；生命是一條直線，大腦不可逆刻度與昨日延續由 advance_causal_state_on_arrival 完整繼承
                state.consecutive_work_streaks = 0;
            }

            // Prune stale duplicate thoughts if needed
            let has_stale_broke = e.magnesium >= 35
                && e.thoughts
                    .iter()
                    .any(|t| t.flavor.as_deref().unwrap_or("").contains("鎂袋空了"));
            let daily_count = e
                .thoughts
                .iter()
                .filter(|t| t.flavor.as_deref().unwrap_or("").contains("今日花用又見底"))
                .count();
            if has_stale_broke || daily_count > 2 {
                crate::npc::causal_core::enqueue_arrival_effect(
                    crate::npc::causal_core::ArrivalEffect::PruneStaleThoughts {
                        entity_id: ent_id.clone(),
                        now_unix,
                    },
                );
            }
            let home_settlement_for_causal =
                normalize_household_home_settlement(slot.home_name);
            let prior_action_code = crate::npc::causal_core::LIVE_NPC_CAUSAL_STATES
                .read()
                .ok()
                .and_then(|m| m.get(&ent_id).map(|s| s.chosen_action_code.clone()))
                .unwrap_or_default();
            if !prior_action_code.is_empty()
                && !in_hunting_party
                && ![
                    "合社獵團集結地",
                    "合圍【",
                    "合圍收網處",
                    "外圍警戒圈",
                    "獸跡追蹤點",
                    "集結哨點",
                ]
                .iter()
                .any(|k| dest_name_appraisal.contains(k))
            {
                crate::npc::causal_core::enqueue_arrival_effect(
                    crate::npc::causal_core::ArrivalEffect::ActionSettlement {
                        entity_id: ent_id.clone(),
                        settlement: home_settlement_for_causal.to_string(),
                        dest_name: dest_name_appraisal.clone(),
                        action_code: prior_action_code,
                        now_unix,
                    },
                );
            }
            let prev_loc_name = state.dest_name.clone();
            let mut obligation_handled = false;

            let afflictions = crate::pg::living_world::get_entity_afflictions(entity_id);
            let vessel_state =
                crate::pg::living_world::get_vessel_state(entity_id, vessel_spec.item_id);
            let actual_scarce_name =
                crate::game::object_ecology::scarcest_item_for_settlement(home_settlement_for_causal)
                    .map(|(_id, nm, _cur, _max, _pr)| nm);
            let res_seed = crate::npc::causal_core::deterministic_resident_seed(entity_id, e.soul_seed);
            let role_combined = format!("{} {}", slot.brief_tag, e.display_title);
            let time_bucket = (now_unix / 20) as u64;

            // (A0) 若此人正參與「合社獵團／自主圍獵小隊（`LIVE_HUNTING_PARTIES`）」，直接由小隊共識板同步四面合圍戰術站位與階段行動！
            if let Some(parties) = LIVE_HUNTING_PARTIES.try_read() {
                for party in parties.values() {
                    if let Some(m_idx) = party.member_ids.iter().position(|id| id == entity_id) {
                        apply_party_directive_to_state(party, m_idx, grid, state);
                        obligation_handled = true;
                        break;
                    }
                }
            }

            let (causal_state, chosen_opt) = crate::npc::causal_core::plan_next_causal_action(
                entity_id,
                &role_combined,
                home_settlement_for_causal,
                res_seed,
                &dest_name_appraisal,
                state.consecutive_work_streaks,
                !afflictions.is_empty(),
                vessel_state.condition != crate::pg::living_world::VesselCondition::Sharp,
                e.magnesium,
                hour.max(0) as u32,
                actual_scarce_name.as_deref(),
            );
            if obligation_handled {
                if let Ok(mut map) = crate::npc::causal_core::LIVE_NPC_CAUSAL_STATES.write()
                    && let Some(st) = map.get_mut(&ent_id)
                {
                    st.chosen_action_code.clear();
                }
            }

            if !obligation_handled {
                let (next_coord, next_dest_name) = Self::resolve_venue_for_requirement(
                    chosen_opt.venue_req,
                    slot,
                    Some(grid),
                    state.current,
                    vessel_spec.is_bladed,
                    res_seed.wrapping_add(time_bucket),
                );
                state.origin_name = prev_loc_name;
                state.dest = next_coord;
                state.dest_name = next_dest_name;
                state.purpose = chosen_opt.action_title.to_string();
                state.stay_activity = crate::npc::causal_core::format_predefined_stay_activity(
                    &state.dest_name,
                    &causal_state,
                    e.magnesium,
                );
                let z_idx = crate::game::nearest_hunting_zone_idx(state.current.0, state.current.1);
                let spillover = crate::game::is_zone_predator_spillover_active(z_idx);
                let (weather, _tide) = crate::game::current_zone_weather_and_tide(z_idx, now_unix / 300);
                let is_urban_dest = crate::game::is_inside_settlement_urban_core(state.dest.0, state.dest.1)
                    || [
                        "文曜", "半嶺鎮", "風舶角", "南哨", "大圳頭", "紅土崁", "北嶺腳", "風櫃尾", "下營", "青竹坪",
                        "牛埔", "烏石鼻", "霜林口", "三號坑口", "水尾寮", "老堰頭", "三水口",
                        "石橋", "街", "巷", "弄", "坊", "鋪", "行", "堂", "館", "閣", "院", "所", "署", "庫", "倉",
                        "食堂", "茶", "酒", "旅社", "驛館", "診", "藥", "居", "宅", "舍", "齋", "井", "哨",
                        "屋", "寮", "房", "棚", "市集",
                    ]
                    .iter()
                    .any(|kw| state.dest_name.contains(kw));
                let is_urban_current = crate::game::is_inside_settlement_urban_core(state.current.0, state.current.1);
                let fallow = if is_urban_dest {
                    None
                } else {
                    crate::game::is_zone_in_fallow_stewardship(&state.dest_name)
                };
                let pers = self.entities.get(entity_id).and_then(|e| e.soul_seed).map(crate::db::expand_soul_seed_to_personality);
                let bold = pers.map(|p| p.boldness).unwrap_or(0.5);
                let person_name = resolve_npc_person_name(entity_id, &e.display_title, Some(e.gender.as_str()));
                let home_settlement = home_settlement_for_causal;
                let dest_settlement = state.dest_name.split('・').next().unwrap_or(&state.dest_name);

                if !is_urban_dest
                    && !is_urban_current
                    && slot.brief_tag.starts_with("古道行旅・")
                    && (spillover || weather == crate::game::ZoneWeatherPhase::RainMist || weather == crate::game::ZoneWeatherPhase::ColdFrost)
                    && bold < 0.52
                {
                    let hazard_prose = if spillover {
                        "山中猛獸越界下山於古道伏行窺伺"
                    } else if weather == crate::game::ZoneWeatherPhase::RainMist {
                        "連綿山雨致古道泥濘難行"
                    } else {
                        "寒霜凜冽封阻高處山徑"
                    };
                    crate::npc::causal_core::create_stranded_cargo_order(
                        entity_id,
                        &person_name,
                        "iron_ore",
                        "高純鐵礦石",
                        2,
                        home_settlement,
                        dest_settlement,
                        &state.dest_name,
                        state.current,
                        hazard_prose,
                        now_unix,
                    );
                    open_active_npc_obligation(
                        entity_id,
                        "stranded_cargo",
                        &format!("受阻於{}的急貨待援", state.dest_name),
                    );
                    state.stay_activity = format!(
                        "因前方{}而不敢貿然強穿，正守著滯留貨擔在{}焦急尋人接應",
                        hazard_prose, state.dest_name
                    );
                } else if bold >= 0.52 || slot.brief_tag.starts_with("合社獵團・") || slot.brief_tag.starts_with("古道行旅・") {
                    if let Some((order, bounty)) = crate::npc::causal_core::claim_or_deliver_stranded_cargo_for_npc(
                        entity_id,
                        &person_name,
                        &state.dest_name,
                        state.current,
                        now_unix,
                    ) {
                        if bounty > 0 {
                            let mut treasuries = crate::npc::causal_core::SETTLEMENT_TREASURIES
                                .write()
                                .unwrap_or_else(|e| e.into_inner());
                            let target_s = if !order.origin_settlement.is_empty() { order.origin_settlement.clone() } else { "文曜".to_string() };
                            *treasuries.entry(target_s).or_insert(2400) += bounty;
                            drop(treasuries);
                            crate::npc::causal_core::enqueue_arrival_effect(
                                crate::npc::causal_core::ArrivalEffect::CustomThoughtAndMg {
                                    entity_id: entity_id.to_string(),
                                    delta_mg: bounty,
                                    delta_disp: 4,
                                    thought: format!(
                                        "替【{}】將受阻於{}的「{}」護送過關，領得公庫賞格 {} 枚鎂幣",
                                        order.carrier_name, order.waystation_name, order.item_name, bounty
                                    ),
                                    now_unix,
                                },
                            );
                        }
                        clear_active_npc_obligation(&order.carrier_id, "stranded_cargo");
                        let _ = crate::game::object_ecology::supply_settlement_commodity_from_work(
                            &order.dest_settlement,
                            "arbitrage",
                            &person_name,
                        );
                        state.stay_activity = format!(
                            "冒險替【{}】將滯留的「{}」搶運過關、送抵{}市集交割",
                            order.carrier_name, order.item_name, order.dest_settlement
                        );
                    }
                }

                if let Some(res_label) = &fallow {
                    let dest_z_idx = crate::game::resolve_zone_idx_from_name_or_settlement(&state.dest_name);
                    let zone_name = crate::game::CANONICAL_HUNTING_ZONES
                        .get(dest_z_idx)
                        .map(|z| z.0)
                        .unwrap_or(state.dest_name.as_str());
                    let prose_summary = match *res_label {
                        "漁場" => format!(
                            "{zone_name}的漁場因連日捕撈正逢休漁養息，漁人多已轉往鄰灣或留守修補網具"
                        ),
                        "礦脈" => format!(
                            "{zone_name}的礦脈因連日開採正逢封坑休養，礦工多已轉往鄰谷或留守加固坑道"
                        ),
                        _ => format!(
                            "{zone_name}的{res_label}因連日採擷正逢封山休養，百工多已轉往鄰谷或留守培土護苗"
                        ),
                    };
                    let fact = crate::npc::causal_core::WorldRippleFact {
                        fact_id: format!("fallow_{}_{}_{}", zone_name, res_label, now_unix / 300),
                        kind: crate::npc::causal_core::RippleFactKind::ResourceFallow,
                        subject_zone_or_settlement: zone_name.to_string(),
                        item_or_actor: res_label.to_string(),
                        order_id: None,
                        severity: 1.0,
                        origin_tick: now_unix,
                        hop_count: 0,
                        max_hops: 4,
                        prose_summary,
                    };
                    crate::npc::causal_core::inject_ripple_fact_into_npc(entity_id, fact, now_unix);
                    if slot.brief_tag.starts_with("入山百工・") {
                        state.stay_activity = match *res_label {
                            "漁場" => format!("見此處{}亟待休養，正親手修整礁岸、養護藻床，兼做「{}」", res_label, chosen_opt.action_title),
                            "礦脈" => format!("見此處{}亟待休養，正親手加固坑木、疏通山溝，兼做「{}」", res_label, chosen_opt.action_title),
                            _ => format!("見此處{}亟待休養，正親手培土護苗、修整山場，兼做「{}」", res_label, chosen_opt.action_title),
                        };
                    }
                } else if slot.brief_tag.starts_with("古道行旅・") && spillover && bold >= 0.52 {
                    state.stay_activity = format!(
                        "沿途留神草莽間下山覓食的猛獸動靜，一邊緊握防身傢伙、一邊{}",
                        chosen_opt.action_title
                    );
                }

                state.stay_ticks_remaining = 6 + ((h % 5) as u8);
            }
            let spk_name = resolve_npc_person_name(entity_id, &e.display_title, Some(e.gender.as_str()));
            arrived_for_ripple = Some((state.current, spk_name, now_unix));
            }
        }

        let step_result = if state.current != state.dest {
            let next_coord = find_bfs_next_step(grid, state.current, state.dest)
                .unwrap_or(state.current);
            let prev_coord = state.current;
            state.current = next_coord;
            (prev_coord, next_coord)
        } else {
            ((old_x, old_y), (old_x, old_y))
        };
        if let Some((cur_pos, speaker_name, now_unix)) = arrived_for_ripple {
            let peer_opt = states
                .iter()
                .find(|(id, st)| {
                    id.as_str() != entity_id
                        && (st.current.0 - cur_pos.0).abs() + (st.current.1 - cur_pos.1).abs() <= 1
                })
                .map(|(id, _)| id.clone());
            drop(states);
            if let Some(peer_id) = peer_opt {
                let peer_name = self
                    .entities
                    .get(&peer_id)
                    .map(|pe| {
                        resolve_npc_person_name(
                            &pe.id,
                            if pe.display_title.trim().is_empty() {
                                "街坊住民"
                            } else {
                                pe.display_title.as_str()
                            },
                            Some(pe.gender.as_str()),
                        )
                    })
                    .unwrap_or_else(|| "坊間住民".to_string());
                crate::npc::causal_core::propagate_ripple_between_npcs(
                    entity_id,
                    &speaker_name,
                    &peer_id,
                    &peer_name,
                    now_unix,
                );
                crate::npc::causal_core::propagate_ripple_between_npcs(
                    &peer_id,
                    &peer_name,
                    entity_id,
                    &speaker_name,
                    now_unix,
                );
            }
        } else {
            drop(states);
        }
        Some(step_result)
    }

    /// 與指定正方格座標重合的實體 id（依 `entity_ids_in_room`）。
    pub fn entity_ids_at_grid(&self, x: i32, y: i32) -> Vec<String> {
        self.entity_ids_in_room(&crate::grid::grid_room_id_from_coord(x, y))
    }

    /// 有正方格座標的存活 NPC id 列表。
    pub fn get_npc_ids_with_grid(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.entities
            .values()
            .filter(|e| e.kind == "npc" && e.vit > 0 && e.grid_x.is_some() && e.grid_y.is_some())
            .map(|e| e.id.clone())
            .collect();
        let live = LIVE_NPC_GRID_STATES.read();
        for id in live.keys() {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids
    }

    pub fn transfer_magnesium(
        &mut self,
        from_id: &str,
        to_id: &str,
        amount: i32,
    ) -> anyhow::Result<()> {
        if amount <= 0 {
            anyhow::bail!("轉帳鎂數須為正");
        }
        let from_mg = self
            .entities
            .get(from_id)
            .ok_or_else(|| anyhow::anyhow!("找不到轉出實體"))?
            .magnesium;
        let _to = self
            .entities
            .get(to_id)
            .ok_or_else(|| anyhow::anyhow!("找不到轉入實體"))?;
        if from_mg < amount {
            anyhow::bail!("鎂不足");
        }
        self.entities.get_mut(from_id).unwrap().magnesium = from_mg - amount;
        let to_mg = self.entities.get(to_id).unwrap().magnesium;
        self.entities.get_mut(to_id).unwrap().magnesium = to_mg + amount;
        if let Some(from_e) = self.entities.get(from_id) {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::UpsertEntity(from_e.clone()));
        }
        if let Some(to_e) = self.entities.get(to_id) {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::UpsertEntity(to_e.clone()));
        }
        self.persist_entities()
    }

    pub fn get_entities_in_box(
        &self,
        x_min: i32,
        x_max: i32,
        y_min: i32,
        y_max: i32,
        kind: &str,
    ) -> Vec<Entity> {
        self.entities
            .values()
            .filter(|e| {
                (kind.is_empty() || e.kind == kind)
                    && e.x >= x_min
                    && e.x <= x_max
                    && e.y >= y_min
                    && e.y <= y_max
            })
            .cloned()
            .collect()
    }

    pub fn get_moving_entity_ids(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|(_, e)| e.move_state == "moving")
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn npc_ids_with_missing_soul_seed(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|(_, e)| e.kind == "npc" && e.soul_seed.is_none())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// 回傳缺少方格座標（`grid_x` 或 `grid_y` 為 None）的 NPC id 列表。
    pub fn npc_ids_missing_grid(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|(_, e)| e.kind == "npc" && (e.grid_x.is_none() || e.grid_y.is_none()))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn clear_all_entities(&mut self) -> anyhow::Result<()> {
        self.entities.clear();
        self.entity_rooms.clear();
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
        {
            let _ = conn.execute("DELETE FROM entities", &[]);
            let _ = conn.execute("DELETE FROM entity_rooms", &[]);
        }
        self.persist_entities()?;
        self.persist_entity_rooms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_salted_wenyao_names_roundtrip_and_gender_separation() {
        for salt in 0..3840 {
            let f_name = deterministic_wenyao_full_name_salted("npc_test_f", Some("F"), salt);
            assert_eq!(
                resolve_npc_person_name("npc_test_f", &f_name, Some("F")),
                f_name,
                "Female salted name failed roundtrip at salt={salt}: {f_name}"
            );
            let m_name = deterministic_wenyao_full_name_salted("npc_test_m", Some("M"), salt);
            assert_eq!(
                resolve_npc_person_name("npc_test_m", &m_name, Some("M")),
                m_name,
                "Male salted name failed roundtrip at salt={salt}: {m_name}"
            );
        }
        assert_ne!(resolve_npc_person_name("npc_100", "韓國", Some("M")), "韓國");
        assert_ne!(resolve_npc_person_name("npc_101", "宋淑", Some("F")), "宋淑");
        assert_ne!(resolve_npc_person_name("npc_102", "盧強勇", Some("M")), "盧強勇");
        assert_ne!(resolve_npc_person_name("npc_103", "萬玲淑", Some("F")), "萬玲淑");
        assert_eq!(resolve_npc_person_name("溫晴", "溫晴", Some("F")), "溫晴");
    }
}

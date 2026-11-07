/// 住民真人生命迴圈：全體白塔住民共用性格權重四問決策與器物磨損動力學（零特例分支）
pub fn step_resident_human_life_loop(
    store: &mut crate::store::Store,
    entity_id: &str,
    sched_x: i32,
    sched_y: i32,
    sched_bldg: &str,
    sched_act: &str,
    game_hour: i32,
    now_unix: i64,
) -> (i32, i32, String) {
    let Some(mut ent) = store.get_entity(entity_id) else {
        return (sched_x, sched_y, sched_act.to_string());
    };

    let mut factors = ResidentIntrinsicFactors::resolve(&ent, sched_x, sched_y, sched_bldg, sched_act);

    // 若實體在記憶體 Store 尚未掛載基準衣裝或行囊，依其內在職務因子自動補齊，不綁定單一姓名
    if ent.equipment_slots.trim().is_empty() || ent.equipment_slots == "{}" {
        let hold_l_entry = factors
            .secondary_tool_id
            .as_deref()
            .unwrap_or("");
        let slots_json = serde_json::json!({
            "head": "wt_cat_0875",
            "body": "wt_cat_0869",
            "legs": "starter_legs_m",
            "feet": "starter_feet_m",
            "wrist": "wt_cat_0852",
            "waist": "wt_cat_0897",
            "hold_r": factors.tool_id,
            "hold_l": hold_l_entry,
        });
        ent.equipment_slots = slots_json.to_string();
    }
    if ent.activated_nodes.is_empty() || ent.activated_nodes == "[]" || ent.activated_nodes == "[\"N000\"]" {
        ent.activated_nodes = r#"["N000","N012","N028","N045","N077","N112"]"#.to_string();
    }
    if ent.inventory.trim().is_empty() || ent.inventory == "[]" {
        let mut seed_items = Vec::new();
        for (out_id, _, _, _) in &factors.outputs {
            seed_items.push(serde_json::json!({"item_id": out_id, "qty": 6}));
        }
        seed_items.push(serde_json::json!({"item_id": &factors.raw_item_id, "qty": 10}));
        seed_items.push(serde_json::json!({"item_id": &factors.tool_id, "qty": 1}));
        if let Some(sec_id) = &factors.secondary_tool_id {
            seed_items.push(serde_json::json!({"item_id": sec_id, "qty": 1}));
        }
        ent.inventory = serde_json::Value::Array(seed_items).to_string();
    }

    let grain_qty = inv_count_item(&ent.inventory, &factors.raw_item_id);
    let primary_out_id = factors
        .outputs
        .first()
        .map(|(id, _, _, _)| id.as_str())
        .unwrap_or("wt_cat_0388");
    let ricemilk_qty = inv_count_item(&ent.inventory, primary_out_id);
    let vstate = get_vessel_state(entity_id, &factors.tool_id);
    let wear_points = if factors.is_bladed { vstate.wear_points } else { 0 };
    let repetitions = vstate.use_count.max(0) as u32;

    let hooks = crate::npc::causal_core::get_relational_hooks(entity_id);
    let (weather, tide) = crate::game::zone_weather_and_tide(&factors.settlement, now_unix);
    let seed_u64 = crate::combat::human_phase::resolve_entity_soul_seed(entity_id, ent.soul_seed) as u64;
    let stress = ((100 - ent.vit.clamp(0, 100)) as u8).saturating_div(2);
    let pw = crate::npc::causal_core::compute_plural_personality_weights_with_experience(
        &ent.display_title,
        &factors.settlement,
        seed_u64,
        stress,
        ent.magnesium,
        repetitions,
        &hooks,
        None,
    );

    // ── 需求決策：自主依房間標籤、距離代價、制服／內場辨識與恩怨尋商（上游供貨） ──
    let cur_xy = ent.grid_x.zip(ent.grid_y).unwrap_or(factors.work_coord);
    let runtime_grid = crate::server::grid_manager::get_runtime_square_grid();

    if factors.supplier_id.is_empty()
        || !store.entities.contains_key(&factors.supplier_id)
        || inv_count_item(
            &store.entities.get(&factors.supplier_id).map(|e| e.inventory.clone()).unwrap_or_default(),
            &factors.raw_item_id,
        ) <= 0
        || hooks.iter().any(|h| (h.target_id == factors.supplier_id || h.target_name == factors.supplier_id) && is_grudge_hook(h) && h.intensity >= 15 && pw.pride >= 95)
    {
        if let Some((chosen_sup, sup_place, sup_coord)) = resolve_need_decision_supplier(
            store,
            entity_id,
            cur_xy,
            &factors,
            &pw,
            &hooks,
            runtime_grid.as_ref(),
        ) {
            factors.supplier_id = chosen_sup;
            factors.supplier_place_label = sup_place;
            factors.supplier_coord = Some(sup_coord);
        } else {
            factors.supplier_id = format!("{}櫃檯", factors.supplier_place_label);
            factors.supplier_coord = Some(factors.work_coord);
        }
    }

    let mut obligation_deliver_coord: Option<(i32, i32)> = None;
    let mut contract_open = crate::store::get_active_npc_obligation(entity_id).is_some();
    if let Some((_kind, target_hint)) = crate::store::get_active_npc_obligation(entity_id) {
        let cp = target_hint.split('・').next().unwrap_or(target_hint.as_str());
        if !cp.is_empty() {
            factors.counterparty_id = cp.to_string();
        }
        if let Some(cp_ent) = store.entities.get(cp)
            && let (Some(x), Some(y)) = (cp_ent.grid_x, cp_ent.grid_y)
        {
            obligation_deliver_coord = Some((x, y));
        }
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        // 僅當買方實際庫存見底（< 4）且距上次交割已逾 1 小時，才因真實供需缺口重啟補貨契約（廢除死板 5 點發條重設）
        if (5..=8).contains(&game_hour) && !factors.counterparty_id.is_empty() {
            let buyer_stock = store
                .entities
                .get(&factors.counterparty_id)
                .map(|cp| inv_count_item(&cp.inventory, primary_out_id))
                .unwrap_or(0);
            if buyer_stock < 4 {
                let due = now_unix + 10_800;
                let stale_cutoff = now_unix - 3_600;
                let _ = client.execute(
                    "UPDATE resident_obligations SET status = 'active', due_at = $2, updated_at = $3
                     WHERE entity_id = $1 AND obligation_kind = 'restock_goods'
                       AND status = 'fulfilled' AND updated_at < $4",
                    &[&entity_id, &due, &now_unix, &stale_cutoff],
                );
            }
        }
        if let Ok(rows) = client.query(
            "SELECT subject_ref FROM resident_obligations WHERE entity_id = $1 AND obligation_kind = 'restock_goods' AND status IN ('open', 'due', 'active') LIMIT 1",
            &[&entity_id],
        ) {
            if !rows.is_empty() {
                contract_open = true;
                let subj: String = rows[0].get(0);
                let cp = subj.split('・').next().unwrap_or(subj.as_str());
                if !cp.is_empty() {
                    factors.counterparty_id = cp.to_string();
                }
                if let Some(cp_ent) = store.entities.get(cp)
                    && let (Some(x), Some(y)) = (cp_ent.grid_x, cp_ent.grid_y)
                {
                    obligation_deliver_coord = Some((x, y));
                }
            }
        }
    }

    // ── 需求決策：自主依房間標籤、距離代價、制服／內場辨識與預算尋客（下游交割） ──
    if (5..=8).contains(&game_hour)
        && (!contract_open
            || factors.counterparty_id.is_empty()
            || !store.entities.contains_key(&factors.counterparty_id))
    {
        if let Some((buyer_id, _buyer_place, buyer_coord)) = resolve_need_decision_buyer(
            store,
            entity_id,
            cur_xy,
            primary_out_id,
            &factors,
            &pw,
            &hooks,
            runtime_grid.as_ref(),
        ) {
            factors.counterparty_id = buyer_id;
            obligation_deliver_coord = Some(buyer_coord);
            contract_open = true;
        } else if factors.counterparty_id.is_empty() {
            factors.counterparty_id = format!("{}櫃檯", factors.deliver_bldg);
            obligation_deliver_coord = Some(factors.deliver_coord);
        }
    }

    if let Some(sup_ent) = store.entities.get(&factors.supplier_id)
        && let (Some(sx), Some(sy)) = (sup_ent.grid_x, sup_ent.grid_y)
    {
        factors.supplier_coord = Some((sx, sy));
    }
    if obligation_deliver_coord.is_none() {
        if let Some(cp_ent) = store.entities.get(&factors.counterparty_id)
            && let (Some(cx), Some(cy)) = (cp_ent.grid_x, cp_ent.grid_y)
        {
            obligation_deliver_coord = Some((cx, cy));
        } else {
            obligation_deliver_coord = Some(factors.deliver_coord);
        }
    }

    let chosen = choose_resident_life_action(
        &ent.display_title,
        &factors.settlement,
        seed_u64,
        ent.vit,
        ent.magnesium,
        grain_qty,
        ricemilk_qty,
        wear_points,
        contract_open,
        game_hour,
        weather,
        tide,
        repetitions,
        &hooks,
    );

    let (target_x, target_y) = match chosen {
        ResidentLifeActionChoice::RepairTool => factors.work_coord,
        ResidentLifeActionChoice::DeliverContract => obligation_deliver_coord.unwrap_or(factors.deliver_coord),
        ResidentLifeActionChoice::RestockGrain => factors.supplier_coord.unwrap_or(factors.work_coord),
        ResidentLifeActionChoice::FloodgateCheck => factors.floodgate_coord,
        ResidentLifeActionChoice::DineParley => factors.canteen_coord,
        ResidentLifeActionChoice::NightRest => factors.home_coord,
        ResidentLifeActionChoice::MillGrain => factors.work_coord,
    };

    let cur_pos = ent.grid_x.zip(ent.grid_y);
    let is_far = cur_pos
        .map(|(cx, cy)| (cx - target_x).abs() + (cy - target_y).abs() > 1)
        .unwrap_or(false);

    let dest_place = format!("grid:{target_x}:{target_y}");
    let work_bldg_plain = factors.work_bldg.replace('・', "");
    let deliver_bldg_plain = factors.deliver_bldg.replace('・', "");
    let floodgate_bldg_plain = factors.floodgate_bldg.replace('・', "");
    let canteen_bldg_plain = factors.canteen_bldg.replace('・', "");
    let home_bldg_plain = factors.home_bldg.replace('・', "");

    // ── 抵達閘門（Arrival Gate）：若尚未抵達場所，沿途推進且嚴禁隔空完成現場交割或工坊產出 ──
    if is_far {
        let (cx, cy) = cur_pos.unwrap_or((target_x, target_y));
        let (next_x, next_y) = crate::server::grid_manager::get_runtime_square_grid()
            .and_then(|g| crate::store::find_bfs_next_step(&g, (cx, cy), (target_x, target_y)))
            .unwrap_or((cx + (target_x - cx).signum(), cy + (target_y - cy).signum()));

        ent.grid_x = Some(next_x);
        ent.grid_y = Some(next_y);
        ent.target_x = if (next_x, next_y) == (target_x, target_y) { None } else { Some(target_x) };
        ent.target_y = if (next_x, next_y) == (target_x, target_y) { None } else { Some(target_y) };

        let dest_building = match chosen {
            ResidentLifeActionChoice::RepairTool => &factors.work_bldg,
            ResidentLifeActionChoice::DeliverContract => &factors.deliver_bldg,
            ResidentLifeActionChoice::RestockGrain => if factors.supplier_coord.is_some() {
                &factors.supplier_place_label
            } else {
                &factors.work_bldg
            },
            ResidentLifeActionChoice::FloodgateCheck => &factors.floodgate_bldg,
            ResidentLifeActionChoice::DineParley => &factors.canteen_bldg,
            ResidentLifeActionChoice::NightRest => &factors.home_bldg,
            ResidentLifeActionChoice::MillGrain => &factors.work_bldg,
        };

        let delivery_names: Vec<&str> = factors
            .delivery_items
            .iter()
            .map(|(_, name, _, _)| name.as_str())
            .collect();
        let delivery_label = if delivery_names.is_empty() {
            "工坊成品".to_string()
        } else {
            delivery_names.join("與")
        };

        let resolve_id_to_person = |raw_id: &str, fallback_role: &str| -> String {
            let trimmed = raw_id.trim();
            if trimmed.is_empty() {
                return String::new();
            }
            let hint = store
                .get_entity(trimmed)
                .map(|e| e.display_title)
                .unwrap_or_default();
            let n = crate::store::resolve_npc_person_name(trimmed, &hint, None);
            if n.is_empty() || n == "陌生人" || n.bytes().any(|b| b.is_ascii_alphabetic()) {
                fallback_role.to_string()
            } else {
                n
            }
        };
        let resolved_cp_person = resolve_id_to_person(&factors.counterparty_id, "買方掌事");
        let resolved_sup_person = resolve_id_to_person(&factors.supplier_id, "供貨掌事");

        let (in_transit_act, transit_thought) = match chosen {
            ResidentLifeActionChoice::DeliverContract => {
                let cp_call = if resolved_cp_person.is_empty() {
                    format!("{}櫃檯", deliver_bldg_plain)
                } else {
                    resolved_cp_person
                };
                (
                    format!(
                        "背負{}與{}，正沿山道前往{}交割早市合約",
                        factors.delivery_vessel_desc, delivery_label, deliver_bldg_plain
                    ),
                    format!(
                        "趁早將{}送達{}處，免得耽誤早市交割",
                        delivery_label, cp_call
                    ),
                )
            }
            ResidentLifeActionChoice::RestockGrain => {
                let sup_call = if resolved_sup_person.is_empty() {
                    format!("{}櫃檯", factors.supplier_place_label)
                } else if resolved_sup_person.starts_with(&factors.supplier_place_label) {
                    resolved_sup_person.clone()
                } else {
                    format!("{}{}", factors.supplier_place_label, resolved_sup_person)
                };
                let sup_target = if resolved_sup_person.is_empty() {
                    format!("{}櫃檯", factors.supplier_place_label)
                } else {
                    resolved_sup_person
                };
                (
                    format!("步向{}處商議補進{}", sup_call, factors.raw_item_name),
                    format!(
                        "工坊{}吃緊，得盡快找{}結算拉料入庫",
                        factors.raw_item_name, sup_target
                    ),
                )
            }
            ResidentLifeActionChoice::FloodgateCheck => (
                format!("沿{}步往{}巡看水勢", floodgate_bldg_plain, factors.floodgate_facility),
                format!("雨霧水位漸漲，得趕在{}受沖前落板減流", floodgate_bldg_plain),
            ),
            ResidentLifeActionChoice::DineParley => (
                format!("收工後沿渠畔步往{}用膳，與街坊商議明日引水與工務時刻", canteen_bldg_plain),
                format!("勞作大半日腹中空乏，先去{}喝碗熱湯歇腿，順道問問街坊近況", canteen_bldg_plain),
            ),
            ResidentLifeActionChoice::NightRest => (
                format!("巡看罷工坊門板，沿石徑緩步走回{}歇宿", home_bldg_plain),
                format!("今日工坊運轉平順，回{}安歇，養足精神明日早起", home_bldg_plain),
            ),
            ResidentLifeActionChoice::RepairTool => (
                format!("取下磨耗之{}，步往{}校準刃口", factors.tool_name, work_bldg_plain),
                format!("{}刃口起毛，活計吃不準分寸，得先去{}修整", factors.tool_name, factors.maintenance_station),
            ),
            ResidentLifeActionChoice::MillGrain => (
                format!("正沿山道返回{}料理手頭差事", work_bldg_plain),
                format!("返回{}開工料理{}，今日這批活計得做得細膩", work_bldg_plain, factors.raw_item_name),
            ),
        };

        ent.current_activity = in_transit_act.clone();
        let intent = "in_transit";
        let reason = format!("正沿路步往{}（{}, {}）", dest_building, target_x, target_y);
        crate::db::add_thought_to_entity_with_delta(&mut ent, crate::db::ThoughtSource::WorkDone, 4, now_unix, transit_thought);
        crate::db::clean_and_recalc_entity_disposition(&mut ent, now_unix);

        let ent_sync = ent.clone();
        let _ = store.upsert_entity_full(ent);
        let current_place = format!("grid:{next_x}:{next_y}");
        let _ = store.move_entity(entity_id, &current_place);
        crate::pg::entity::upsert_async(ent_sync);

        record_resident_continuity_row(entity_id, intent, &reason, &dest_place, now_unix);

        return (target_x, target_y, in_transit_act);
    }

    // ── 現場履行（On-Site Execution）：已抵達現場，全體角色共用單一實物、器物、體能、吐納與鎂幣守恆路徑 ──
    let reason: String;
    let activity: String;
    let thought_src: crate::db::ThoughtSource;
    let thought_text: String;
    let mut knowledge_update: Option<(String, String, String)> = None;
    let intent: &str;

    match chosen {
        ResidentLifeActionChoice::RepairTool => {
            intent = "repair_tool";
            let honed = match sharpen_vessel_at_grindstone(entity_id, &factors.tool_id, now_unix) {
                Ok(rec) => rec,
                Err(_) => temper_vessel_at_forge(entity_id, &factors.tool_id, now_unix),
            };
            let habit = get_or_derive_breath_habit(entity_id, &ent.display_title, seed_u64 as i64).dominant;
            let _ = record_breath_activity(entity_id, habit, 2, now_unix);
            crate::store::clear_active_npc_obligation(entity_id, "maintain_tool");

            reason = format!(
                "於{}前以水礪法細細修整{}刃口，磨損歸零，刃口狀態自鈍刃重回「{}」（已運使 {} 次）",
                factors.maintenance_station,
                factors.tool_name,
                honed.condition.label(),
                honed.use_count,
            );
            activity = format!(
                "在{}前澆水細礪{}，用手指輕抹刃口檢視鋒芒分寸",
                factors.maintenance_station, factors.tool_name
            );
            thought_src = crate::db::ThoughtSource::WorkDone;
            thought_text = format!(
                "{}經{}修礪後重現鋒芒，磨損重歸於零（{}），隨時可再投產",
                factors.tool_name,
                factors.maintenance_station,
                honed.condition.label()
            );
            knowledge_update = Some((
                factors.work_bldg.clone(),
                "chisel_maintenance_state".to_string(),
                format!(
                    "{}已於石台精礪完畢（狀態：{}，累計運使 {} 次），隨時可投產",
                    factors.tool_name,
                    honed.condition.label(),
                    honed.use_count
                ),
            ));
            // 甲的行為 -> 乙的事件：若甲在早市交割時段因重視器物鋒芒而選擇「先磨刀、延後送貨」，買方乙立即產生等待與應急事件
            if contract_open
                && (5..=8).contains(&game_hour)
                && let Some(mut cp) = store.get_entity(&factors.counterparty_id)
            {
                let wait_item = factors
                    .delivery_items
                    .first()
                    .map(|(_, n, _, _)| n.as_str())
                    .unwrap_or("早市物料");
                let self_person = crate::store::resolve_npc_person_name(entity_id, &ent.display_title, None);
                let cp_thought = format!(
                    "早市時分尚未見{}送來{}，想是工坊修整器物耽擱了片刻，先拿店裡存貨應急",
                    self_person, wait_item
                );
                crate::db::add_thought_to_entity_with_delta(
                    &mut cp,
                    crate::db::ThoughtSource::WorkDone,
                    2,
                    now_unix,
                    cp_thought,
                );
                let cp_sync = cp.clone();
                let _ = store.upsert_entity_full(cp);
                crate::pg::entity::upsert_async(cp_sync);
            }
        }
        ResidentLifeActionChoice::DeliverContract => {
            intent = "deliver_contract";
            let mut delivered_parts = Vec::new();
            let mut remaining_parts = Vec::new();
            for (item_id, item_name, unit, qty) in &factors.delivery_items {
                let rem = inv_mutate_item(&mut ent.inventory, item_id, -(*qty), 12);
                delivered_parts.push(format!("{item_name} {qty} {unit}"));
                remaining_parts.push(format!("{item_name} {rem} {unit}"));
            }
            let delivered_str = delivered_parts.join("與");
            let remaining_str = remaining_parts.join("、");
            let mut payment_mg = 0;
            if let Some(mut cp) = store.get_entity(&factors.counterparty_id) {
                payment_mg = cp.magnesium.clamp(0, factors.delivery_payout_mg);
                cp.magnesium -= payment_mg;
                for (item_id, _, _, qty) in &factors.delivery_items {
                    inv_mutate_item(&mut cp.inventory, item_id, *qty, 12);
                }
                // 甲的行為 -> 乙的事件：買方乙收到實物交割後，立即寫入乙的思緒與雙向信任鉤子
                let self_person = crate::store::resolve_npc_person_name(entity_id, &ent.display_title, None);
                let cp_event = format!(
                    "收到{}送來的{}，當面付訖 {} 枚鎂幣，今日店裡備料寬裕不少",
                    self_person, delivered_str, payment_mg
                );
                crate::db::add_thought_to_entity_with_delta(
                    &mut cp,
                    crate::db::ThoughtSource::Trade,
                    6,
                    now_unix,
                    cp_event,
                );
                crate::npc::causal_core::upsert_relational_hook(
                    &factors.counterparty_id,
                    entity_id,
                    &self_person,
                    "守諾交割",
                    format!("{}準時送達{}", self_person, delivered_str),
                    12,
                    now_unix,
                );
                let cp_sync = cp.clone();
                let _ = store.upsert_entity_full(cp);
                crate::pg::entity::upsert_async(cp_sync);
            }
            ent.magnesium += payment_mg;
            ent.vit = (ent.vit - 5).max(20);
            crate::store::clear_active_npc_obligation(entity_id, "restock_goods");
            if let Some(pool) = crate::pg::pool()
                && let Ok(mut client) = pool.get()
            {
                let _ = client.execute(
                    "UPDATE resident_obligations SET status = 'fulfilled', updated_at = $2
                     WHERE entity_id = $1 AND obligation_kind = 'restock_goods'",
                    &[&entity_id, &now_unix],
                );
            }

            let cp_person = if factors.counterparty_id.is_empty() {
                format!("{}櫃檯", deliver_bldg_plain)
            } else {
                let hint = store
                    .get_entity(&factors.counterparty_id)
                    .map(|e| e.display_title)
                    .unwrap_or_default();
                let n = crate::store::resolve_npc_person_name(&factors.counterparty_id, &hint, None);
                if n.is_empty() || n == "陌生人" || n.bytes().any(|b| b.is_ascii_alphabetic()) {
                    format!("{}櫃檯", deliver_bldg_plain)
                } else {
                    n
                }
            };
            reason = format!(
                "依早市承諾背負{}赴{}（{}, {}），向{}實物交割{}，實收 {} 枚鎂幣",
                factors.delivery_vessel_desc,
                deliver_bldg_plain,
                factors.deliver_coord.0,
                factors.deliver_coord.1,
                cp_person,
                delivered_str,
                payment_mg
            );
            activity = format!(
                "在{}將{}內的{}點交予{}，當面結清早市貨款",
                deliver_bldg_plain,
                factors.delivery_vessel_desc,
                delivered_str,
                cp_person
            );
            thought_src = crate::db::ThoughtSource::Trade;
            thought_text = format!(
                "趁熱將{}送到{}交給{}，入帳 {} 枚鎂幣，手頭尚餘{}",
                delivered_str,
                deliver_bldg_plain,
                cp_person,
                payment_mg,
                remaining_str
            );
            let cp_settlement = store
                .get_entity(&factors.counterparty_id)
                .map(|cp| crate::world::materials::infer_origin_settlement(&format!("grid:{}:{}", cp.x, cp.y)))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    if !factors.settlement.is_empty() {
                        factors.settlement.as_str()
                    } else {
                        crate::world::materials::infer_origin_settlement(&format!("grid:{}:{}", ent.x, ent.y))
                    }
                });
            let contract_tag = factors
                .delivery_items
                .first()
                .map(|(item_id, _, _, _)| {
                    let clean = item_id
                        .trim_start_matches("wt_cat_")
                        .trim_start_matches("wt-good-")
                        .trim_start_matches("mat_");
                    if clean.is_empty() {
                        "settlement_supply_contract".to_string()
                    } else {
                        format!("{clean}_supply_contract")
                    }
                })
                .unwrap_or_else(|| "settlement_supply_contract".to_string());
            knowledge_update = Some((
                format!("{cp_settlement}・{cp_person}"),
                contract_tag,
                format!(
                    "今日已親赴{}向{}交割{}（實收 {} 枚鎂幣，隨身尚存{}）",
                    deliver_bldg_plain,
                    cp_person,
                    delivered_str,
                    payment_mg,
                    remaining_str
                ),
            ));
        }
        ResidentLifeActionChoice::RestockGrain => {
            intent = "restock_grain";
            let cost_mg = ent.magnesium.min(factors.restock_cost_mg).max(0);
            ent.magnesium -= cost_mg;
            let buy_qty = factors.restock_qty;
            if let Some(mut sup) = store.get_entity(&factors.supplier_id) {
                let sup_stock = inv_count_item(&sup.inventory, &factors.raw_item_id);
                if sup_stock > 0 {
                    inv_mutate_item(&mut sup.inventory, &factors.raw_item_id, -buy_qty, 12);
                }
                sup.magnesium += cost_mg;
                // 甲的行為 -> 乙的事件：上游供應商賣出原料並收受鎂幣後，更新其思緒與雙向關係鉤子
                let self_person = crate::store::resolve_npc_person_name(entity_id, &ent.display_title, None);
                let sup_event = format!(
                    "將 {} {}{}點交予{}，收訖 {} 枚鎂幣",
                    buy_qty, factors.raw_item_unit, factors.raw_item_name, self_person, cost_mg
                );
                crate::db::add_thought_to_entity_with_delta(
                    &mut sup,
                    crate::db::ThoughtSource::Trade,
                    5,
                    now_unix,
                    sup_event,
                );
                crate::npc::causal_core::upsert_relational_hook(
                    &factors.supplier_id,
                    entity_id,
                    &self_person,
                    "守諾供料",
                    format!("向{}供料{} {} {}", self_person, buy_qty, factors.raw_item_unit, factors.raw_item_name),
                    10,
                    now_unix,
                );
                let sup_sync = sup.clone();
                let _ = store.upsert_entity_full(sup);
                crate::pg::entity::upsert_async(sup_sync);
            }
            let new_stock = inv_mutate_item(&mut ent.inventory, &factors.raw_item_id, buy_qty, 12);

            let resolved_sup_name = if factors.supplier_id.is_empty() {
                String::new()
            } else {
                let hint = store
                    .get_entity(&factors.supplier_id)
                    .map(|e| e.display_title)
                    .unwrap_or_default();
                let n = crate::store::resolve_npc_person_name(
                    &factors.supplier_id,
                    &hint,
                    None,
                );
                if n.is_empty() || n == "陌生人" || n.bytes().any(|b| b.is_ascii_alphabetic()) {
                    "供貨掌事".to_string()
                } else {
                    n
                }
            };
            let sup_desc = if resolved_sup_name.is_empty() {
                format!("{}櫃檯", factors.supplier_place_label)
            } else if resolved_sup_name.starts_with(&factors.supplier_place_label) {
                resolved_sup_name.clone()
            } else {
                format!("{}{}", factors.supplier_place_label, resolved_sup_name)
            };
            let sup_person = if resolved_sup_name.is_empty() {
                format!("{}櫃檯", factors.supplier_place_label)
            } else {
                resolved_sup_name
            };

            reason = format!(
                "工坊{}存量見底，向{}付訖 {} 枚鎂幣提領 {} {}{}入倉（現存 {} {}）",
                factors.raw_item_name,
                sup_desc,
                cost_mg,
                buy_qty,
                factors.raw_item_unit,
                factors.raw_item_name,
                new_stock,
                factors.raw_item_unit
            );
            activity = format!(
                "在{}畔向{}點收新運抵的 {} {}{}，當面結清鎂幣並扛入{}",
                work_bldg_plain,
                sup_desc,
                buy_qty,
                factors.raw_item_unit,
                factors.raw_item_name,
                work_bldg_plain
            );
            thought_src = crate::db::ThoughtSource::Trade;
            thought_text = format!(
                "向{}付了 {} 枚鎂幣補進 {} {}{}（現存 {} {}），{}今日不愁斷料",
                sup_desc,
                cost_mg,
                buy_qty,
                factors.raw_item_unit,
                factors.raw_item_name,
                new_stock,
                factors.raw_item_unit,
                work_bldg_plain
            );
            knowledge_update = Some((
                if factors.supplier_id.is_empty() {
                    format!("{}・櫃檯", factors.supplier_place_label)
                } else if factors.supplier_id.starts_with(&factors.supplier_place_label) {
                    factors.supplier_id.clone()
                } else {
                    format!("{}・{}", factors.supplier_place_label, factors.supplier_id)
                },
                "upstream_grain_supply".to_string(),
                format!(
                    "因工坊{}存量見底，已向{}支付 {} 枚鎂幣補入 {} {}{}，目前隨身存量 {} {}",
                    factors.raw_item_name,
                    sup_person,
                    cost_mg,
                    buy_qty,
                    factors.raw_item_unit,
                    factors.raw_item_name,
                    new_stock,
                    factors.raw_item_unit
                ),
            ));
        }
        ResidentLifeActionChoice::FloodgateCheck => {
            intent = "floodgate_check";
            ent.vit = (ent.vit - 3).max(20);
            ent.qi = (ent.qi - 2).max(20);
            let habit = get_or_derive_breath_habit(entity_id, &ent.display_title, seed_u64 as i64).dominant;
            let _ = record_breath_activity(entity_id, habit, 2, now_unix);

            reason = format!(
                "觀測到北坡雨霧伴隨水脈漲潮，依水文經驗趕赴{}（{}, {}）落下{}",
                floodgate_bldg_plain,
                factors.floodgate_coord.0,
                factors.floodgate_coord.1,
                factors.floodgate_facility
            );
            activity = format!(
                "冒雨在{}落下{}",
                floodgate_bldg_plain, factors.floodgate_facility
            );
            thought_src = crate::db::ThoughtSource::WorkDone;
            thought_text = format!(
                "北坡雨霧水位漲起三寸，好在及時趕赴{}落下{}，保住了工坊器物與產出細度",
                floodgate_bldg_plain, factors.floodgate_facility
            );
            knowledge_update = Some((
                factors.floodgate_bldg.clone(),
                "hydrology_and_weather_omen".to_string(),
                format!(
                    "北坡雨霧漲水之際已親赴{}落下{}（當前體力 {}、內息 {}），水勢恢復平穩",
                    floodgate_bldg_plain, factors.floodgate_facility, ent.vit, ent.qi
                ),
            ));
        }
        ResidentLifeActionChoice::DineParley => {
            intent = "dine_and_parley";
            if ent.magnesium >= 2 {
                crate::npc::causal_core::deposit_to_settlement_treasury(&factors.settlement, 2);
                ent.magnesium -= 2;
            } else if ent.magnesium > 0 {
                let paid = ent.magnesium;
                crate::npc::causal_core::deposit_to_settlement_treasury(&factors.settlement, paid);
                ent.magnesium = 0;
            }
            ent.vit = (ent.vit + 18).min(100);
            ent.qi = (ent.qi + 15).min(100);

            reason = format!(
                "勞作後赴{}（{}, {}）用膳回補體力（體力恢復至 {}），並與鄰里核對明日引水與工坊時刻",
                canteen_bldg_plain, factors.canteen_coord.0, factors.canteen_coord.1, ent.vit
            );
            activity = format!(
                "在{}喝著現磨米漿與熱麵湯，與北郊鄰里商議明日引水與工務時刻",
                canteen_bldg_plain
            );
            thought_src = crate::db::ThoughtSource::Ate;
            thought_text = format!(
                "在{}喝過熱米漿與麵湯，一身勞作的疲乏舒展開來，順道與鄰里敲定了明日工務時刻",
                canteen_bldg_plain
            );
        }
        ResidentLifeActionChoice::NightRest => {
            intent = "night_rest";
            ent.vit = (ent.vit + 25).min(100);
            ent.qi = (ent.qi + 25).min(100);
            let habit = get_or_derive_breath_habit(entity_id, &ent.display_title, seed_u64 as i64).dominant;
            let _ = record_breath_activity(entity_id, habit, 1, now_unix);

            reason = format!(
                "返抵{}（{}, {}）巡看門板插板後安歇，體力回補至 {}、內息回補至 {}",
                home_bldg_plain, factors.home_coord.0, factors.home_coord.1, ent.vit, ent.qi
            );
            activity = format!(
                "在{}檢查水門與門栓插板，伴著規律水聲歇息養神",
                home_bldg_plain
            );
            thought_src = crate::db::ThoughtSource::Rested;
            thought_text = format!(
                "檢查完{}門栓插板，聽著窗外規律水聲安穩養神",
                home_bldg_plain
            );
        }
        ResidentLifeActionChoice::MillGrain => {
            intent = "work";
            let rem_raw = inv_mutate_item(
                &mut ent.inventory,
                &factors.raw_item_id,
                -factors.raw_consume_qty,
                12,
            );
            let mut produced_parts = Vec::new();
            for (out_id, out_name, out_unit, out_qty) in &factors.outputs {
                let new_qty = inv_mutate_item(&mut ent.inventory, out_id, *out_qty, 12);
                produced_parts.push(format!("{out_name}（存 {new_qty} {out_unit}）"));
                crate::npc::causal_core::record_settlement_market_production(
                    &factors.settlement,
                    out_id,
                    (*out_qty).max(1),
                );
            }
            ent.vit = (ent.vit - 4).max(20);
            ent.qi = (ent.qi - 3).max(20);
            let v_primary = record_vessel_use(
                entity_id,
                &factors.tool_id,
                if factors.is_bladed { 1 } else { 0 },
                1,
                now_unix,
            );
            let v_secondary_use = if let Some(sec_id) = &factors.secondary_tool_id {
                Some(record_vessel_use(entity_id, sec_id, 0, 1, now_unix).use_count)
            } else {
                None
            };
            let habit = get_or_derive_breath_habit(entity_id, &ent.display_title, seed_u64 as i64).dominant;
            let _ = record_breath_activity(entity_id, habit, 2, now_unix);
            {
                let mut guard = ANATOMY_FAMILIARITY_CACHE.write().unwrap();
                let map = guard.entry(entity_id.to_string()).or_default();
                *map.entry(factors.anatomy_familiarity_subject.clone()).or_insert(36) += 1;
            }

            let produced_str = produced_parts.join("與");
            reason = format!(
                "於{}（{}, {}）消耗 {} {}{}（餘 {} {}）產出{}，{}磨損 {}/3（{}）",
                work_bldg_plain,
                factors.work_coord.0,
                factors.work_coord.1,
                factors.raw_consume_qty,
                factors.raw_item_unit,
                factors.raw_item_name,
                rem_raw,
                factors.raw_item_unit,
                produced_str,
                factors.tool_name,
                v_primary.wear_points,
                v_primary.condition.label(),
            );
            activity = factors.work_activity_desc.clone();
            thought_src = crate::db::ThoughtSource::WorkDone;
            thought_text = format!(
                "在{}將 {} {}{}化作{}，手中{}已運使 {} 次",
                work_bldg_plain,
                factors.raw_consume_qty,
                factors.raw_item_unit,
                factors.raw_item_name,
                produced_str,
                factors.tool_name,
                v_primary.use_count
            );
            let sec_note = match (&factors.secondary_tool_name, v_secondary_use) {
                (Some(sname), Some(scount)) => format!("{}已運使 {} 次，", sname, scount),
                _ => String::new(),
            };
            knowledge_update = Some((
                factors.work_bldg.clone(),
                "millstone_and_sluice_state".to_string(),
                format!(
                    "今日已耗{}產出{}（待用{}餘 {} {}）；{}{}已運使 {} 次（磨損點數 {}・{}）",
                    factors.raw_item_name,
                    produced_str,
                    factors.raw_item_name,
                    rem_raw,
                    factors.raw_item_unit,
                    sec_note,
                    factors.tool_name,
                    v_primary.use_count,
                    v_primary.wear_points,
                    v_primary.condition.label(),
                ),
            ));
        }
    }

    let (next_x, next_y) = (target_x, target_y);
    ent.grid_x = Some(next_x);
    ent.grid_y = Some(next_y);
    ent.target_x = None;
    ent.target_y = None;
    ent.current_activity = activity.clone();
    ent.thoughts.retain(|t| {
        let f = t.flavor.as_deref().unwrap_or("");
        !f.contains("鎂袋空了")
            && !f.contains("今日花用又見底")
            && !f.contains("記掛著「內室榻上熄燈安歇」")
            && !f.contains("記掛著『診所抓藥』")
            && !f.contains("記掛著『手頭差事』")
    });
    crate::db::add_thought_to_entity_with_delta(&mut ent, thought_src, 8, now_unix, thought_text);
    crate::db::clean_and_recalc_entity_disposition(&mut ent, now_unix);

    let ent_sync = ent.clone();
    let _ = store.upsert_entity_full(ent);
    let current_place = format!("grid:{next_x}:{next_y}");
    let _ = store.move_entity(entity_id, &current_place);
    crate::pg::entity::upsert_async(ent_sync);

    record_resident_continuity_row(entity_id, intent, &reason, &dest_place, now_unix);
    if let Some((subj, pred, val)) = knowledge_update {
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute(
                "INSERT INTO resident_knowledge (entity_id, subject_ref, predicate, value, certainty, source_ref, learned_at, updated_at)
                 VALUES ($1, $2, $3, $4, 98, 'four_question_loop', $5, $5)
                 ON CONFLICT (entity_id, subject_ref, predicate) DO UPDATE SET
                   value = EXCLUDED.value,
                   source_ref = EXCLUDED.source_ref,
                   updated_at = EXCLUDED.updated_at",
                &[&entity_id, &subj, &pred, &val, &now_unix],
            );
        }
    }

    (target_x, target_y, activity)
}


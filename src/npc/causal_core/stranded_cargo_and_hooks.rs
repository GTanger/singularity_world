/// 建立滯留物資貨單並自目的聚落公庫鎖定託管懸賞（SFC 嚴格守恆）
pub fn create_stranded_cargo_order(
    carrier_id: &str,
    carrier_name: &str,
    item_id: &str,
    item_name: &str,
    qty: i32,
    origin_settlement: &str,
    dest_settlement: &str,
    waystation_name: &str,
    waystation_coord: (i32, i32),
    hazard_prose: &str,
    now_unix: i64,
) -> Option<StrandedCargoOrder> {
    let clean_dest = clean_settlement_name(dest_settlement);
    let escrow_bounty_mg = {
        let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
        let bal = treasuries.entry(clean_dest.to_string()).or_insert(2400);
        if *bal >= 220 {
            *bal -= 8;
            8
        } else {
            0
        }
    };

    static ORDER_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let count = ORDER_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let order_id = format!("cargo_{}_{}_{}", carrier_id, now_unix, count);
    let final_hazard = if hazard_prose.trim().is_empty() {
        format!("山道遭遇異獸盤踞，山嵐濃霧阻道難行，行商【{carrier_name}】受困於{waystation_name}")
    } else {
        hazard_prose.trim().to_string()
    };

    let order = StrandedCargoOrder {
        order_id: order_id.clone(),
        carrier_id: carrier_id.to_string(),
        carrier_name: carrier_name.to_string(),
        item_id: item_id.to_string(),
        item_name: item_name.to_string(),
        qty,
        origin_settlement: origin_settlement.to_string(),
        dest_settlement: dest_settlement.to_string(),
        waystation_name: waystation_name.to_string(),
        waystation_coord,
        escrow_bounty_mg,
        hazard_prose: final_hazard,
        claimed_by: None,
        created_at_unix: now_unix,
        expires_at_unix: now_unix + 3600,
    };

    {
        let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
        let mut expired_refunds: Vec<(String, i32)> = Vec::new();
        for o in orders.values_mut() {
            if o.expires_at_unix < now_unix && o.escrow_bounty_mg > 0 {
                expired_refunds.push((o.dest_settlement.clone(), o.escrow_bounty_mg));
                o.escrow_bounty_mg = 0;
            }
        }
        if !expired_refunds.is_empty() {
            let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
            for (dest, refund) in expired_refunds {
                let s = clean_settlement_name(&dest);
                *treasuries.entry(s.to_string()).or_insert(2400) += refund;
            }
        }
        orders.retain(|_, o| o.expires_at_unix >= now_unix);
        if orders.len() < 32 {
            orders.insert(order_id.clone(), order.clone());
        } else {
            if escrow_bounty_mg > 0 {
                let mut treasuries = SETTLEMENT_TREASURIES.write().unwrap_or_else(|e| e.into_inner());
                *treasuries.entry(clean_dest.to_string()).or_insert(2400) += escrow_bounty_mg;
            }
            return None;
        }
    }

    let fact = WorldRippleFact {
        fact_id: format!("rip_{}", order.order_id),
        kind: RippleFactKind::StrandedCargo,
        subject_zone_or_settlement: waystation_name.to_string(),
        item_or_actor: item_name.to_string(),
        order_id: Some(order.order_id.clone()),
        severity: 0.9,
        origin_tick: now_unix,
        hop_count: 0,
        max_hops: 3,
        prose_summary: format!(
            "行商【{}】運送「{}」行經{}受阻，貨單受滯急待接應",
            carrier_name, item_name, waystation_name
        ),
    };
    inject_ripple_fact_into_npc(carrier_id, fact, now_unix);

    Some(order)
}

/// 住民抵達或途經驛站／終點時，認領或交割受阻貨單並領取託管懸賞（SFC 守恆）
pub fn claim_or_deliver_stranded_cargo_for_npc(
    rescuer_id: &str,
    rescuer_name: &str,
    current_settlement_or_room: &str,
    current_coord: (i32, i32),
    now_unix: i64,
) -> Option<(StrandedCargoOrder, i32)> {
    let mut target_order_id = None;
    {
        let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
        for (id, o) in orders.iter() {
            if o.escrow_bounty_mg > 0 && o.carrier_id != rescuer_id && now_unix <= o.expires_at_unix {
                if o.claimed_by.as_deref().is_some_and(|c| c != rescuer_id) {
                    continue;
                }
                let near_waystation = (current_coord.0 - o.waystation_coord.0).abs()
                    + (current_coord.1 - o.waystation_coord.1).abs()
                    <= 6;
                let near_room = !current_settlement_or_room.is_empty()
                    && (current_settlement_or_room.contains(&o.waystation_name)
                        || current_settlement_or_room.contains(&o.dest_settlement)
                        || current_settlement_or_room.contains(&o.origin_settlement));
                if near_waystation || near_room {
                    target_order_id = Some(id.clone());
                    break;
                }
            }
        }
    }

    let order_id = target_order_id?;
    let mut order = {
        let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
        orders.remove(&order_id)?
    };

    let bounty = order.escrow_bounty_mg;
    order.escrow_bounty_mg = 0;

    upsert_relational_hook(
        &order.carrier_id,
        rescuer_id,
        rescuer_name,
        "山道解圍",
        format!(
            "承蒙義士【{}】在{}涉險解圍、順利起獲受阻貨物",
            rescuer_name, order.waystation_name
        ),
        4,
        now_unix,
    );

    let fact = WorldRippleFact {
        fact_id: format!("rescued_{}", order.order_id),
        kind: RippleFactKind::ApexSlainOrRescued,
        subject_zone_or_settlement: order.waystation_name.clone(),
        item_or_actor: order.item_name.clone(),
        order_id: Some(order.order_id.clone()),
        severity: 0.95,
        origin_tick: now_unix,
        hop_count: 0,
        max_hops: 3,
        prose_summary: format!(
            "滯留於{}的「{}」已蒙【{}】解圍護送，商道重新暢通",
            order.waystation_name, order.item_name, rescuer_name
        ),
    };
    inject_ripple_fact_into_npc(&order.carrier_id, fact.clone(), now_unix);
    inject_ripple_fact_into_npc(rescuer_id, fact, now_unix);

    Some((order, bounty))
}

/// 玩家承接指定滯留物資貨單（鎖定該貨單之領取人，避免被其他遊蕩者或住民誤領）
pub fn claim_stranded_cargo_for_player(
    player_id: &str,
    _player_name: &str,
    order_id_or_target: &str,
    settlement_or_room: &str,
    now_unix: i64,
) -> Option<StrandedCargoOrder> {
    let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
    let mut target_id = None;

    if !order_id_or_target.is_empty() {
        for (id, o) in orders.iter() {
            if o.escrow_bounty_mg > 0
                && now_unix <= o.expires_at_unix
                && o.claimed_by.is_none()
            {
                if o.order_id == order_id_or_target
                    || o.carrier_id == order_id_or_target
                    || o.carrier_name.contains(order_id_or_target)
                    || o.item_id == order_id_or_target
                    || o.item_name.contains(order_id_or_target)
                    || o.dest_settlement == order_id_or_target
                    || o.waystation_name.contains(order_id_or_target)
                {
                    target_id = Some(id.clone());
                    break;
                }
            }
        }
    }

    if target_id.is_none() && !settlement_or_room.is_empty() {
        for (id, o) in orders.iter() {
            if o.escrow_bounty_mg > 0
                && now_unix <= o.expires_at_unix
                && o.claimed_by.is_none()
            {
                if settlement_or_room.contains(&o.dest_settlement)
                    || settlement_or_room.contains(&o.waystation_name)
                    || settlement_or_room.contains(&o.origin_settlement)
                    || o.dest_settlement.contains(settlement_or_room)
                    || o.waystation_name.contains(settlement_or_room)
                {
                    target_id = Some(id.clone());
                    break;
                }
            }
        }
    }

    if target_id.is_none() {
        for (id, o) in orders.iter() {
            if o.escrow_bounty_mg > 0
                && now_unix <= o.expires_at_unix
                && o.claimed_by.is_none()
            {
                target_id = Some(id.clone());
                break;
            }
        }
    }

    let id = target_id?;
    if let Some(o) = orders.get_mut(&id) {
        o.claimed_by = Some(player_id.to_string());
        Some(o.clone())
    } else {
        None
    }
}

/// 玩家在特定場所交割滯留貨單並領取公庫託管懸賞（SFC 守恆）
pub fn deliver_stranded_cargo_for_player(
    player_id: &str,
    player_name: &str,
    carrier_or_target_id: &str,
    settlement_or_room: &str,
    now_unix: i64,
) -> Option<(StrandedCargoOrder, i32)> {
    let mut target_order_id = None;
    {
        let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
        for (id, o) in orders.iter() {
            if o.escrow_bounty_mg > 0
                && now_unix <= o.expires_at_unix
                && o.claimed_by.as_deref() == Some(player_id)
            {
                let matches_target = carrier_or_target_id.is_empty()
                    || o.carrier_id == carrier_or_target_id
                    || o.dest_settlement == carrier_or_target_id
                    || o.order_id == carrier_or_target_id;
                let matches_place = settlement_or_room.is_empty()
                    || settlement_or_room.contains(&o.dest_settlement)
                    || (!carrier_or_target_id.is_empty()
                        && (settlement_or_room.contains(&o.waystation_name)
                            || settlement_or_room.contains(&o.origin_settlement)));
                if matches_target && matches_place {
                    target_order_id = Some(id.clone());
                    break;
                }
            }
        }
        if target_order_id.is_none() && !carrier_or_target_id.is_empty() {
            for (id, o) in orders.iter() {
                if o.escrow_bounty_mg > 0
                    && now_unix <= o.expires_at_unix
                    && (o.claimed_by.is_none() || o.claimed_by.as_deref() == Some(player_id))
                {
                    let matches_target = carrier_or_target_id.is_empty()
                        || o.carrier_id == carrier_or_target_id
                        || o.dest_settlement == carrier_or_target_id
                        || o.order_id == carrier_or_target_id;
                    let matches_place = settlement_or_room.is_empty()
                        || settlement_or_room.contains(&o.dest_settlement)
                        || settlement_or_room.contains(&o.waystation_name)
                        || settlement_or_room.contains(&o.origin_settlement);
                    if matches_target && matches_place {
                        target_order_id = Some(id.clone());
                        break;
                    }
                }
            }
        }
    }

    let order_id = target_order_id?;
    let mut order = {
        let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
        orders.remove(&order_id)?
    };

    let bounty = order.escrow_bounty_mg;
    order.escrow_bounty_mg = 0;

    if let Some(arc) = crate::store::get_store() {
        credit_magnesium_bounty(&mut arc.write(), player_id, bounty);
    }
    crate::store::clear_active_npc_obligation(&order.carrier_id, "stranded_cargo");
    crate::game::object_ecology::supply_settlement_commodity_from_work(
        &order.dest_settlement,
        "arbitrage",
        player_name,
    );

    upsert_relational_hook(
        &order.carrier_id,
        player_id,
        player_name,
        "山道解圍",
        format!(
            "承蒙義士【{}】在{}涉險解圍、順利送達受阻貨物",
            player_name, order.waystation_name
        ),
        4,
        now_unix,
    );

    let fact = WorldRippleFact {
        fact_id: format!("rescued_{}", order.order_id),
        kind: RippleFactKind::ApexSlainOrRescued,
        subject_zone_or_settlement: order.dest_settlement.clone(),
        item_or_actor: order.item_name.clone(),
        order_id: Some(order.order_id.clone()),
        severity: 0.95,
        origin_tick: now_unix,
        hop_count: 0,
        max_hops: 3,
        prose_summary: format!(
            "滯留於{}的「{}」已蒙義士【{}】護送至{}交割，各方額手稱慶",
            order.waystation_name, order.item_name, player_name, order.dest_settlement
        ),
    };
    inject_ripple_fact_into_npc(&order.carrier_id, fact.clone(), now_unix);
    inject_ripple_fact_into_npc(player_id, fact, now_unix);

    Some((order, bounty))
}

/// 查詢指定行商或場所關聯之進行中滯留貨單
pub fn active_stranded_cargo_for_carrier_or_room(
    entity_id: &str,
    room_name: &str,
    now_unix: i64,
) -> Option<StrandedCargoOrder> {
    let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
    for o in orders.values() {
        if o.escrow_bounty_mg > 0 && now_unix <= o.expires_at_unix {
            if (!entity_id.is_empty() && o.carrier_id == entity_id)
                || (!room_name.is_empty()
                    && (room_name.contains(&o.waystation_name)
                        || room_name.contains(&o.dest_settlement)
                        || room_name.contains(&o.origin_settlement)))
            {
                return Some(o.clone());
            }
        }
    }
    None
}

/// 列出全島即時在線之未結案滯留物資貨單清單
pub fn list_active_stranded_cargo_orders(now_unix: i64) -> Vec<StrandedCargoOrder> {
    let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
    orders
        .values()
        .filter(|o| o.escrow_bounty_mg > 0 && now_unix <= o.expires_at_unix)
        .cloned()
        .collect()
}

/// 動態更新或建立住民人情／恩怨鉤子（Relational Hook）
pub fn upsert_relational_hook(
    entity_id: &str,
    target_id: &str,
    target_name: &str,
    kind_label: &str,
    summary: String,
    intensity_delta: i8,
    now_unix: i64,
) {
    let mut states = LIVE_NPC_CAUSAL_STATES.write().unwrap_or_else(|e| e.into_inner());
    let state = states.entry(entity_id.to_string()).or_insert_with(|| {
        NpcCausalState {
            cornerstone: Cornerstone::Honor,
            stress: 15,
            stress_break_active: false,
            hunger: 30,
            thirst: 25,
            fatigue: 20,
            plan_steps: ["打點隨身器物".to_string(), "巡看周遭動靜".to_string(), "如常操持生計".to_string()],
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
            belief_summary: "常業平順".to_string(),
            active_ripples: Vec::new(),
            immune_fact_ids: Vec::new(),
        }
    });

    let mut found = false;
    for h in &mut state.hooks {
        let matches_id = !target_id.is_empty() && h.target_id == target_id;
        let matches_name = !target_name.is_empty() && h.target_name == target_name;
        if matches_id || matches_name {
            h.kind_label = kind_label.to_string();
            h.target_name = target_name.to_string();
            if !target_id.is_empty() {
                h.target_id = target_id.to_string();
            }
            h.summary = summary.clone();
            let new_int = (h.intensity as i16) + (intensity_delta as i16);
            h.intensity = new_int.clamp(1, 100) as u8;
            h.updated_at_unix = now_unix;
            found = true;
            break;
        }
    }

    if !found {
        let init_int = (5 + (intensity_delta as i16)).clamp(1, 100) as u8;
        state.hooks.push(RelationalHook {
            kind_label: kind_label.to_string(),
            target_name: target_name.to_string(),
            summary,
            intensity: init_int,
            target_id: target_id.to_string(),
            updated_at_unix: now_unix,
        });
        if state.hooks.len() > 8 {
            state.hooks.sort_by(|a, b| b.intensity.cmp(&a.intensity));
            state.hooks.truncate(8);
        }
    }
}

/// 查詢指定人物目前的關係鉤子列表
pub fn get_relational_hooks(entity_id: &str) -> Vec<RelationalHook> {
    let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
    states.get(entity_id).map(|s| s.hooks.clone()).unwrap_or_default()
}

/// 查詢指定住民對目標人物之既有關係鉤子
pub fn find_relational_hook_for_target(
    entity_id: &str,
    target_id: &str,
    target_name: &str,
) -> Option<RelationalHook> {
    let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
    let state = states.get(entity_id)?;
    state
        .hooks
        .iter()
        .find(|h| {
            (!target_id.is_empty() && h.target_id == target_id)
                || (!target_name.is_empty() && h.target_name == target_name)
        })
        .cloned()
}


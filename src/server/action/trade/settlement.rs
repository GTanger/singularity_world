fn handle_trade_settlement(
    conn: &WsConnection,
    pid: &str,
    target_id: &str,
    target: &Character,
    now: i64,
    trade_target_name: String,
    player_trade_input: &str,
    pending: crate::db::TradePending,
) -> bool {
    if player_trade_input.is_empty() {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "請輸入你願付的鎂數（整數），或輸入「拒絕」取消交易。".into(),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    }
    let Ok(offer) = player_trade_input.parse::<i32>() else {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "請輸入有效的鎂數（非負整數），或「拒絕」。".into(),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    };
    if offer < 0 {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "請輸入有效的鎂數（非負整數），或「拒絕」。".into(),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    }
    let Ok(Some(buyer)) = db::get_entity(pid) else {
        conn.send_error(gametext::client("get_self_failed"));
        return true;
    };
    let paid = if offer >= pending.ask_mg {
        pending.ask_mg
    } else if offer >= pending.floor_mg {
        offer
    } else {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: format!(
                "【{trade_target_name}】搖頭：至少要 {} 鎂才肯點頭。",
                pending.floor_mg
            ),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    };
    let has_lineage_or_trust = !crate::pg::living_world::get_entity_lineage_imprints(pid).is_empty()
        || crate::pg::pool()
            .and_then(|p| crate::pg::living_world::get_relationship_summary(&p, target_id, pid).ok())
            .map(|r| r.balance_score >= 2 || r.familiarity_score >= 3)
            .unwrap_or(false);
    let mut paid = paid;
    let fellowship_note = if has_lineage_or_trust && paid > 3 {
        paid = (paid - (paid / 10).max(1)).max(2);
        "（對方見你身具行家師承且素有交情，特意算了熟客同道價）"
    } else {
        ""
    };
    if buyer.magnesium < paid {
        let seller_id = &pending.npc_id;
        let (_floor, trust_score, _note) = compute_relationship_trade_terms(pid, seller_id, paid);
        let credit_outcome = crate::npc::debt_interaction::try_credit_purchase(
            pid,
            seller_id,
            &pending.item_id,
            paid,
            buyer.magnesium,
            trust_score,
            now / 60,
        );
        match credit_outcome {
            crate::npc::debt_interaction::CreditOutcome::PurchasedOnCredit {
                cash_paid,
                debt_created,
                ..
            }
            | crate::npc::debt_interaction::CreditOutcome::DebtCreated {
                upfront_paid: cash_paid,
                deferred_amount: debt_created,
                ..
            } => {
                if cash_paid > 0 {
                    let _ = db::add_magnesium(pid, -cash_paid);
                    let _ = db::add_magnesium(seller_id, cash_paid);
                }
                if pending.item_id.starts_with("wt_cat_") {
                    let _ = consume_shop_item_stock(seller_id, &pending.item_id, pending.item_qty);
                    let _ = db::add_to_inventory(pid, &pending.item_id, pending.item_qty);
                } else {
                    let _ = db::remove_from_inventory(seller_id, &pending.item_id, pending.item_qty);
                    let _ = db::add_to_inventory(pid, &pending.item_id, pending.item_qty);
                }
                let base_price = crate::world::materials::trade_price_anchor_mg(&pending.item_id)
                    .or_else(|| {
                        crate::game::peninsula_catalog::find_catalog_entry(&pending.item_id)
                            .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
                    })
                    .unwrap_or(paid)
                    .max(2);
                let player_room = super::resolve_action_room_name(pid, target);
                let _ = crate::game::object_ecology::record_settlement_market_trade(
                    &player_room,
                    &pending.item_id,
                    base_price,
                    -pending.item_qty,
                    "賒帳記契採買",
                );
                if let Some(pool) = crate::pg::pool() {
                    let _ = crate::pg::living_world::open_obligation(
                        &pool,
                        pid,
                        "repay_debt",
                        &format!("{}:{}mg", seller_id, debt_created),
                        None,
                        "credit_trade",
                        now,
                    );
                    let _ = crate::pg::living_world::record_relationship_delta(
                        &pool,
                        seller_id,
                        pid,
                        "traded",
                        1,
                        "player_trade_credit",
                        now,
                    );
                }
                let item_name = get_item_def(&pending.item_id)
                    .map(|d| d.name)
                    .filter(|n| !n.is_empty())
                    .or_else(|| {
                        crate::game::peninsula_catalog::find_catalog_entry(&pending.item_id)
                            .map(|c| c.name.to_string())
                    })
                    .or_else(|| crate::db::fallback_item_name(&pending.item_id).map(|s| s.to_string()))
                    .unwrap_or_else(|| pending.item_id.clone());
                let ep_note = advance_room_event_pack_on_trade(&player_room, &pending.item_id, &item_name);
                let ep_suffix = if let Some(ep_line) = ep_note {
                    format!("\n{ep_line}")
                } else {
                    String::new()
                };
                trade_offer_clear(pid, target_id);
                push_refresh(conn, pid);
                let _ = event::append(now, pid, types::TRADE, target_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!(
                        "你手頭現鎂僅有 {cash_paid} 鎂，【{trade_target_name}】念在彼此交情，允你先付現鎂、將差額 {debt_created} 鎂立契記帳賒欠，把【{item_name}】交予你手中。{ep_suffix}"
                    ),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return false;
            }
            _ => {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: "你的鎂不足，且交情尚淺未能立契賒欠，無法成交。".into(),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return false;
            }
        }
    }
    let settlement_result = if let Some(market_id) = pending.market_id {
        let Some(pool) = crate::pg::pool() else {
            trade_offer_clear(pid, target_id);
            conn.send_error("市場暫時無法結算，請稍後再試。");
            return true;
        };
        crate::pg::world_trade::settle_market_sale(
            &pool,
            pid,
            target_id,
            market_id,
            &pending.item_id,
            pending.item_qty,
            paid,
            now,
            "player_market_trade",
            None,
        )
        .map(|settlement| {
            mirror_market_trade_settlement(&settlement);
        })
    } else {
        let Ok(Some(seller)) = db::get_entity(target_id) else {
            trade_offer_clear(pid, target_id);
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Trade".into(),
                target_id: target.id.clone(),
                target_name: trade_target_name.clone(),
                narrative: format!("【{trade_target_name}】已不在可交易狀態，這筆交易作廢。"),
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return false;
        };
        if !inventory_has_qty(&seller.inventory, &pending.item_id, pending.item_qty) {
            if pending.item_id.starts_with("wt_cat_") {
                match consume_shop_item_stock(target_id, &pending.item_id, pending.item_qty) {
                    Ok(rem) => {
                        let item_name = get_item_def(&pending.item_id)
                            .map(|d| d.name)
                            .filter(|n| !n.is_empty())
                            .or_else(|| {
                                crate::game::peninsula_catalog::find_catalog_entry(&pending.item_id)
                                    .map(|c| c.name.to_string())
                            })
                            .unwrap_or_else(|| pending.item_id.clone());
                        if rem <= 1 {
                            crate::store::open_active_npc_obligation(
                                target_id,
                                "restock_goods",
                                &format!("店內【{}】現貨見底待補", item_name),
                            );
                        }
                        db::transfer_magnesium(pid, target_id, paid)
                            .or_else(|_| {
                                db::add_magnesium(pid, -paid)?;
                                let _ = db::add_magnesium(target_id, paid);
                                Ok(())
                            })
                            .and_then(|_| db::add_to_inventory(pid, &pending.item_id, pending.item_qty))
                            .map(|_| ())
                    }
                    Err(_) => {
                        trade_offer_clear(pid, target_id);
                        conn.send_json(&ActionResultMsg {
                            msg_type: "action_result".into(),
                            action: "Trade".into(),
                            target_id: target.id.clone(),
                            target_name: trade_target_name.clone(),
                            narrative: format!("【{trade_target_name}】抱歉地表示，該貨物眼下已售罄無存，這筆交易作廢。"),
                            success: false,
                            actions: vec![],
                            move_target_id: String::new(),
                            open_trade: false,
                            suggested_action: String::new(),
                            tactical_choices: vec![],
                        });
                        return false;
                    }
                }
            } else {
                trade_offer_clear(pid, target_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("【{trade_target_name}】手上的貨已不在，這筆交易作廢。"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return false;
            }
        } else {
            settle_inventory_trade(
                pid,
                target_id,
                &pending.item_id,
                pending.item_qty,
                paid,
                now,
                "player_trade",
            )
            .map(|_| ())
            .or_else(|_| {
                db::transfer_magnesium(pid, target_id, paid)
                    .or_else(|_| {
                        db::add_magnesium(pid, -paid)?;
                        let _ = db::add_magnesium(target_id, paid);
                        Ok(())
                    })
                    .and_then(|_| db::remove_from_inventory(target_id, &pending.item_id, pending.item_qty))
                    .and_then(|_| db::add_to_inventory(pid, &pending.item_id, pending.item_qty))
            })
        }
    };
    if settlement_result.is_err() {
        trade_offer_clear(pid, target_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "成交時交易結算失敗，請重新確認對方貨物與你的鎂。".into(),
            success: false,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
    }
    if let Ok(Some(buyer_after)) = db::get_entity(pid) {
        let expected_max_mg = (buyer.magnesium - paid).max(0);
        if buyer_after.magnesium > expected_max_mg {
            let diff = buyer_after.magnesium - expected_max_mg;
            let _ = db::add_magnesium(pid, -diff);
        }
    }
    let base_price = crate::world::materials::trade_price_anchor_mg(&pending.item_id)
        .or_else(|| {
            crate::game::peninsula_catalog::find_catalog_entry(&pending.item_id)
                .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
        })
        .or_else(|| {
            for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                if fg.id == pending.item_id {
                    return Some(fg.base_price);
                }
            }
            None
        })
        .unwrap_or(pending.ask_mg)
        .max(2);
    let player_room = super::resolve_action_room_name(pid, target);
    let _ = crate::game::object_ecology::record_settlement_market_trade(
        &player_room,
        &pending.item_id,
        base_price,
        -pending.item_qty,
        "玩家採買",
    );
    trade_offer_clear(pid, target_id);
    let _ = db::adjust_favorability(target_id, pid, 2);
    let _ = db::add_thought(target_id, crate::db::ThoughtSource::Trade);
    if let Some(pool) = &crate::pg::pool() {
        let _ = crate::pg::living_world::record_relationship_delta(
            pool,
            target_id,
            pid,
            "traded",
            1,
            "player_trade",
            now,
        );
        let _ = crate::pg::living_world::resolve_obligations_by_kind(
            pool,
            target_id,
            "stranded_cargo",
            now,
        );
        if crate::npc::debt_interaction::active_debt_amount(pid) == 0 {
            let _ = crate::pg::living_world::resolve_obligations_by_kind(
                pool,
                pid,
                "repay_debt",
                now,
            );
        }
        let player_room = get_entity_room(pid).unwrap_or_default();
        let _ = crate::pg::living_world::record_player_world_action(
            pool,
            pid,
            "trade",
            target_id,
            &player_room,
            "success",
            serde_json::json!({
                "item_id": pending.item_id,
                "paid_mg": paid,
                "ask_mg": pending.ask_mg,
                "offer_mg": offer,
            }),
            now,
        );
    }
    let item_name = get_item_def(&pending.item_id)
        .map(|d| d.name)
        .or_else(|| {
            crate::world::materials::material_by_item_id(&pending.item_id)
                .map(|m| m.name.to_string())
        })
        .or_else(|| {
            crate::game::peninsula_catalog::find_catalog_entry(&pending.item_id)
                .map(|c| c.name.to_string())
        })
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            if pending.item_id.is_ascii() {
                "隨身器物".to_string()
            } else {
                pending.item_id.clone()
            }
        });
    let player_name = db::get_entity(pid)
        .ok()
        .flatten()
        .map(|e| e.display_title)
        .unwrap_or_else(|| "遊歷客友".to_string());
    crate::npc::causal_core::mutate_npc_from_player_action(
        target_id,
        pid,
        &player_name,
        &player_room,
        "trade",
        &item_name,
        paid,
        now,
    );
    let delivered_cargo = crate::npc::causal_core::deliver_stranded_cargo_for_player(
        pid,
        &player_name,
        target_id,
        &player_room,
        now,
    );
    let _ = &delivered_cargo;
    let stock_suffix = if pending.item_id.starts_with("wt_cat_") {
        let rem = get_shop_item_stock(target_id, &pending.item_id);
        if rem <= 0 {
            "（這是店裡最後一件現貨，店家收拾妥當準備動身去補貨）".to_string()
        } else {
            if target_id.starts_with("wt_vendor_") {
                "（攤主俐落地將現貨包妥遞交予你）".to_string()
            } else {
                "（店家點清銀貨，將物件妥善交付予你）".to_string()
            }
        }
    } else {
        String::new()
    };
    let mut narrative = if offer >= pending.ask_mg {
        format!("你以開價 {paid} 鎂買下「{item_name}」。{stock_suffix}{fellowship_note}")
    } else {
        format!("一番議價後，你以 {paid} 鎂買下「{item_name}」。{stock_suffix}{fellowship_note}")
    };
    if let Some((order, bounty)) = delivered_cargo {
        narrative.push_str(&format!("（同行滯留貨單「{}」順利解送交付，公庫撥付賞格 {} 鎂）", order.item_name, bounty));
    }
    let ep_note = advance_room_event_pack_on_trade(&player_room, &pending.item_id, &item_name);
    if let Some(ep_line) = ep_note {
        narrative.push_str(&format!("\n{ep_line}"));
    }
    let _ = event::append(now, pid, types::TRADE, target_id);
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "Trade".into(),
        target_id: target.id.clone(),
        target_name: trade_target_name,
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
    push_refresh(conn, pid);
    false
}


pub(super) fn is_eligible_shop_staff(target: &Character, room_name: &str) -> bool {
    let t_id = target.id.as_str();
    let title = target.display_title.as_str();

    // 衝突外單位職稱或訪客身份，嚴禁代表商鋪賣店貨
    const CONFLICTING_KEYWORDS: &[&str] = &[
        "學生", "學員", "學子", "童生", "生員",
        "編輯", "校勘", "校書生", "記錄員", "抄寫員", "抄錄書生", "書僮", "書童",
        "訪客", "遊客", "香客", "旅人", "食客", "茶客", "讀者", "信眾", "遊俠", "客商", "外來者",
    ];
    if CONFLICTING_KEYWORDS.iter().any(|k| title.contains(k) || t_id.contains(k) || target.current_activity.contains(k)) {
        return false;
    }

    // 檢查若有指定外單位機構前綴且與當前場所衝突
    const ORG_PREFIXES: &[&str] = &[
        "藏書閣", "文曜學院", "公議堂", "東橋旅社", "三水大食堂",
        "三水總商行", "泉香溫水浴場", "溫氏診所", "岑氏鍛造", "鍛造工坊", "鍛造", "木作工坊", "木作", "大圳織造", "織造",
        "風舶聯運", "水尾寮早市", "算學舍", "鐘樓", "蒙學堂", "觀星台", "習武堂", "剪剃鋪", "柑仔店", "百工商算學舍", "文曜蒙學堂",
    ];
    for org in ORG_PREFIXES {
        if (title.contains(org) || t_id.contains(org)) && !room_name.is_empty() && !room_name.contains(org) {
            return false;
        }
    }

    // 檢查居民 household 排班之工作場所是否為衝突外單位
    let slot = crate::store::wenyao_household_slot_for(&target.id);
    if !slot.work_name.is_empty() {
        let work = slot.work_name;
        let is_same_venue = room_name.is_empty()
            || room_name.contains(work) || work.contains(room_name)
            || (room_name.contains("商行") && work.contains("商行"))
            || (room_name.contains("旅社") && (work.contains("旅社") || work.contains("茶屋") || work.contains("歇舍") || work.contains("臨水")))
            || (room_name.contains("食堂") && work.contains("食堂"))
            || (room_name.contains("工坊") && work.contains("工坊"))
            || (room_name.contains("診所") && work.contains("診所"))
            || (room_name.contains("浴場") && work.contains("浴場"));
        if !is_same_venue && (work.contains("學院") || work.contains("藏書閣") || work.contains("公議堂")
            || work.contains("學堂") || work.contains("診所") || work.contains("官署") || work.contains("巡防")
            || work.contains("鍛造") || work.contains("工坊") || work.contains("商行") || work.contains("旅社")
            || work.contains("食堂") || work.contains("浴場") || work.contains("織造") || work.contains("鐘樓")
            || work.contains("車隊") || work.contains("貨運") || work.contains("鋪") || work.contains("店")) {
            return false;
        }
    }

    // 整合現場人員辨識：制服／圍裙／工務裝束或站在管制櫃檯區
    let mut probe = crate::store::Entity::new_npc(&target.id, &target.display_char);
    probe.display_title = if target.current_activity.is_empty() {
        target.display_title.clone()
    } else {
        format!("{}|{}", target.display_title, target.current_activity)
    };
    probe.current_activity = target.current_activity.clone();
    probe.equipment_slots = target.equipment_slots.clone();
    probe.inventory = target.inventory.clone();
    probe.grid_x = target.grid_x;
    probe.grid_y = target.grid_y;
    probe.magnesium = target.magnesium;

    let (has_uniform, in_counter_zone) =
        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder_entity(
            &probe,
            (target.grid_x.unwrap_or(0), target.grid_y.unwrap_or(0)),
            "",
        );
    if has_uniform || in_counter_zone {
        return true;
    }

    // 正向判定：具備店東、店員、幫工、大廚等商鋪職銜，或工作場所相符
    const STAFF_KEYWORDS: &[&str] = &[
        "掌事", "店主", concat!("店", "長"), "店員", "店助", concat!("助", "理"), "幫工", "學徒", "師傅",
        "匠", "主事", "管事", "大廚", "廚工", "跑堂", "侍者", "侍應", "服務生", "販", "商",
        "茶師", "藥師", "工", "役", "東家", concat!("掌", "櫃"), concat!("老", "闆"), "掌勺",
    ];
    if STAFF_KEYWORDS.iter().any(|k| title.contains(k) || t_id.contains(k)) {
        return true;
    }
    if !slot.work_name.is_empty() && !room_name.is_empty() && (room_name.contains(slot.work_name) || slot.work_name.contains(room_name)) {
        return true;
    }
    let pers_role = crate::store::npc_persistent_role_label(&target.id, &target.display_title);
    if STAFF_KEYWORDS.iter().any(|k| pers_role.contains(k)) {
        return true;
    }

    // 若未被判定為衝突職稱，且無其他工作衝突
    if slot.work_name.is_empty() {
        let is_commercial_venue = room_name.is_empty()
            || [
                "店", "鋪", "攤", "坊", "行", "市", "商", "堂", "閣", "館", "舍", "樓",
                "局", "所", "屋", "社", "場", "食堂", "旅社", "診所", "浴場", "工坊",
                "書局", "早市", "集市", "拍賣",
            ]
            .iter()
            .any(|k| room_name.contains(k));

        let is_passerby = target.current_activity.contains("路過")
            || target.current_activity.contains("閒逛")
            || target.current_activity.contains("茶客")
            || target.current_activity.contains("食客");

        let is_on_duty_or_active = has_uniform
            || in_counter_zone
            || target.current_activity.is_empty()
            || target.current_activity.contains("當值")
            || target.current_activity.contains("櫃檯")
            || target.current_activity.contains("看顧")
            || target.current_activity.contains("掌");

        if is_commercial_venue && !is_passerby && is_on_duty_or_active {
            return true;
        }
    }

    false
}

pub(super) fn canonical_shopkeeper_trade_offer(
    target: &Character,
    room_name: &str,
    now: i64,
) -> Option<(String, String, i32)> {
    let t_id = target.id.as_str();
    let title = target.display_title.as_str();

    let hour = crate::store::current_game_hour();
    let is_always_open = title.contains("旅社")
        || title.contains("客舍")
        || title.contains("夜班")
        || room_name.contains("旅社")
        || room_name.contains("風舶臨水旅社")
        || room_name.contains("北灣臨水旅社")
        || room_name.contains("臨水旅社")
        || room_name.contains("歇舍");
    let in_hours = if is_always_open {
        true
    } else if t_id.starts_with("wt_vendor_") {
        !target.current_activity.contains("安歇")
    } else {
        (6..=21).contains(&hour)
            || target.current_activity.contains("當值")
            || target.current_activity.contains("櫃檯")
    };
    if !in_hours {
        return None;
    }

    if let Some((item_id, item_name, price_mg, _)) = vendor_stall_offer_for_npc(t_id) {
        if t_id.starts_with("wt_stall_") || t_id.starts_with("wt_vendor_") {
            return Some((item_id.to_string(), item_name.to_string(), price_mg));
        }
    }

    if is_eligible_shop_staff(target, room_name) {
        if let Some(venue) = crate::grid::resolve_room_venue_object(room_name, title) {
            if !venue.counter_trade_offers.is_empty() {
                let idx = ((now.unsigned_abs() as usize) / 60) % venue.counter_trade_offers.len();
                let (id, name, price) = venue.counter_trade_offers[idx];
                return Some((id.to_string(), name.to_string(), price));
            }
        }
    }
    if !is_shop_person(target, room_name) {
        return None;
    }

    if let Some(venue) = crate::grid::resolve_market_or_specialty_room_object(room_name, title) {
        if !venue.counter_trade_offers.is_empty() {
            let idx = ((now.unsigned_abs() as usize) / 60) % venue.counter_trade_offers.len();
            let (id, name, price) = venue.counter_trade_offers[idx];
            return Some((id.to_string(), name.to_string(), price));
        }
    }

    None
}

pub(super) fn is_shop_person(target: &Character, room_name: &str) -> bool {
    is_eligible_shop_staff(target, room_name)
}

pub(super) fn target_has_shop_offer(target: &Character, room_name: &str) -> bool {
    let hour = crate::store::current_game_hour();
    let is_always_open = target.display_title.contains("旅社")
        || target.display_title.contains("客舍")
        || target.display_title.contains("夜班")
        || room_name.contains("旅社")
        || room_name.contains("風舶臨水旅社")
        || room_name.contains("北灣臨水旅社")
        || room_name.contains("臨水旅社")
        || room_name.contains("歇舍");
    let in_hours = if is_always_open {
        true
    } else if target.id.starts_with("wt_vendor_") {
        (6..=22).contains(&hour)
    } else {
        (6..=21).contains(&hour)
    };
    if !in_hours {
        return false;
    }
    canonical_shopkeeper_trade_offer(target, room_name, 0).is_some()
}

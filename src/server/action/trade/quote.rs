fn is_trade_reject_input(s: &str) -> bool {
    matches!(
        s.trim().to_lowercase().as_str(),
        "拒絕" | "取消" | "算了" | "不要" | "no" | "n"
    )
}
fn apply_player_sale_payout_with_debt_settlement(pid: &str, payout: i32, now: i64) -> (i32, i32, String) {
    let active_debt = crate::npc::debt_interaction::active_debt_amount(pid);
    if active_debt > 0 && payout > 0 {
        let max_deduct = (payout / 2).max(1).min(payout);
        let repay_amount = max_deduct.min(active_debt);
        let cleared = crate::npc::debt_interaction::settle_debt_for_actor(pid, repay_amount);
        let actual_payout = (payout - cleared).max(0);
        let _ = db::add_magnesium(pid, actual_payout);
        if let Some(pool) = crate::pg::pool() {
            if crate::npc::debt_interaction::active_debt_amount(pid) == 0 {
                let _ = crate::pg::living_world::resolve_obligations_by_kind(&pool, pid, "repay_debt", now);
            }
        }
        let note = if cleared > 0 {
            format!("「順帶清償了先前記在帳上的 {cleared} 鎂賒欠。」")
        } else {
            String::new()
        };
        (actual_payout, cleared, note)
    } else {
        let _ = db::add_magnesium(pid, payout);
        (payout, 0, String::new())
    }
}

fn handle_craft_mutate_sell_trade(
    conn: &WsConnection,
    pid: &str,
    target_id: &str,
    target: &Character,
    now: i64,
    trade_target_name: &str,
    player_trade_input: &str,
) -> bool {
    let trade_target_name = trade_target_name.to_string();
    let is_craft_intent = player_trade_input.starts_with("煉製")
        || player_trade_input.starts_with("加工")
        || player_trade_input.to_lowercase().starts_with("craft");
    if is_craft_intent {
        let raw_query = player_trade_input
            .trim_start_matches(|c: char| c == '煉' || c == '製' || c == '加' || c == '工' || c == 'c' || c == 'C' || c == 'r' || c == 'R' || c == 'a' || c == 'A' || c == 'f' || c == 'F' || c == 't' || c == 'T')
            .trim();
        let room_name = db::get_entity_room(pid)
            .ok()
            .and_then(|r| db::get_room_name(&r).ok())
            .unwrap_or_else(|| "白塔商市".to_string());
        match crate::game::object_ecology::craft_from_raw_material(pid, if raw_query.is_empty() { "auto" } else { raw_query }, &room_name, now) {
            Ok((_item_id, _item_name, _is_mut, craft_narrative)) => {
                push_refresh(conn, pid);
                let _ = db::adjust_favorability(target_id, pid, 1);
                let _ = event::append(now, pid, "craft_trade", target_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("你請【{trade_target_name}】協同掌爐加工：\n{craft_narrative}"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
            }
            Err(err) => {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("【{trade_target_name}】看過你遞上的原料清單後道：「{err}」"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
            }
        }
        return true;
    }

    let is_mutate_intent = player_trade_input.starts_with("異變")
        || player_trade_input.to_lowercase().starts_with("mutate");
    if is_mutate_intent {
        let item_query = player_trade_input
            .trim_start_matches(|c: char| c == '異' || c == '變' || c == 'm' || c == 'M' || c == 'u' || c == 'U' || c == 't' || c == 'T' || c == 'a' || c == 'A' || c == 'e' || c == 'E')
            .trim();
        let room_name = db::get_entity_room(pid)
            .ok()
            .and_then(|r| db::get_room_name(&r).ok())
            .unwrap_or_else(|| "白塔商市".to_string());
        match crate::game::object_ecology::mutate_finished_item(pid, if item_query.is_empty() { "auto" } else { item_query }, &room_name, now) {
            Ok((_mut_id, _mut_name, mutate_narrative)) => {
                push_refresh(conn, pid);
                let _ = db::adjust_favorability(target_id, pid, 2);
                let _ = event::append(now, pid, "mutate_trade", target_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("你請【{trade_target_name}】共同護持氣脈淬火：\n{mutate_narrative}"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
            }
            Err(err) => {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("【{trade_target_name}】凝神審視你的隨身器物與氣脈後道：「{err}」"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
            }
        }
        return true;
    }
    let is_selling_intent = player_trade_input.starts_with("賣")
        || player_trade_input.starts_with("售")
        || player_trade_input.to_lowercase().starts_with("sell");
    if is_selling_intent {
        let raw_query = player_trade_input
            .trim_start_matches(|c: char| c == '賣' || c == '售' || c == 's' || c == 'S' || c == 'e' || c == 'E' || c == 'l' || c == 'L')
            .trim();
        let Ok(Some(player_ent)) = db::get_entity(pid) else {
            conn.send_error("玩家實體不存在");
            return true;
        };
        let inv = db::get_inventory(&player_ent.inventory, player_ent.vit);
        let candidate = if !raw_query.is_empty() {
            inv.items.into_iter().find(|it| {
                it.item_id.contains(raw_query)
                    || get_item_def(&it.item_id).is_some_and(|d| d.name.contains(raw_query))
                    || crate::game::peninsula_catalog::find_catalog_entry(&it.item_id).is_some_and(|c| c.name.contains(raw_query))
                    || crate::game::object_ecology::lookup_ecology_item_def(&it.item_id).is_some_and(|e| e.name.contains(raw_query))
                    || crate::db::fallback_item_name(&it.item_id).is_some_and(|n| n.contains(raw_query))
            })
        } else {
            inv.items.into_iter().find(|it| {
                it.item_id.starts_with("mat_")
                    || it.item_id.starts_with("harvest_")
                    || it.item_id.starts_with("craft_")
                    || it.item_id.starts_with("wt_cat_")
                    || it.item_id.starts_with("wt-good-")
                    || it.item_id.starts_with("shanhai_")
                    || it.item_id.starts_with("mut_")
                    || crate::game::object_ecology::lookup_ecology_item_def(&it.item_id).is_some()
                    || crate::world::materials::trade_price_anchor_mg(&it.item_id).is_some()
            })
        };

        if let Some(item) = candidate {
            let item_name = get_item_def(&item.item_id)
                .map(|d| d.name)
                .or_else(|| {
                    crate::world::materials::material_by_item_id(&item.item_id)
                        .map(|m| m.name.to_string())
                })
                .or_else(|| {
                    crate::game::peninsula_catalog::find_catalog_entry(&item.item_id)
                        .map(|c| c.name.to_string())
                })
                .or_else(|| {
                    crate::game::object_ecology::lookup_ecology_item_def(&item.item_id)
                        .map(|d| d.name)
                })
                .or_else(|| crate::db::fallback_item_name(&item.item_id).map(|s| s.to_string()))
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| {
                    if item.item_id.is_ascii() {
                        "隨身器物".to_string()
                    } else {
                        item.item_id.clone()
                    }
                });

            let base_price = crate::world::materials::trade_price_anchor_mg(&item.item_id)
                .or_else(|| {
                    crate::game::peninsula_catalog::find_catalog_entry(&item.item_id)
                        .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
                })
                .or_else(|| {
                    for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                        if fg.id == item.item_id {
                            return Some(fg.base_price);
                        }
                    }
                    get_item_def(&item.item_id).map(|d| (d.weight * 15.0).round() as i32)
                })
                .unwrap_or(3)
                .max(3);

            let player_room = db::get_entity_room(pid)
                .ok()
                .and_then(|r| db::get_room_name(&r).ok())
                .unwrap_or_else(|| "文曜".to_string());

            let quote = crate::game::object_ecology::dynamic_settlement_market_quote(
                &player_room,
                &item.item_id,
                base_price,
            );
            let unit_price = quote.unit_price;

            if db::remove_from_inventory(pid, &item.item_id, 1).is_ok() {
                let (_actual_payout, _cleared, debt_note) =
                    apply_player_sale_payout_with_debt_settlement(pid, unit_price, now);
                let _ = db::add_to_inventory(target_id, &item.item_id, 1);
                let npc_paid = target.magnesium.max(0).min(unit_price);
                if npc_paid > 0 {
                    let _ = db::add_magnesium(target_id, -npc_paid);
                }
                let _after_quote = crate::game::object_ecology::record_settlement_market_trade(
                    &player_room,
                    &item.item_id,
                    base_price,
                    1,
                    "玩家售出",
                );
                push_refresh(conn, pid);
                let _ = db::adjust_favorability(target_id, pid, 1);
                let _ = event::append(now, pid, "trade", target_id);
                let ep_note = advance_room_event_pack_on_trade(&player_room, &item.item_id, &item_name);
                let ep_suffix = if let Some(ep_line) = ep_note {
                    format!("\n{ep_line}")
                } else {
                    String::new()
                };
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: {
                        let debt_suffix = if debt_note.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", debt_note)
                        };
                        format!(
                            "【{trade_target_name}】端詳你遞上的【{item_name}】（{}），依當地供需行情以 {} 鎂收下。雙方款貨兩訖，妥善收訖入庫。{debt_suffix}{ep_suffix}",
                            quote.market_narrative,
                            quote.unit_price
                        )
                    },
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return true;
            }
        } else {
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Trade".into(),
                target_id: target.id.clone(),
                target_name: trade_target_name.clone(),
                narrative: format!("你向【{trade_target_name}】提出售賣，但行囊中目前沒有可供收購的物資。"),
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return true;
        }
    }
    false
}

fn handle_trade_quotation(
    conn: &WsConnection,
    pid: &str,
    target_id: &str,
    target: &Character,
    now: i64,
    trade_target_name: &str,
    player_trade_input: &str,
) -> bool {
        let trade_target_name = trade_target_name.to_string();
        if !player_trade_input.is_empty() {
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Trade".into(),
                target_id: target.id.clone(),
                target_name: trade_target_name.clone(),
                narrative: "對方尚未開價。請先點【交易】取得報價，再在輸入欄填寫出價（鎂）。"
                    .into(),
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return false;
        }
        let personal_item = pick_entity_trade_offer(&target.inventory, &target.equipment_slots);
        let market_offer = if personal_item.is_none() {
            crate::pg::pool().and_then(
                |pool| match crate::pg::world_market::offer_for_operator(
                    &pool, target_id, pid, now,
                ) {
                    Ok(offer) => offer,
                    Err(error) => {
                        tracing::warn!("market offer failed {target_id}/{pid}: {error}");
                        None
                    }
                },
            )
        } else {
            None
        };
        let room_name = super::resolve_action_room_name(pid, target);
        let canonical_offer = if personal_item.is_none() && market_offer.is_none() {
            canonical_shopkeeper_trade_offer(target, &room_name, now)
        } else {
            None
        };
        if personal_item.is_none() && market_offer.is_none() && canonical_offer.is_none() {
            let player_sell_mat = db::get_entity(pid).ok().flatten().and_then(|player_ent| {
                let inv = db::get_inventory(&player_ent.inventory, player_ent.vit);
                inv.items.into_iter().find(|it| {
                    it.item_id.starts_with("mat_")
                        || it.item_id.starts_with("harvest_")
                        || it.item_id.starts_with("craft_")
                        || it.item_id.starts_with("wt_cat_")
                        || it.item_id.starts_with("wt-good-")
                        || it.item_id.starts_with("shanhai_")
                        || it.item_id.starts_with("mut_")
                        || crate::game::object_ecology::lookup_ecology_item_def(&it.item_id).is_some()
                        || crate::world::materials::trade_price_anchor_mg(&it.item_id).is_some()
                })
            });

            if let Some(mat_item) = player_sell_mat {
                let resolve_item_display_name = |item_id: &str| -> String {
                    get_item_def(item_id)
                        .map(|d| d.name)
                        .or_else(|| {
                            crate::world::materials::material_by_item_id(item_id)
                                .map(|m| m.name.to_string())
                        })
                        .or_else(|| {
                            crate::game::peninsula_catalog::find_catalog_entry(item_id)
                                .map(|c| c.name.to_string())
                        })
                        .or_else(|| {
                            crate::game::object_ecology::lookup_ecology_item_def(item_id)
                                .map(|d| d.name)
                        })
                        .or_else(|| crate::db::fallback_item_name(item_id).map(|s| s.to_string()))
                        .filter(|n| !n.trim().is_empty())
                        .unwrap_or_else(|| {
                            if item_id.is_ascii() {
                                "隨身器物".to_string()
                            } else {
                                item_id.to_string()
                            }
                        })
                };
                let item_name = resolve_item_display_name(&mat_item.item_id);

                let base_price = crate::world::materials::trade_price_anchor_mg(&mat_item.item_id)
                    .or_else(|| {
                        crate::game::peninsula_catalog::find_catalog_entry(&mat_item.item_id)
                            .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
                    })
                    .or_else(|| {
                        for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                            if fg.id == mat_item.item_id {
                                return Some(fg.base_price);
                            }
                        }
                        None
                    })
                    .unwrap_or(3)
                    .max(3);
                let player_room = {
                    let raw = if !room_name.is_empty() {
                        room_name.clone()
                    } else {
                        db::get_entity_room(pid)
                            .ok()
                            .and_then(|r| db::get_room_name(&r).ok())
                            .unwrap_or_default()
                    };
                    if raw.is_empty() {
                        "文曜".to_string()
                    } else if raw.contains('・') {
                        raw.split('・').next().unwrap_or(&raw).trim().to_string()
                    } else {
                        raw
                    }
                };

                let quote = crate::game::object_ecology::dynamic_settlement_market_quote(
                    &player_room,
                    &mat_item.item_id,
                    base_price,
                );
                let _unit_price = quote.unit_price;

                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!(
                        "【{trade_target_name}】見你行囊中攜有【{item_name}】（{}），表示願按當下行情以 {} 鎂收購。若欲售出，請輸入「賣 {item_name}」，或輸入「拒絕」取消。",
                        quote.market_narrative,
                        quote.unit_price
                    ),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: true,
                    suggested_action: format!("賣 {item_name}"),
                    tactical_choices: vec![],
                });
                return false;
            }

            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Trade".into(),
                target_id: target.id.clone(),
                target_name: trade_target_name.clone(),
                narrative: {
                    let mut narr = format!(
                        "你向【{trade_target_name}】提出交易，對方表示目前暫無可交易之物。"
                    );
                    let report = crate::game::object_ecology::format_settlement_supply_demand_report(&room_name);
                    if !report.is_empty() {
                        narr.push_str(&format!(
                            "對方順口指點道：「我手頭現下並無餘貨可讓渡；客友若想採買或出手貨物，{}」",
                            report
                        ));
                    }
                    narr
                },
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return false;
        }
        let is_canonical_offer = canonical_offer.is_some();
        let (item_id, item_name, ask, market_id, stock_note) = if let Some(offer) = market_offer {
            (
                offer.item_id,
                offer.display_name,
                offer.ask_mg,
                Some(offer.source_market_id),
                String::new(),
            )
        } else if let Some((cat_id, cat_name, mut cat_ask)) = canonical_offer {
            let active_ob = crate::store::get_active_npc_obligation(&target.id);
            let has_active_restock = active_ob
                .as_ref()
                .map(|(k, _)| k == "restock_goods")
                .unwrap_or(false);
            let current_stock = get_shop_item_stock(&target.id, &cat_id);
            if !has_active_restock && current_stock <= 0 {
                restock_shop_items_for_seller(&target.id);
            }
            let stock = get_shop_item_stock(&target.id, &cat_id);
            if stock <= 0 {
                crate::store::open_active_npc_obligation(
                    &target.id,
                    "restock_goods",
                    &format!("店內【{}】已售罄急需提貨補倉", cat_name),
                );
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("【{trade_target_name}】看了一眼空蕩的貨架，向你抱歉地笑說：「實在不巧，店裡的【{}】現貨全被買空了，我正準備去總商行與聯運站提領新貨，勞駕稍候再來。」", cat_name),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return false;
            }
            let note = if stock == 1 {
                let bump = ((cat_ask as f64 * 0.15).round() as i32).max(1);
                cat_ask += bump;
                "（架上現貨僅餘最後 1 件，物稀價微揚）".to_string()
            } else if target.id.starts_with("wt_vendor_") {
                "（攤上尚有現貨）".to_string()
            } else {
                "（店內尚有存貨）".to_string()
            };
            let quote = crate::game::object_ecology::dynamic_settlement_market_quote(&room_name, &cat_id, cat_ask);
            cat_ask = quote.unit_price;
            (cat_id, cat_name, cat_ask, None, note)
        } else {
            let item_id = personal_item.expect("personal or market offer checked");
            let item_name = get_item_def(&item_id)
                .map(|definition| definition.name)
                .or_else(|| {
                    crate::world::materials::material_by_item_id(&item_id)
                        .map(|m| m.name.to_string())
                })
                .or_else(|| {
                    crate::game::peninsula_catalog::find_catalog_entry(&item_id)
                        .map(|c| c.name.to_string())
                })
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| {
                    if item_id.is_ascii() {
                        "隨身器物".to_string()
                    } else {
                        item_id.clone()
                    }
                });
            let ask = default_trade_ask_mg(&item_id);
            let quote = crate::game::object_ecology::dynamic_settlement_market_quote(&room_name, &item_id, ask);
            (item_id, item_name, quote.unit_price, None, String::new())
        };
        let (floor, _trust, rel_note) = compute_relationship_trade_terms(pid, target_id, ask);
        trade_offer_set(
            pid,
            TradePending {
                npc_id: target_id.to_string(),
                item_id: item_id.clone(),
                item_qty: 1,
                ask_mg: ask,
                floor_mg: floor,
                market_id,
                expires_unix: 0,
            },
        );
        let narrative = if market_id.is_some() {
            format!(
                "【{trade_target_name}】查過此地市場現有的貨，願意交付「{item_name}」一份，開價 {ask} 鎂{rel_note}，若有誠意亦可略作講價。請輸入你願付的鎂數，或輸入「拒絕」取消。"
            )
        } else if is_canonical_offer {
            let stock_place = if target.id.starts_with("wt_vendor_") {
                "攤車上的現貨"
            } else {
                "店內架上的存貨"
            };
            format!(
                "【{trade_target_name}】看了一下{stock_place}，願意賣出「{item_name}」一份，開價 {ask} 鎂{stock_note}{rel_note}，若有誠意亦可略作講價。請輸入你願付的鎂數，或輸入「拒絕」取消。"
            )
        } else {
            format!(
                "【{trade_target_name}】願意從隨身備品中勻出「{item_name}」一份，開價 {ask} 鎂{rel_note}，若有誠意亦可略作講價。請輸入你願付的鎂數，或輸入「拒絕」取消。"
            )
        };
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name,
            narrative: {
                if market_id.is_some() {
                    let report = crate::game::object_ecology::format_settlement_supply_demand_report(&room_name);
                    if report.is_empty() {
                        narrative
                    } else {
                        format!("{narrative}\n{report}")
                    }
                } else {
                    narrative
                }
            },
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: true,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return false;
}

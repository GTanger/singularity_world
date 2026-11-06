pub fn step_offline_player_once(player_id: &str, now_unix: i64) -> Option<String> {
    if crate::server::session::list_online_player_ids()
        .iter()
        .any(|id| id == player_id)
    {
        return None;
    }
    let (settlement, player_name, place_id, is_exhausted, vit, magnesium, soul_seed, eq_slots, inv_json, ent_opt) =
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get(player_id) {
                let place = e.place_id.as_deref().unwrap_or("");
                let clean = clean_settlement_name(place);
                let s_name = if clean.is_empty() { "文曜" } else { clean };
                (
                    s_name.to_string(),
                    if e.display_title.is_empty() {
                        "行旅者".to_string()
                    } else {
                        e.display_title.clone()
                    },
                    e.place_id.clone(),
                    e.vit < 30,
                    e.vit,
                    e.magnesium,
                    e.soul_seed.unwrap_or(42) as u64,
                    e.equipment_slots.clone(),
                    e.inventory.clone(),
                    Some(e.clone()),
                )
            } else {
                (
                    "文曜".to_string(),
                    "行旅者".to_string(),
                    None,
                    false,
                    80,
                    20,
                    42,
                    String::new(),
                    String::new(),
                    None,
                )
            }
        } else {
            (
                "文曜".to_string(),
                "行旅者".to_string(),
                None,
                false,
                80,
                20,
                42,
                String::new(),
                String::new(),
                None,
            )
        };

    let mut dummy_ent = crate::store::Entity::default();
    dummy_ent.id = player_id.to_string();
    dummy_ent.display_title = player_name.clone();
    dummy_ent.equipment_slots = eq_slots;
    dummy_ent.inventory = inv_json;

    let (cx, cy) = if let Some(ent) = &ent_opt {
        (ent.grid_x.unwrap_or(0), ent.grid_y.unwrap_or(0))
    } else {
        (0, 0)
    };
    let cur_pos = if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        s.entities
            .get(player_id)
            .and_then(|e| e.grid_x.zip(e.grid_y))
            .unwrap_or((cx, cy))
    } else {
        (cx, cy)
    };

    let factors = if let Some(ent) = &ent_opt {
        crate::pg::living_world::ResidentIntrinsicFactors::resolve(
            ent,
            cur_pos.0,
            cur_pos.1,
            &settlement,
            &ent.current_activity,
        )
    } else {
        crate::pg::living_world::ResidentIntrinsicFactors::resolve(
            &dummy_ent,
            0,
            0,
            &settlement,
            "",
        )
    };
    let (tool_id, tool_name, is_bladed, vstate, raw_stock_qty, finished_stock_qty) =
        crate::pg::living_world::inspect_store_entity_live_stock_and_tool(&dummy_ent);

    let (weather, tide) = crate::game::zone_weather_and_tide(&settlement, now_unix);
    let chosen_action = crate::pg::living_world::choose_resident_life_action(
        &player_name,
        &settlement,
        soul_seed,
        vit,
        magnesium,
        raw_stock_qty,
        finished_stock_qty,
        if is_bladed { vstate.wear_points } else { 0 },
        crate::store::get_active_npc_obligation(player_id).is_some(),
        crate::store::current_game_hour() as i32,
        weather,
        tide,
        vstate.use_count.max(0) as u32,
        &get_relational_hooks(player_id),
    );

    if is_bladed && vstate.wear_points >= 3 && chosen_action == crate::pg::living_world::ResidentLifeActionChoice::RepairTool {
        let _ = crate::pg::living_world::sharpen_vessel_at_grindstone(player_id, &tool_id, now_unix)
            .or_else(|_| Ok::<_, &str>(crate::pg::living_world::temper_vessel_at_forge(player_id, &tool_id, now_unix)));
        let act = format!("於{}工坊磨石前推磨修整{}，校準刃口鋒芒", settlement, tool_name);
        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let _ = s.update_entity(player_id, |e| {
                e.current_activity = act.clone();
            });
            s.append_thought(
                player_id,
                crate::db::ThoughtSource::WorkDone,
                format!("於{}工坊磨石前推磨修整{}，校準刃口鋒芒，手感漸入佳境", settlement, tool_name),
                now_unix,
            );
        }
        let _ = crate::pg::living_world::record_breath_activity(
            player_id,
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            2,
            now_unix,
        );
        let title = "駐坊磨器 校準鋒芒".to_string();
        let body = format!("於{}工坊活水磨石前推磨修整隨身{}，細緻校準刃口分寸，器物重歸鋒利。", settlement, tool_name);
        let category = "social".to_string();
        {
            let mut recaps = PLAYER_OFFLINE_RECAPS
                .write()
                .unwrap_or_else(|e| e.into_inner());
            let list = recaps.entry(player_id.to_string()).or_default();
            if let Some(existing) = list.iter_mut().find(|(_, t, _)| t == &title) {
                *existing = (category, title, body.clone());
            } else {
                list.push((category, title, body.clone()));
                if list.len() > 4 {
                    list.remove(0);
                }
            }
        }
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let next_wake = now_unix + 60;
            let _ = client.execute(
                "INSERT INTO resident_continuity (
                    entity_id, control_mode, life_status, current_intent,
                    intent_reason, destination_place_id, intent_started_at,
                    next_wake_at, updated_at, source_kind, source_ref
                ) VALUES (
                    $1, 'autonomous', 'alive', 'repair_tool',
                    $2, $3, $4,
                    $5, $4, 'offline_autonomy', 'step_offline_player_once'
                )
                ON CONFLICT (entity_id) DO UPDATE SET
                    control_mode = 'autonomous',
                    life_status = 'alive',
                    current_intent = EXCLUDED.current_intent,
                    intent_reason = EXCLUDED.intent_reason,
                    destination_place_id = EXCLUDED.destination_place_id,
                    blocked_by = NULL,
                    next_wake_at = EXCLUDED.next_wake_at,
                    source_kind = EXCLUDED.source_kind,
                    source_ref = EXCLUDED.source_ref,
                    updated_at = EXCLUDED.updated_at",
                &[
                    &player_id,
                    &format!("於{}工坊推磨修整{}", settlement, tool_name),
                    &place_id,
                    &now_unix,
                    &next_wake,
                ],
            );
        }
        return Some(body);
    }

    let mut directive = get_player_offline_directive(player_id);
    if is_exhausted {
        directive = OfflineAutonomyDirective::InnRestAndHeal;
    }

    let (title, body, (dest_x, dest_y, dest_bldg)) = match directive {
        OfflineAutonomyDirective::InnRestAndHeal => {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(player_id, |e| {
                    if is_exhausted {
                        e.vit = (e.vit + 10).min(100);
                        e.qi = (e.qi + 10).min(100);
                    } else {
                        e.vit = (e.vit + 8).min(100);
                        e.qi = (e.qi + 8).min(100);
                        e.dex = (e.dex + 4).min(100);
                    }
                    e.current_activity = "於客舍內閉門靜養、調勻氣血".to_string();
                });
            }
            crate::pg::living_world::clear_entity_afflictions(player_id, true, now_unix);
            let is_indoors = if let Some(arc) = crate::store::get_store() {
                let s = arc.read();
                s.entities
                    .get(player_id)
                    .and_then(|e| e.place_id.as_deref())
                    .is_some_and(|p| {
                        p.contains("居")
                            || p.contains("舍")
                            || p.contains("宅")
                            || p.contains("客")
                            || p.contains("旅")
                            || p.contains("宿")
                            || p.contains("房")
                    })
            } else {
                false
            };
            let (t, b) = if is_indoors || is_exhausted {
                (
                    "客舍安歇 調勻氣血".to_string(),
                    "於房舍內閉門靜養、調勻氣血，舊傷與勞頓隨之舒緩散去。".to_string(),
                )
            } else {
                (
                    "行旅小憩 調勻氣息".to_string(),
                    "行旅駐足期間靜心調勻氣息，舒緩奔波勞頓，身心漸歸安寧。".to_string(),
                )
            };
            (t, b, (factors.home_coord.0, factors.home_coord.1, factors.home_bldg.clone()))
        }
        OfflineAutonomyDirective::WorkshopCraftAndAssist => {
            let hooks = get_relational_hooks(player_id);
            let pw = PluralPersonalityWeights::default();
            let runtime_grid = crate::server::grid_manager::get_runtime_square_grid();

            let is_real_resident = ent_opt.is_some();
            if is_real_resident && raw_stock_qty <= 0 {
                let sup_opt = if let Some(arc) = crate::store::get_store() {
                    let s = arc.read();
                    crate::pg::living_world::resolve_need_decision_supplier(
                        &s,
                        player_id,
                        cur_pos,
                        &factors,
                        &pw,
                        &hooks,
                        runtime_grid.as_ref(),
                    )
                } else {
                    None
                };
                let (sup_id, sup_place, (sup_x, sup_y)) = sup_opt.unwrap_or_else(|| {
                    let coord = factors.supplier_coord.unwrap_or(factors.work_coord);
                    let place = if factors.supplier_place_label.is_empty() {
                        factors.work_bldg.clone()
                    } else {
                        factors.supplier_place_label.clone()
                    };
                    (format!("{}櫃檯", place), place, coord)
                });
                let sup_person = if sup_id.ends_with("櫃檯") {
                    sup_id.clone()
                } else {
                    let n = crate::store::resolve_npc_person_name(&sup_id, "", None);
                    if n.is_empty() || n.bytes().any(|b| b.is_ascii_alphabetic()) {
                        format!("{}櫃檯", sup_place)
                    } else {
                        n
                    }
                };

                let on_site = (cur_pos.0 - sup_x).abs() + (cur_pos.1 - sup_y).abs() <= 1;
                if on_site {
                    let buy_qty = factors.restock_qty;
                    let cost_mg = factors.restock_cost_mg.min(magnesium).max(0);
                    if let Some(arc) = crate::store::get_store() {
                        let mut s = arc.write();
                        let _ = s.update_entity(player_id, |e| {
                            e.current_activity = format!(
                                "在{}向{}採買{}，補充工坊原料",
                                sup_place, sup_person, factors.raw_item_name
                            );
                            e.magnesium = e.magnesium.saturating_sub(cost_mg);
                            e.vit = (e.vit - 2).max(20);
                            let mut entries: Vec<crate::db::inventory::InventoryEntry> =
                                serde_json::from_str(&e.inventory).unwrap_or_default();
                            if let Some(entry) =
                                entries.iter_mut().find(|it| it.item_id == factors.raw_item_id)
                            {
                                entry.qty += buy_qty;
                            } else {
                                entries.push(crate::db::inventory::InventoryEntry {
                                    item_id: factors.raw_item_id.clone(),
                                    qty: buy_qty,
                                });
                            }
                            if let Ok(serialized) = serde_json::to_string(&entries) {
                                e.inventory = serialized;
                            }
                        });
                        if let Some(sup_ent) = s.entities.get_mut(&sup_id) {
                            sup_ent.magnesium = sup_ent.magnesium.saturating_add(cost_mg);
                            let mut entries: Vec<crate::db::inventory::InventoryEntry> =
                                serde_json::from_str(&sup_ent.inventory).unwrap_or_default();
                            if let Some(pos) =
                                entries.iter().position(|it| it.item_id == factors.raw_item_id)
                            {
                                if entries[pos].qty > buy_qty {
                                    entries[pos].qty -= buy_qty;
                                } else {
                                    entries.remove(pos);
                                }
                                if let Ok(serialized) = serde_json::to_string(&entries) {
                                    sup_ent.inventory = serialized;
                                }
                            }
                            crate::pg::entity::upsert_async(sup_ent.clone());
                        }
                    }
                    let _ =
                        settle_player_with_settlement_treasury(player_id, &settlement, -cost_mg);
                    let _ =
                        crate::db::add_to_inventory(player_id, &factors.raw_item_id, buy_qty);
                    (
                        "採購原料 備足工坊".to_string(),
                        format!(
                            "於{}向{}支付 {} 枚鎂幣採買 {} {}{}，備足工坊手頭用料。",
                            sup_place,
                            sup_person,
                            cost_mg,
                            buy_qty,
                            factors.raw_item_unit,
                            factors.raw_item_name
                        ),
                        (sup_x, sup_y, sup_place),
                    )
                } else {
                    if let Some(arc) = crate::store::get_store() {
                        let _ = arc.write().update_entity(player_id, |e| {
                            e.current_activity =
                                format!("前往{}採買{}", sup_place, factors.raw_item_name);
                            e.vit = (e.vit - 1).max(20);
                        });
                    }
                    (
                        "動身採買 補給原料".to_string(),
                        format!(
                            "因工坊{}存量告罄，動身前往{}向{}採買原料補足庫存。",
                            factors.raw_item_name, sup_place, sup_id
                        ),
                        (sup_x, sup_y, sup_place),
                    )
                }
            } else if is_real_resident && finished_stock_qty >= 2 {
                let primary_out_id =
                    factors.outputs.first().map(|(id, ..)| id.as_str()).unwrap_or("");
                let buy_opt = if let Some(arc) = crate::store::get_store() {
                    let s = arc.read();
                    crate::pg::living_world::resolve_need_decision_buyer(
                        &s,
                        player_id,
                        cur_pos,
                        primary_out_id,
                        &factors,
                        &pw,
                        &hooks,
                        runtime_grid.as_ref(),
                    )
                } else {
                    None
                };
                let (buyer_id, buyer_place, (buy_x, buy_y)) = buy_opt.unwrap_or_else(|| {
                    let coord = factors.deliver_coord;
                    let place = factors.deliver_bldg.clone();
                    (format!("{}櫃檯", place), place, coord)
                });
                let buyer_person = if buyer_id.ends_with("櫃檯") {
                    buyer_id.clone()
                } else {
                    let n = crate::store::resolve_npc_person_name(&buyer_id, "", None);
                    if n.is_empty() || n.bytes().any(|b| b.is_ascii_alphabetic()) {
                        format!("{}櫃檯", buyer_place)
                    } else {
                        n
                    }
                };

                let on_site = (cur_pos.0 - buy_x).abs() + (cur_pos.1 - buy_y).abs() <= 1;
                if on_site {
                    let payout = factors.delivery_payout_mg;
                    if let Some(arc) = crate::store::get_store() {
                        let mut s = arc.write();
                        let _ = s.update_entity(player_id, |e| {
                            e.current_activity =
                                format!("在{}向{}交割成品，結清貨款", buyer_place, buyer_person);
                            e.magnesium = e.magnesium.saturating_add(payout);
                            e.vit = (e.vit - 3).max(20);
                            let mut entries: Vec<crate::db::inventory::InventoryEntry> =
                                serde_json::from_str(&e.inventory).unwrap_or_default();
                            for (deliv_id, _, _, deliv_qty) in &factors.delivery_items {
                                if let Some(pos) =
                                    entries.iter().position(|it| &it.item_id == deliv_id)
                                {
                                    if entries[pos].qty > *deliv_qty {
                                        entries[pos].qty -= *deliv_qty;
                                    } else {
                                        entries.remove(pos);
                                    }
                                }
                            }
                            if let Ok(serialized) = serde_json::to_string(&entries) {
                                e.inventory = serialized;
                            }
                        });
                        if let Some(buyer_ent) = s.entities.get_mut(&buyer_id) {
                            buyer_ent.magnesium = buyer_ent.magnesium.saturating_sub(payout);
                            let mut entries: Vec<crate::db::inventory::InventoryEntry> =
                                serde_json::from_str(&buyer_ent.inventory).unwrap_or_default();
                            for (deliv_id, _, _, deliv_qty) in &factors.delivery_items {
                                if let Some(entry) =
                                    entries.iter_mut().find(|it| &it.item_id == deliv_id)
                                {
                                    entry.qty += *deliv_qty;
                                } else {
                                    entries.push(crate::db::inventory::InventoryEntry {
                                        item_id: deliv_id.clone(),
                                        qty: *deliv_qty,
                                    });
                                }
                            }
                            if let Ok(serialized) = serde_json::to_string(&entries) {
                                buyer_ent.inventory = serialized;
                            }
                            crate::pg::entity::upsert_async(buyer_ent.clone());
                        }
                    }
                    let _ =
                        settle_player_with_settlement_treasury(player_id, &settlement, payout);
                    for (deliv_id, _, _, deliv_qty) in &factors.delivery_items {
                        let _ = crate::db::add_to_inventory(player_id, deliv_id, -(*deliv_qty));
                    }
                    (
                        "完工交割 兌現貨款".to_string(),
                        format!(
                            "於{}將工坊製作的成品點交予{}，結算收得 {} 枚鎂幣，備料充足、工務順遂。",
                            buyer_place, buyer_person, payout
                        ),
                        (buy_x, buy_y, buyer_place),
                    )
                } else {
                    if let Some(arc) = crate::store::get_store() {
                        let _ = arc.write().update_entity(player_id, |e| {
                            e.current_activity =
                                format!("前往{}送交工坊成品", buyer_place);
                            e.vit = (e.vit - 1).max(20);
                        });
                    }
                    (
                        "整裝出發 送貨交割".to_string(),
                        format!(
                            "背負工坊製作之成品，動身前往{}向{}點交交割。",
                            buyer_place, buyer_person
                        ),
                        (buy_x, buy_y, buyer_place),
                    )
                }
            } else {
                let wage = 4;
                let earned = if let Some(arc) = crate::store::get_store() {
                    let mut s = arc.write();
                    pay_from_settlement_treasury(&mut s, player_id, &settlement, wage)
                } else {
                    0
                };
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(player_id, |e| {
                        e.current_activity = "於工坊推磨校準器物，協助百工".to_string();
                        e.vit = (e.vit - 3).max(20);
                        e.qi = (e.qi - 2).max(20);
                        let mut entries: Vec<crate::db::inventory::InventoryEntry> =
                            serde_json::from_str(&e.inventory).unwrap_or_default();
                        if let Some(pos) = entries
                            .iter()
                            .position(|it| it.item_id == factors.raw_item_id && it.qty >= 1)
                        {
                            entries[pos].qty -= 1;
                            if entries[pos].qty <= 0 {
                                entries.remove(pos);
                            }
                            for (out_id, _, _, out_qty) in &factors.outputs {
                                if let Some(out_entry) =
                                    entries.iter_mut().find(|it| &it.item_id == out_id)
                                {
                                    out_entry.qty += *out_qty;
                                } else {
                                    entries.push(crate::db::inventory::InventoryEntry {
                                        item_id: out_id.clone(),
                                        qty: *out_qty,
                                    });
                                }
                            }
                            if let Ok(serialized) = serde_json::to_string(&entries) {
                                e.inventory = serialized;
                            }
                        }
                    });
                }
                crate::game::object_ecology::supply_settlement_commodity_from_work(
                    &settlement,
                    "craft",
                    &player_name,
                );
                (
                    "駐坊校器 習練百工".to_string(),
                    if earned > 0 {
                        "於工坊協助推磨校準器物，掌事按行規結給工錢並指點發力分寸，手藝與心性皆有精進。".to_string()
                    } else {
                        "於工坊細心觀摩師傅鍛打校準，熟悉各項器物分寸走向，心手相應，大有收穫。".to_string()
                    },
                    (factors.work_coord.0, factors.work_coord.1, factors.work_bldg.clone()),
                )
            }
        }
        OfflineAutonomyDirective::FieldTendAndForage => {
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().update_entity(player_id, |e| {
                    e.current_activity = "沿田壟巡查、採收應季草木".to_string();
                    e.vit = (e.vit - 3).max(20);
                    e.qi = (e.qi - 2).max(20);
                });
            }
            let forage_item = if (now_unix / 60) % 2 == 0 {
                "harvest_herb"
            } else {
                "mat_reed_bundle"
            };
            let cur_qty = crate::db::get_entity(player_id)
                .ok()
                .flatten()
                .map(|e| {
                    crate::db::get_inventory(&e.inventory, e.vit)
                        .items
                        .iter()
                        .find(|it| it.item_id == forage_item)
                        .map(|it| it.qty)
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            if cur_qty < 5 {
                let _ = crate::db::add_to_inventory(player_id, forage_item, 1);
            }
            crate::game::object_ecology::supply_settlement_commodity_from_work(
                &settlement,
                "forage",
                &player_name,
            );
            (
                "田壟巡查 採收草木".to_string(),
                "沿著聚落田壟與近郊林緣慢步巡看，隨手採得應季生鮮草木收入隨身行囊，泥土芬芳怡人。".to_string(),
                (factors.floodgate_coord.0, factors.floodgate_coord.1, factors.floodgate_bldg.clone()),
            )
        }
        OfflineAutonomyDirective::RoadEscortAndPatrol => {
            let (peer_id, peer_name, peer_coord) = if let Some(arc) = crate::store::get_store() {
                let s = arc.read();
                let mut candidates: Vec<(&crate::store::Entity, (i32, i32), i32)> = Vec::new();
                for e in s.entities.values() {
                    if e.kind != "npc" || e.id == player_id {
                        continue;
                    }
                    let coord = match (e.grid_x, e.grid_y) {
                        (Some(x), Some(y)) => (x, y),
                        _ => continue,
                    };
                    let dist = (coord.0 - cur_pos.0).abs() + (coord.1 - cur_pos.1).abs();
                    if dist > 30 {
                        continue;
                    }
                    let is_traveler = e.display_title.contains("商")
                        || e.display_title.contains("獵")
                        || e.display_title.contains("客")
                        || e.display_title.contains("行")
                        || e.display_title.contains("旅")
                        || e.display_title.contains("伴");
                    let (is_staff, is_counter) =
                        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder_entity(
                            e,
                            coord,
                            &factors.raw_item_id,
                        );
                    if is_traveler || is_staff || is_counter {
                        candidates.push((e, coord, dist));
                    }
                }
                candidates.sort_by(|(e1, c1, d1), (e2, c2, d2)| {
                    d1.cmp(d2).then_with(|| c1.cmp(c2)).then_with(|| e1.id.cmp(&e2.id))
                });
                if let Some((npc, coord, _)) = candidates.first() {
                    (npc.id.clone(), npc.display_title.clone(), Some(*coord))
                } else {
                    (
                        format!("escort_{}", settlement),
                        format!("{}往來行旅同伴", settlement),
                        None,
                    )
                }
            } else {
                (
                    format!("escort_{}", settlement),
                    format!("{}往來行旅同伴", settlement),
                    None,
                )
            };
            if let Some(arc) = crate::store::get_store() {
                let act = format!("與【{}】結伴巡行古道，護送行旅", peer_name);
                let _ = arc.write().update_entity(player_id, |e| {
                    e.current_activity = act;
                    e.vit = (e.vit - 3).max(20);
                    e.qi = (e.qi - 2).max(20);
                });
            }
            upsert_relational_hook(
                player_id,
                &peer_id,
                &peer_name,
                "古道同行",
                format!("與【{}】結伴於古道隨行照應，同迎山嵐晨昏", peer_name),
                3,
                now_unix,
            );
            upsert_relational_hook(
                &peer_id,
                player_id,
                &player_name,
                "古道同行",
                format!("承蒙【{}】沿途隨行護持，商道行旅倍感心安", player_name),
                3,
                now_unix,
            );
            let fact = WorldRippleFact {
                fact_id: format!("escort_{}_{}", player_id, now_unix / 300),
                kind: RippleFactKind::PlayerDeed,
                subject_zone_or_settlement: settlement,
                item_or_actor: player_name.clone(),
                order_id: None,
                severity: 0.6,
                origin_tick: now_unix,
                hop_count: 0,
                max_hops: 2,
                prose_summary: format!(
                    "【{}】與【{}】結伴巡行古道，沿途安寧無虞",
                    player_name, peer_name
                ),
            };
            inject_ripple_fact_into_npc(&peer_id, fact, now_unix);
            let (dest_x, dest_y, dest_bldg) = if let Some((px, py)) = peer_coord {
                (px, py, factors.deliver_bldg.clone())
            } else {
                (factors.deliver_coord.0, factors.deliver_coord.1, factors.deliver_bldg.clone())
            };
            (
                "古道相隨 護送行旅".to_string(),
                format!(
                    "與【{}】結伴沿古道漫步同行，彼此照應山路動靜，言談甚歡，互道珍重而別。",
                    peer_name
                ),
                (dest_x, dest_y, dest_bldg),
            )
        }
    };

    let cur_grid = if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        s.entities.get(player_id).and_then(|e| match (e.grid_x, e.grid_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        })
    } else {
        None
    };

    if let Some((cx, cy)) = cur_grid {
        let (target_x, target_y, dest_bldg) = (dest_x, dest_y, dest_bldg);
        if (cx - target_x).abs() + (cy - target_y).abs() > 1 {
            let (nx, ny) = crate::server::grid_manager::get_runtime_square_grid()
                .and_then(|g| crate::store::find_bfs_next_step(&g, (cx, cy), (target_x, target_y)))
                .unwrap_or((cx + (target_x - cx).signum(), cy + (target_y - cy).signum()));
            if let Some(arc) = crate::store::get_store() {
                let mut s = arc.write();
                let mut current_act = String::new();
                let _ = s.update_entity(player_id, |e| {
                    e.grid_x = Some(nx);
                    e.grid_y = Some(ny);
                    e.target_x = if (nx, ny) == (target_x, target_y) { None } else { Some(target_x) };
                    e.target_y = if (nx, ny) == (target_x, target_y) { None } else { Some(target_y) };
                    let place_id = crate::world::place::legacy_place_id(crate::grid::SquareCoord::new(nx, ny));
                    e.place_id = Some(place_id);
                    current_act = e.current_activity.clone();
                });
                let _ = s.move_entity(player_id, &format!("grid:{nx}:{ny}"));
                crate::server::grid_manager::RESIDENT_MOVE_TARGETS.write().insert(
                    player_id.to_string(),
                    (target_x, target_y, dest_bldg, current_act),
                );
            }
        } else {
            crate::server::grid_manager::RESIDENT_MOVE_TARGETS.write().remove(player_id);
        }
    }

    let habit_kind = match directive {
        OfflineAutonomyDirective::InnRestAndHeal => crate::pg::living_world::BreathHabitKind::ChamberClear,
        OfflineAutonomyDirective::WorkshopCraftAndAssist => crate::pg::living_world::BreathHabitKind::ForgeHeavy,
        OfflineAutonomyDirective::FieldTendAndForage => crate::pg::living_world::BreathHabitKind::ForestLong,
        OfflineAutonomyDirective::RoadEscortAndPatrol => crate::pg::living_world::BreathHabitKind::ForestLong,
    };
    let _ = crate::pg::living_world::record_breath_activity(player_id, habit_kind, 2, now_unix);

    if let Some(arc) = crate::store::get_store() {
        let mut s = arc.write();
        s.append_thought(player_id, crate::db::ThoughtSource::WorkDone, body.clone(), now_unix);
    }

    if directive != OfflineAutonomyDirective::InnRestAndHeal {
        let mut tool_ids = Vec::new();
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get(player_id) {
                if let Ok(slots) = serde_json::from_str::<std::collections::HashMap<String, String>>(&e.equipment_slots) {
                    for slot in &["hold_r", "hold_l"] {
                        if let Some(tid) = slots.get(*slot).filter(|id| !id.trim().is_empty()) {
                            tool_ids.push(tid.clone());
                        }
                    }
                }
            }
        }
        if tool_ids.is_empty() {
            if let Ok(Some(ent)) = crate::db::get_entity(player_id) {
                if let Ok(slots) = serde_json::from_str::<std::collections::HashMap<String, String>>(&ent.equipment_slots) {
                    for slot in &["hold_r", "hold_l"] {
                        if let Some(tid) = slots.get(*slot).filter(|id| !id.trim().is_empty()) {
                            tool_ids.push(tid.clone());
                        }
                    }
                }
            }
        }
        for tool_id in tool_ids {
            let is_bladed_tool = !tool_id.contains("0973") && !tool_id.contains("abacus") && !tool_id.contains("basket");
            let vstate = crate::pg::living_world::record_vessel_use(
                player_id,
                &tool_id,
                if is_bladed_tool { 1 } else { 0 },
                1,
                now_unix,
            );
            if is_bladed_tool && vstate.wear_points >= 3 {
                let _ = crate::pg::living_world::sharpen_vessel_at_grindstone(player_id, &tool_id, now_unix);
            }
        }
    }

    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let next_wake = now_unix + 60;
        let _ = client.execute(
            "INSERT INTO resident_continuity (
                entity_id, control_mode, life_status, current_intent,
                intent_reason, destination_place_id, intent_started_at,
                next_wake_at, updated_at, source_kind, source_ref
            ) VALUES (
                $1, 'autonomous', 'alive', $2,
                $3, $4, $5,
                $6, $5, 'offline_autonomy', 'step_offline_player_once'
            )
            ON CONFLICT (entity_id) DO UPDATE SET
                control_mode = 'autonomous',
                life_status = 'alive',
                current_intent = EXCLUDED.current_intent,
                intent_reason = EXCLUDED.intent_reason,
                destination_place_id = EXCLUDED.destination_place_id,
                blocked_by = NULL,
                next_wake_at = EXCLUDED.next_wake_at,
                source_kind = EXCLUDED.source_kind,
                source_ref = EXCLUDED.source_ref,
                updated_at = EXCLUDED.updated_at",
            &[
                &player_id,
                &directive.code(),
                &directive.label(),
                &place_id,
                &now_unix,
                &next_wake,
            ],
        );
    }

    let category = "social".to_string();
    {
        let mut recaps = PLAYER_OFFLINE_RECAPS
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let list = recaps.entry(player_id.to_string()).or_default();
        if let Some(existing) = list.iter_mut().find(|(_, t, _)| t == &title) {
            *existing = (category, title, body.clone());
        } else {
            list.push((category, title, body.clone()));
            if list.len() > 4 {
                list.remove(0);
            }
        }
    }

    Some(body)
}


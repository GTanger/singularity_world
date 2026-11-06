#[allow(clippy::too_many_arguments)]
fn try_run_vendor_interactions_in_room(
    sessions: &SessionStore,
    room_id: &str,
    want: &str,
    now_ts: i64,
    hour: i32,
    active_npcs: &[String],
    npc_snapshots: &HashMap<String, (crate::store::Entity, String)>,
) -> bool {
                // 優先檢查看是否有正在營業擺攤的攤販
                let vendor_npcs: Vec<String> = active_npcs
                    .iter()
                    .filter(|id| {
                        if let Some((e, _)) = npc_snapshots.get(id.as_str()) {
                            let act = e.current_activity.as_str();
                            let is_excluded = act.contains("正在前往")
                                || act.contains("動身前往")
                                || act.contains("返回【")
                                || act.contains("正步行前往")
                                || act.contains("正沿街前往")
                                || act.contains("正推著攤車")
                                || act.contains("已收攤")
                                || act.contains("邁步離開")
                                || act.contains("安歇")
                                || act.contains("熟睡")
                                || act.contains("就寢");
                            if is_excluded {
                                return false;
                            }
                            let is_vending = act.contains('攤')
                                || act.contains("叫賣")
                                || act.contains("吆喝")
                                || act.contains("招呼")
                                || act.contains("楦頭");
                            (id.starts_with("wt_vendor_") || id.as_str() == "wt_npc_ma_buxue") && is_vending
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();

                let get_vendor_bark = |v_id: &str, caller: &str| -> String {
                    let title = npc_snapshots
                        .get(v_id)
                        .map(|(e, _)| e.display_title.as_str())
                        .unwrap_or("");
                    crate::server::action::trade::synthesize_vendor_bark_from_catalog(
                        v_id,
                        title,
                        room_id,
                        caller,
                    )
                };

                if !vendor_npcs.is_empty() {
                    let step = (now_ts as usize) / 4;
                    let v_idx = step % vendor_npcs.len();
                    let mode = step % 3;
                    if mode == 2 && vendor_npcs.len() >= 2 {
                        let id_a = vendor_npcs[v_idx].clone();
                        let id_b = vendor_npcs[(v_idx + 1) % vendor_npcs.len()].clone();
                        if let (Some((ea, _)), Some((eb, _))) = (npc_snapshots.get(id_a.as_str()), npc_snapshots.get(id_b.as_str())) {
                            for s in sessions.all_sessions() {
                                if crate::server::session::player_location_key(&s.player_id) == want {
                                    let caller_a = resolve_caller_name(&s.player_id, &id_a, ea);
                                    let caller_b = resolve_caller_name(&s.player_id, &id_b, eb);
                                    let banter = generate_vendor_banter(&caller_a, &caller_b, &id_a, &id_b, hour);
                                    crate::store::record_active_npc_dialogue(room_id, &id_a, &id_b, &banter);
                                    crate::server::send_npc_dialogue_to_player(&s, &id_a, &id_b, &banter);
                                }
                            }
                            return true;
                        }
                    } else {
                        let v_id = vendor_npcs[v_idx].clone();
                        if let Some((v_ent, _)) = npc_snapshots.get(v_id.as_str()) {
                            let customer_id = active_npcs.iter().find(|id| !vendor_npcs.contains(id)).cloned();
                            let c_ent = customer_id
                                .as_ref()
                                .and_then(|c_id| npc_snapshots.get(c_id.as_str()))
                                .map(|(e, _)| e)
                                .filter(|e| {
                                    !e.current_activity.contains("安歇")
                                        && !e.current_activity.contains("熟睡")
                                        && !e.current_activity.contains("熄燈")
                                        && !e.current_activity.contains("就寢")
                                });
                            if let (Some(c_id), Some(c_ent)) = (customer_id, c_ent) {
                                let purchase_outcome = crate::server::action::trade::settle_live_vendor_npc_purchase(
                                    &v_id,
                                    &c_id,
                                    room_id,
                                    now_ts,
                                );
                                match purchase_outcome {
                                    Some(crate::server::action::trade::VendorNpcPurchaseOutcome::Purchased {
                                        item_name,
                                        price_mg,
                                        remaining_stock: _,
                                        ..
                                    }) => {
                                        for s in sessions.all_sessions() {
                                            if crate::server::session::player_location_key(&s.player_id) == want {
                                                let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                                let c_name = resolve_caller_name(&s.player_id, &c_id, c_ent);
                                                let bark = get_vendor_bark(&v_id, &v_name);
                                                let is_raw_or_craft = [
                                                    "芹菜", "鮮菜", "河蝦", "文蛤", "鮮魚", "青蟹",
                                                    "牡蠣", "乾貨", "魚乾", "粗鹽", "海帶", "紫菜",
                                                    "茶葉", "茶磚", "竹筍", "野蕈", "山藥", "鮮果",
                                                    "皮掌", "鞋底", "摺疊傘", "冰鎮", "仙草凍",
                                                    "涼茶", "冷泡",
                                                ]
                                                .iter()
                                                .any(|kw| item_name.contains(kw));
                                                let pattern_idx = (now_ts as usize + v_id.len()) % 5;
                                                let interaction = if is_raw_or_craft {
                                                    match pattern_idx % 3 {
                                                        0 => format!("{bark} 身旁的{c_name}行色匆匆趕路經過，停下腳步喊道：「東家，幫我快手包一份【{item_name}】帶走！」說著遞上 {price_mg} 鎂。 {v_name}手腳利索包妥遞過，笑道：「好咧收您 {price_mg} 鎂！細心挑揀好的成色最地道，路上慢走啊！」"),
                                                        1 => format!("{bark} 身旁的{c_name}駐足端詳片刻，溫言道：「東家，家裡最認你這攤的【{item_name}】，幫我備一份帶回去！」說罷取出 {price_mg} 鎂付帳。 {v_name}格外細心地包妥，樂呵呵道：「識貨！這批【{item_name}】品質上乘，收您 {price_mg} 鎂，慢走啊！」"),
                                                        _ => format!("{bark} 身旁的{c_name}迎面熟絡地點頭招呼：「頭家，老樣子來一份【{item_name}】！」隨手將 {price_mg} 鎂整齊排在攤前。 {v_name}會心一笑，手腳極快地備齊遞上：「熟客就是內行！早幫您留著好貨呢，收您 {price_mg} 鎂，回頭再來！」"),
                                                    }
                                                } else {
                                                    match pattern_idx {
                                                        0 => format!("{bark} 身旁的{c_name}行色匆匆趕路經過，停下腳步喊道：「東家，手頭趕工時間緊，幫我快手包一份【{item_name}】！」說著遞上 {price_mg} 鎂。 {v_name}手腳極快包好遞過，笑道：「好咧收您 {price_mg} 鎂！路上慢走趁熱吃，幹工順遂啊！」"),
                                                        1 => format!("{bark} 身旁的{c_name}深吸一口氣、聞香靠攏過來笑道：「遠遠就聞到香味，實在忍不住嘴饞！東家，來一份【{item_name}】嚐嚐！」隨即掏出 {price_mg} 鎂遞過去。 {v_name}笑臉盈盈盛妥遞上：「識貨！熱騰騰起鍋的滋味最地道，收您 {price_mg} 鎂，慢用啊！」"),
                                                        2 => format!("{bark} 身旁的{c_name}駐足端詳片刻，溫言道：「東家，家裡長輩最念叨你這手藝，幫我包一份【{item_name}】帶回去給老人家嚐鮮！」說罷取出 {price_mg} 鎂付帳。 {v_name}格外細心地用厚紙包妥，樂呵呵道：「孝順！給長輩帶的特別挑了份量足的，收您 {price_mg} 鎂，替我向老人家問好！」"),
                                                        3 => format!("{bark} 身旁的{c_name}拍了拍腰間沉甸甸的工錢袋，朗聲笑道：「辛苦奔波半天，領了工錢，非得來一份【{item_name}】犒勞犒勞自己不可！」利落地數出 {price_mg} 鎂遞上。 {v_name}連忙接過笑道：「踏實幹活就該吃點好的！收您 {price_mg} 鎂，吃飽喝足渾身都是力氣！」"),
                                                        _ => format!("{bark} 身旁的{c_name}迎面熟絡地點頭招呼：「頭家，老樣子一份【{item_name}】，口味你知道的！」隨手將 {price_mg} 鎂整齊排在攤前。 {v_name}會心一笑，手腳極快地備齊遞上：「熟客就是懂吃！早幫您留心著呢，收您 {price_mg} 鎂，回頭再來坐啊！」"),
                                                    }
                                                };
                                                crate::store::record_active_npc_dialogue(room_id, &v_id, &c_id, &interaction);
                                                crate::server::send_npc_dialogue_to_player(&s, &v_id, &c_id, &interaction);
                                            }
                                        }
                                        return true;
                                    }
                                    Some(crate::server::action::trade::VendorNpcPurchaseOutcome::OutOfStock {
                                        item_name,
                                    }) => {
                                        for s in sessions.all_sessions() {
                                            if crate::server::session::player_location_key(&s.player_id) == want {
                                                let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                                let c_name = resolve_caller_name(&s.player_id, &c_id, c_ent);
                                                let interaction = format!("一旁的{c_name}駐足問道：「東家，想買一份【{item_name}】！」 {v_name}看了一眼見底的鍋具，抱歉笑道：「真不巧，最後一份【{item_name}】賣完啦，我得趕緊去補料，勞駕稍候再來！」");
                                                crate::store::record_active_npc_dialogue(room_id, &v_id, &c_id, &interaction);
                                                crate::server::send_npc_dialogue_to_player(&s, &v_id, &c_id, &interaction);
                                            }
                                        }
                                        return true;
                                    }
                                    Some(crate::server::action::trade::VendorNpcPurchaseOutcome::InsufficientFunds {
                                        item_name,
                                        price_mg,
                                        ..
                                    }) => {
                                        for s in sessions.all_sessions() {
                                            if crate::server::session::player_location_key(&s.player_id) == want {
                                                let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                                let c_name = resolve_caller_name(&s.player_id, &c_id, c_ent);
                                                let bark = get_vendor_bark(&v_id, &v_name);
                                                let interaction = format!("{bark} 一旁的{c_name}聞香駐足，摸了摸錢袋嘆道：「【{item_name}】一份要 {price_mg} 鎂，身上零錢不夠，等領了工錢再來捧場！」");
                                                crate::store::record_active_npc_dialogue(room_id, &v_id, &c_id, &interaction);
                                                crate::server::send_npc_dialogue_to_player(&s, &v_id, &c_id, &interaction);
                                            }
                                        }
                                        return true;
                                    }
                                    None => {
                                        for s in sessions.all_sessions() {
                                            if crate::server::session::player_location_key(&s.player_id) == want {
                                                let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                                let c_name = resolve_caller_name(&s.player_id, &c_id, c_ent);
                                                let bark = get_vendor_bark(&v_id, &v_name);
                                                let idle_idx = (now_ts as usize + v_id.len()) % 2;
                                                let line = match idle_idx {
                                                    0 => format!("{bark} 一旁的{c_name}在攤前停下打量，好奇問道：「東家，今天這料看著挺新鮮，火候正好呢！」 {v_name}笑著抬頭應道：「那是自然，天天現備的好料，您先瞧瞧，待會想吃隨時吩咐！」"),
                                                    _ => format!("{bark} 一旁的{c_name}駐足探頭望著攤車，客氣問道：「頭家，今天生意挺熱絡啊？」 {v_name}擦了擦汗爽朗笑道：「承蒙街坊照顧！您先慢慢看，有合意的儘管開口！」"),
                                                };
                                                crate::store::record_active_npc_dialogue(room_id, &v_id, &c_id, &line);
                                                crate::server::send_npc_dialogue_to_player(&s, &v_id, &c_id, &line);
                                            }
                                        }
                                        return true;
                                    }
                                }
                            } else {
                                for s in sessions.all_sessions() {
                                    if crate::server::session::player_location_key(&s.player_id) == want {
                                        let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                        let bark = get_vendor_bark(&v_id, &v_name);
                                        crate::server::send_narrate_to_player(&s, &bark);
                                    }
                                }
                                return true;
                            }
                        }
                    }
                }

    false
}

pub(crate) fn emit_presence_reflex_on_arrival(conn: &WsConnection, player_id: &str, new_room: &str) {
    let now = crate::game::now_unix();
    {
        let mut cds = PRESENCE_REFLEX_COOLDOWN.write();
        if let Some(&expire) = cds.get(&(player_id.to_string(), new_room.to_string())) {
            if now < expire {
                return;
            }
        }
        cds.insert((player_id.to_string(), new_room.to_string()), now + 8);
    }

    let gh = current_game_hour(&conn.cfg);
    let entities = if let Ok(Some(view)) = game::get_place_room_view(new_room, gh) {
        view.entities
    } else if let Ok(ents) = db::get_entities_in_room(new_room, -1) {
        ents
    } else {
        return;
    };

    let mut wild_creatures = Vec::new();
    let mut npcs = Vec::new();
    for e in entities {
        if e.kind == crate::entity::EntityKind::Npc && e.id != player_id {
            if e.id.starts_with("shanhai_") || e.id.starts_with("ambient_") {
                wild_creatures.push(e);
            } else {
                npcs.push(e);
            }
        }
    }

    if npcs.is_empty() {
        if let Some(creature) = wild_creatures.first() {
            let c_act = creature
                .current_activity
                .trim()
                .trim_start_matches("正在")
                .trim_start_matches('正');
            let line = format!(
                "林隙草木間傳來細碎動靜——不遠處的【{}】正{}，察覺有人踏足此處，警覺地昂首凝視。",
                creature.display_title,
                if c_act.is_empty() { "於棲地間巡行覓食" } else { c_act }
            );
            if creature.id.starts_with("shanhai_") {
                conn.send_json(&NpcAddressMsg {
                    msg_type: "npc_address".into(),
                    npc_id: creature.id.clone(),
                    npc_name: creature.display_title.clone(),
                    text: line,
                    intent: "combat".into(),
                });
            } else {
                conn.send_json(&super::super::protocol::NarrateMsg {
                    msg_type: "narrate".into(),
                    text: line,
                });
            }
        }
        return;
    }
    let get_display_name = |npc: &crate::entity::Character| -> String {
        let is_named = db::get_acquaintance(player_id, &npc.id)
            .ok()
            .flatten()
            .map(|a| a.named)
            .unwrap_or(false);
        if is_named {
            crate::store::resolve_npc_person_name(&npc.id, &npc.display_title, None)
        } else {
            let tag = crate::store::npc_brief_appearance_tag(&npc.id, &npc.display_title);
            let in_hunting_party = crate::store::LIVE_HUNTING_PARTIES
                .read()
                .values()
                .any(|p| p.member_ids.contains(&npc.id));
            let role = if in_hunting_party || tag.contains("合社獵團") {
                "獵手"
            } else if tag.contains("入山百工") {
                "山林百工"
            } else if tag.contains("古道行旅") {
                "行旅護商"
            } else if npc.display_title.contains("鐵") || npc.display_title.contains("匠") {
                "匠人"
            } else if npc.display_title.contains("商") || npc.display_title.contains("販") {
                "攤販"
            } else if npc.display_title.contains("醫") || npc.display_title.contains("藥") {
                "醫工"
            } else if npc.display_title.contains("學") || npc.display_title.contains("書") {
                "學者"
            } else {
                "街坊"
            };
            if !tag.is_empty() {
                if matches!(tag.as_str(), "合社獵團" | "入山百工" | "古道行旅") {
                    format!("{tag}的{role}")
                } else {
                    format!("一名{tag}的{role}")
                }
            } else {
                format!("一名{role}")
            }
        }
    };
    let awake_npcs: Vec<_> = npcs
        .iter()
        .filter(|e| !e.current_activity.contains("安歇") && !e.current_activity.contains("熟睡") && !e.current_activity.contains("熄燈") && !e.current_activity.contains("就寢"))
        .cloned()
        .collect();

    // 高優先因果存在感反射（Causal Presence Reflexes）
    let player_name = db::get_entity(player_id)
        .ok()
        .flatten()
        .map(|e| e.display_title)
        .unwrap_or_default();

    // Priority 1: 直接針對玩家的人情與恩怨交感
    for npc in &awake_npcs {
        if let Some(hook) = crate::npc::causal_core::find_relational_hook_for_target(&npc.id, player_id, &player_name) {
            if hook.intensity >= 3 {
                let pair_key = (player_id.to_string(), npc.id.clone());
                let should_emit = {
                    let cds = PAIRWISE_REFLEX_COOLDOWN.read();
                    match cds.get(&pair_key) {
                        Some((expire, last_reason)) => now >= *expire || last_reason != &hook.kind_label,
                        None => true,
                    }
                };
                if should_emit {
                    PAIRWISE_REFLEX_COOLDOWN.write().insert(pair_key, (now + 180, hook.kind_label.clone()));
                    let caller = get_display_name(npc);
                    let (reflex, intent) = match hook.kind_label.as_str() {
                        "山道解圍" | "救命之恩" | "合社並肩" | "守諾歸還" | "大宗交割" => (
                            format!("{caller}見你邁步走入，神色間帶著感念之意，上前抱拳致意：「別來無恙！因『{}』之誼，在下銘記於心，今日能在此處與足下相逢，甚是欣慰。」", hook.kind_label),
                            "talk",
                        ),
                        "結下樑子" | "交鋒結怨" | "毀器芥蒂" => (
                            format!("{caller}抬眼瞥見你走入，神色微沉，下意識按向身側戒備，冷冷道：「你我因『{}』結下的過節未解，今日至此，又有何貴幹？」", hook.kind_label),
                            "combat",
                        ),
                        "切磋落敗" => (
                            format!("{caller}見你踏入，肅然正色，抱拳沉聲道：「足下身手精湛，上次切磋受教良多，武道深邃，在下佩服。」"),
                            "talk",
                        ),
                        _ => (
                            format!("{caller}望向你時目光微動，主動點頭示意：「因『{}』之故，在下記得足下，別來無恙。」", hook.kind_label),
                            "talk",
                        ),
                    };
                    conn.send_json(&NpcAddressMsg {
                        msg_type: "npc_address".into(),
                        npc_id: npc.id.clone(),
                        npc_name: caller,
                        text: reflex,
                        intent: intent.into(),
                    });
                    return;
                }
            }
        }
    }

    let room_display_name =
        crate::db::get_room_name(new_room).unwrap_or_else(|_| new_room.to_string());

    // Priority 2: 滯留貨物託運或地方緊急見聞
    for npc in &awake_npcs {
        if let Some(order) = crate::npc::causal_core::active_stranded_cargo_for_carrier_or_room(
            &npc.id,
            &room_display_name,
            now,
        ) {
            let pair_key = (player_id.to_string(), npc.id.clone());
            let reason_key = format!("stranded_cargo:{}", order.order_id);
            let should_emit = {
                let cds = PAIRWISE_REFLEX_COOLDOWN.read();
                match cds.get(&pair_key) {
                    Some((expire, last_reason)) => now >= *expire || last_reason != &reason_key,
                    None => true,
                }
            };
            if should_emit {
                PAIRWISE_REFLEX_COOLDOWN.write().insert(pair_key, (now + 180, reason_key));
                let caller = get_display_name(npc);
                let reflex = format!(
                    "{}守著行囊貨擔正自發愁，見你踏入連忙上前拱手道：「客友留步！我有一批要發往【{}】的『{}』因{}受阻於此，公庫已備妥 {} 枚鎂幣賞格，不知可願搭手幫忙護送過關？」",
                    caller, order.dest_settlement, order.item_name, order.hazard_prose, order.escrow_bounty_mg
                );
                conn.send_json(&NpcAddressMsg {
                    msg_type: "npc_address".into(),
                    npc_id: npc.id.clone(),
                    npc_name: caller,
                    text: reflex,
                    intent: "errand".into(),
                });
                return;
            }
        }

        if crate::store::player_knows_npc(player_id, &npc.id) {
            if let Some(rf) = crate::npc::causal_core::top_active_ripple_for_npc(&npc.id, now) {
                if rf.hop_count <= 2 {
                    let pair_key = (player_id.to_string(), npc.id.clone());
                    let reason_key = format!("ripple:{}", rf.fact_id);
                    let should_emit = {
                        let cds = PAIRWISE_REFLEX_COOLDOWN.read();
                        match cds.get(&pair_key) {
                            Some((expire, last_reason)) => now >= *expire || last_reason != &reason_key,
                            None => true,
                        }
                    };
                    if should_emit {
                        PAIRWISE_REFLEX_COOLDOWN.write().insert(pair_key, (now + 180, reason_key));
                        let caller = get_display_name(npc);
                        let ending = if rf.prose_summary.contains("重新暢通") || rf.prose_summary.contains("額手稱慶") {
                            "這幾日往來行旅總算能鬆一口氣了。"
                        } else {
                            "你出入山道市集可得多留個心眼。"
                        };
                        let reflex = format!(
                            "{}見你行色匆匆踏入，壓低嗓音朝你提點道：「前刻外頭傳來的風聲——{}，{}」",
                            caller, rf.prose_summary, ending
                        );
                        conn.send_json(&NpcAddressMsg {
                            msg_type: "npc_address".into(),
                            npc_id: npc.id.clone(),
                            npc_name: caller,
                            text: reflex,
                            intent: "talk".into(),
                        });
                        return;
                    }
                }
            }
        }
    }

    // 0. 玩家在場反應（Presence Reflex）優先判定
    // (a) 若在場有熟識的 NPC，主動相認或託付差事
    if let Some(known_npc) = awake_npcs.iter().find(|n| crate::store::player_knows_npc(player_id, &n.id)) {
        let name = crate::store::resolve_npc_person_name(&known_npc.id, &known_npc.display_title, None);
        let active_ob = crate::store::get_active_npc_obligation(&known_npc.id).filter(|(k, _)| {
            matches!(k.as_str(), "restock_goods" | "repair_tool" | "treat_injury" | "repay_debt" | "stranded_cargo")
        });
        if let Some((kind, subject_ref)) = active_ob {
            let desc = if !subject_ref.is_empty() {
                subject_ref
            } else {
                match kind.as_str() {
                    "restock_goods" => "鋪內現貨見底急需提貨補倉".to_string(),
                    "repair_tool" => "手頭工具耗損待修".to_string(),
                    "treat_injury" => "連日勞作氣息微滯需調理".to_string(),
                    "repay_debt" => "貨單票據核銷結算".to_string(),
                    _ => "手頭未了的差事".to_string(),
                }
            };
            let reflex = format!("{name}見你踏入此處，上前拱手低聲道：「近來正為【{desc}】發愁，不知可否請你搭把手？」");
            conn.send_json(&NpcAddressMsg {
                msg_type: "npc_address".into(),
                npc_id: known_npc.id.clone(),
                npc_name: name,
                text: reflex,
                intent: "errand".into(),
            });
            return;
        }
        let reflex = match (crate::game::now_unix() as usize + known_npc.id.len()) % 3 {
            0 => format!("{name}見你邁步走入，神色微動，溫和點頭致意：「別來無恙，先前還在思忖何時能與你會面。」"),
            1 => format!("{name}停下手頭的事務，抬眼望見你，含笑招呼：「旅途可還順遂？若有事需要打點，隨時知會一聲。」"),
            _ => format!("{name}朝你微微頷首：「來得正好，四方市面平穩，請入內歇步。」"),
        };
        conn.send_json(&NpcAddressMsg {
            msg_type: "npc_address".into(),
            npc_id: known_npc.id.clone(),
            npc_name: name,
            text: reflex,
            intent: "talk".into(),
        });
        return;
    }

    // (a2) 若在場有「合社獵團（`LIVE_HUNTING_PARTIES`）」成員，主動通報圍獵進度並號召同行分鹿
    let hunting_reflex = {
        let parties = crate::store::LIVE_HUNTING_PARTIES.read();
        awake_npcs.iter().find_map(|n| {
            parties
                .values()
                .find(|p| p.member_ids.contains(&n.id))
                .map(|p| (n, p.clone()))
        })
    };
    if let Some((hunter_npc, party)) = hunting_reflex {
        let caller = get_display_name(hunter_npc);
        let (px, py) = party.prey_coord;
        let prey_place = crate::game::CANONICAL_HUNTING_ZONES
            .get(crate::game::nearest_hunting_zone_idx(px, py))
            .map(|z| z.0.to_string())
            .unwrap_or_else(|| {
                let (z, s, _) = crate::grid::r#gen::classify_wilderness_cell(px, py);
                format!("{z}・{s}")
            });
        let reflex = match party.phase {
            crate::store::HuntingPartyPhase::Rallying => format!(
                "{caller}手按腰間山豬牙號角，見你踏入此處，精神一振低聲招呼：「來得正好！領隊【{}】正召集大夥（現已聚齊 {} 人）準備前往【{prey_place}】一帶合圍【{}】，你若願一同助陣分鹿，儘管與我交談（Talk）入團！」",
                party.leader_name,
                party.member_ids.len(),
                party.prey_name
            ),
            crate::store::HuntingPartyPhase::SurroundingPrey => format!(
                "{caller}壓低身形、緊盯【{prey_place}】一帶方向的【{}】，見你靠近連忙打出手勢低喝：「別驚動獵物！我們【{}】合社獵團正從八方收攏包圍圈，隨同出手合擊者見者有份！」",
                party.prey_name, party.leader_name
            ),
            crate::store::HuntingPartyPhase::TakedownAndSplit => format!(
                "{caller}正與【{}】合社獵團夥伴全力圍殺【{}】、刀光與號角齊鳴，見你趕到高呼道：「快搭把手合擊此獸，剝取的【{}】與賞格見者有份！」",
                party.leader_name, party.prey_name, party.prey_loot_name
            ),
            crate::store::HuntingPartyPhase::ReturningToMarket => format!(
                "{caller}肩扛圍獵所獲的【{}】，隨【{}】合社獵團結伴趕往【{}】交割分成，見你迎面走來爽朗笑道：「這趟合圍【{}】大獲全勝，正要進鎮過磅分鎂幣呢！」",
                party.prey_loot_name, party.leader_name, party.market_name, party.prey_name
            ),
        };
        conn.send_json(&NpcAddressMsg {
            msg_type: "npc_address".into(),
            npc_id: hunter_npc.id.clone(),
            npc_name: caller,
            text: reflex,
            intent: "hunt".into(),
        });
        return;
    }

    // (b) 工匠對攜帶原料之行旅者的反應（僅限工坊環境）
    let is_workshop_room = room_display_name.contains("坊") || room_display_name.contains("打鐵") || room_display_name.contains("木作") || room_display_name.contains("印行") || room_display_name.contains("窯");
    let artisan_npc = if is_workshop_room {
        awake_npcs.iter().find(|n| {
            !n.id.starts_with("wt_vendor_") && (n.id.contains("cen_") || n.id.contains("ke_") || n.id.contains("ma_buxue") || n.id.contains("smith") || n.id.contains("carpenter"))
        })
    } else {
        None
    };
    if let Some(artisan) = artisan_npc {
        let caller = get_display_name(artisan);
        let reflex = format!("{caller}見你步入坊內，擦了擦手主動招呼道：「客友請進，案頭爐火正旺，隨身若有金石草木良材或器物待修整鍛打，皆可就爐操持！」");
        conn.send_json(&NpcAddressMsg {
            msg_type: "npc_address".into(),
            npc_id: artisan.id.clone(),
            npc_name: caller,
            text: reflex,
            intent: "craft".into(),
        });
        return;
    }


    // 1. 優先檢查是否有正在擺攤的攤販業者
    let vendor_npc = npcs.iter().find(|e| {
        (e.id.starts_with("wt_vendor_") || e.display_title.contains("攤") || e.display_title.contains("補靴") || e.display_title.contains("修傘"))
            && (e.current_activity.contains("擺攤")
                || e.current_activity.contains("攤位")
                || e.current_activity.contains("叫賣")
                || e.current_activity.contains("修補攤")
                || e.current_activity.contains("掌勺")
                || e.current_activity.contains("招呼"))
    });

    if let Some(v) = vendor_npc {
        let caller = get_display_name(v);
        let bark = crate::server::action::trade::synthesize_vendor_bark_from_catalog(
            &v.id,
            &v.display_title,
            new_room,
            &caller,
        );

        let other_customer = npcs.iter().find(|e| {
            e.id != v.id
                && !e.id.starts_with("wt_vendor_")
                && !e.current_activity.contains("安歇")
                && !e.current_activity.contains("熟睡")
                && !e.current_activity.contains("熄燈")
                && !e.current_activity.contains("就寢")
        });
        let line = if let Some(cust) = other_customer {
            let is_cust_named = db::get_acquaintance(player_id, &cust.id)
                .ok()
                .flatten()
                .map(|a| a.named)
                .unwrap_or(false);
            let cust_name = if is_cust_named {
                crate::store::resolve_npc_person_name(&cust.id, &cust.display_title, None)
            } else {
                let tag = crate::store::npc_brief_appearance_tag(&cust.id, &cust.display_title);
                let clean_tag = tag.strip_prefix("一名").unwrap_or(&tag);
                format!("一名{}的客人", clean_tag)
            };
            let v_name = &caller;
            let c_name = &cust_name;
            let now_ts = crate::game::now_unix();
            match crate::server::action::trade::settle_live_vendor_npc_purchase(
                &v.id,
                &cust.id,
                new_room,
                now_ts,
            ) {
                Some(crate::server::action::trade::VendorNpcPurchaseOutcome::Purchased {
                    item_name,
                    price_mg,
                    remaining_stock: _,
                    ..
                }) => {
                    let is_raw_or_craft = [
                        "芹菜", "鮮菜", "河蝦", "文蛤", "鮮魚", "青蟹", "牡蠣", "乾貨", "魚乾",
                        "粗鹽", "海帶", "紫菜", "茶葉", "茶磚", "竹筍", "野蕈", "山藥", "鮮果",
                        "皮掌", "鞋底", "摺疊傘", "冰鎮", "仙草凍", "涼茶", "冷泡",
                    ]
                    .iter()
                    .any(|kw| item_name.contains(kw));
                    let pattern_idx = (now_ts as usize + v.id.len()) % 5;
                    if is_raw_or_craft {
                        match pattern_idx % 3 {
                            0 => format!("{bark} 身旁的{c_name}行色匆匆趕路經過，停下腳步喊道：「東家，幫我快手包一份【{item_name}】帶走！」說著遞上 {price_mg} 鎂。 {v_name}手腳利索包妥遞過，笑道：「好咧收您 {price_mg} 鎂！細心挑揀好的成色最地道，路上慢走啊！」"),
                            1 => format!("{bark} 身旁的{c_name}駐足端詳片刻，溫言道：「東家，家裡最認你這攤的【{item_name}】，幫我備一份帶回去！」說罷取出 {price_mg} 鎂付帳。 {v_name}格外細心地包妥，樂呵呵道：「識貨！這批【{item_name}】品質上乘，收您 {price_mg} 鎂，慢走啊！」"),
                            _ => format!("{bark} 身旁的{c_name}迎面熟絡地點頭招呼：「頭家，老樣子來一份【{item_name}】！」隨手將 {price_mg} 鎂整齊排在攤前。 {v_name}會心一笑，手腳極快地備齊遞上：「熟客就是內行！早幫您留著好貨呢，收您 {price_mg} 鎂，回頭再來！」"),
                        }
                    } else {
                        match pattern_idx {
                            0 => format!("{bark} 身旁的{c_name}行色匆匆趕路經過，停下腳步喊道：「東家，手頭趕工時間緊，幫我快手包一份【{item_name}】！」說著遞上 {price_mg} 鎂。 {v_name}手腳極快包好遞過，笑道：「好咧收您 {price_mg} 鎂！路上慢走趁熱吃，幹工順遂啊！」"),
                            1 => format!("{bark} 身旁的{c_name}深吸一口氣、聞香靠攏過來笑道：「遠遠就聞到香味，實在忍不住嘴饞！東家，來一份【{item_name}】嚐嚐！」笑著掏出 {price_mg} 鎂遞過去。 {v_name}笑臉盈盈盛妥遞上：「識貨！熱騰騰起鍋的滋味最地道，收您 {price_mg} 鎂，慢用啊！」"),
                            2 => format!("{bark} 身旁的{c_name}駐足端詳片刻，溫言道：「東家，家裡長輩最掛念你這手藝，幫我包一份【{item_name}】帶回去給老人家嚐鮮！」說罷取出 {price_mg} 鎂付帳。 {v_name}格外細心地用厚紙包妥，樂呵呵道：「孝順！給長輩帶的特別挑了份量足的，收您 {price_mg} 鎂，替我向老人家問好！」"),
                            3 => format!("{bark} 身旁的{c_name}拍了拍腰間的工錢袋，朗聲笑道：「辛苦奔波半天，領了工錢，非得來一份【{item_name}】犒勞犒勞自己不可！」利落地數出 {price_mg} 鎂遞上。 {v_name}連忙接過笑道：「踏實做工就該吃點好的！收您 {price_mg} 鎂，吃飽喝足渾身都是力氣！」"),
                            _ => format!("{bark} 身旁的{c_name}迎面熟絡地點頭招呼：「頭家，老樣子一份【{item_name}】，口味你知道的！」隨手將 {price_mg} 鎂整齊排在攤前。 {v_name}會心一笑，手腳極快地備齊遞上：「熟客就是懂吃！早幫您留心著呢，收您 {price_mg} 鎂，回頭再來坐啊！」"),
                        }
                    }
                }
                Some(crate::server::action::trade::VendorNpcPurchaseOutcome::OutOfStock {
                    item_name,
                }) => {
                    format!("一旁的{c_name}駐足問道：「東家，想買一份【{item_name}】！」 {v_name}看了一眼見底的鍋具，抱歉笑道：「真不巧，最後一份【{item_name}】賣完啦，我得趕緊去補料，勞駕稍候再來！」")
                }
                Some(crate::server::action::trade::VendorNpcPurchaseOutcome::InsufficientFunds {
                    item_name,
                    price_mg,
                    ..
                }) => {
                    format!("{bark} 一旁的{c_name}聞香駐足，摸了摸錢袋嘆道：「【{item_name}】一份要 {price_mg} 鎂，身上零錢不夠，等領了工錢再來捧場！」")
                }
                None => {
                    let idle_idx = (now_ts as usize + v.id.len()) % 3;
                    match idle_idx {
                        0 => bark,
                        1 => format!("{bark} 一旁的{c_name}在攤前停下打量，好奇問道：「東家，今天這料看著挺新鮮，火候正好呢！」 {v_name}笑著抬頭應道：「那是自然，天天現備的好料，您先瞧瞧，待會想吃隨時吩咐！」"),
                        _ => format!("{bark} 一旁的{c_name}駐足探頭望著攤車，客氣問道：「頭家，今天生意挺熱絡啊？」 {v_name}擦了擦汗爽朗笑道：「承蒙街坊照顧！您先慢慢看，有合意的儘管開口！」"),
                    }
                }
            }
        } else {
            bark
        };
        if let Some(cust) = other_customer {
            crate::store::record_active_npc_dialogue(new_room, &v.id, &cust.id, &line);
            conn.send_json(&NpcDialogueMsg {
                msg_type: "npc_dialogue".into(),
                npc_a: v.id.clone(),
                npc_b: cust.id.clone(),
                text: line,
            });
        } else {
            conn.send_json(&NpcAddressMsg {
                msg_type: "npc_address".into(),
                npc_id: v.id.clone(),
                npc_name: caller,
                text: line,
                intent: "trade".into(),
            });
        }
        return;
    }

    // 2. 若同格無攤販、但有 >= 2 名清醒 NPC 在場

    if awake_npcs.len() >= 2 {
        let a = &awake_npcs[0];
        let b = &awake_npcs[1];

        let resolve_person = |npc: &crate::entity::Character| -> String {
            let is_named = db::get_acquaintance(player_id, &npc.id)
                .ok()
                .flatten()
                .map(|a| a.named)
                .unwrap_or(false);
            if is_named {
                crate::store::resolve_npc_person_name(&npc.id, &npc.display_title, None)
            } else {
                let tag = crate::store::npc_brief_appearance_tag(&npc.id, &npc.display_title);
                format!("那名{}的街坊", tag)
            }
        };

        let name_a = resolve_person(a);
        let name_b = resolve_person(b);
        let room_resolved = crate::db::get_room_name(new_room).unwrap_or_else(|_| new_room.to_string());
        let room = room_resolved.as_str();
        static ENTRY_CHAT_SEQ: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        let dialogues: &[(&str, &str)] = if room.contains("橋") || room.contains("街") || room.contains("巷") || room.contains("道") {
            &[
                ("「自三水口過來，看著河道水位退得乾淨，這兩天船運順得很。」", "「可不是嘛，連帶這兩天的乾貨瓜果也新鮮便宜不少。」"),
                ("「前頭熟食攤又擺起炭烤跟切仔麵了，那香味隔了半條街都聞得到！」", "「稍後辦完手頭這趟差事，正想過去叫碗熱湯暖暖身子呢。」"),
                ("「聽說西門老匠那邊新熬了一批防雨桐油，拿去抹靴子正耐走。」", "「下回得把家裡那把竹骨傘也順便送去給他過過油。」"),
            ]
        } else if room.contains("食堂") || room.contains("茶屋") || room.contains("酒") {
            &[
                ("「這家大骨高湯熬得真醇厚，油蔥酥撒下去那叫一個提味！」", "「再切一份黑白切嘴邊肉，配著這碗熱麵才算過癮。」"),
                ("「聽說南街這幾日來了不少外島客商，茶桌都快坐不下了。」", "「大家都是衝著這幾天風平浪靜、趕在開洋前來談生意的。」"),
            ]
        } else if room.contains("商行") || room.contains("帳房") || room.contains("商算") || room.contains("會計") {
            &[
                ("「這筆東灣進口的苧麻粗布帳目核算過兩遍，總數對上了。」", "「妥當！把籤條壓在庫單上，下午就能發往分家巷作坊了。」"),
                ("「這批烏木與精鋼的公道貨價冊子，今天行商們反應如何？」", "「都照著公議堂紅印市價走，四方貨客沒一個有異議的。」"),
            ]
        } else if room.contains("蒙學") || room.contains("學院") || room.contains("書局") || room.contains("書舍") || room.contains("講堂") || room.contains("藏書") {
            &[
                ("「先前先生講的氣象潮汐篇，那段測風速的法子真精妙。」", "「稍後去借閱室把《半島風物圖誌》找出來，核對一下就全通了。」"),
                ("「這批新到的竹綿紙墨色吃得極好，抄寫筆札不化不透。」", "「我也挑了兩刀，打算謄寫先生留下來的天文算曆課卷。」"),
            ]
        } else if room.contains("鐵坊") || room.contains("鍛造") || room.contains("木作") || room.contains("工坊") {
            &[
                ("「這爐耐火土吸熱勻實，這批精鋼農具淬火出來韌性正好！」", "「再敲兩遍冷縮槽裡的卡榫，保證耕石地三年都不捲刃。」"),
                ("「大水車排鋸切出來的雲杉原木平整得很，刨起來省了大半工夫。」", "「留著給西門車馬行做新大車轅木，結實又吃得住重載。」"),
            ]
        } else if room.contains("浴場") || room.contains("泉") {
            &[
                ("「這寒水玉溫石槽引的活泉水，泡上一刻鐘全身骨頭都舒暢了。」", "「勞作一整天，就盼著這一瓢溫熱泉水把寒氣都洗透。」"),
                ("「裡頭松炭烘衣櫥真方便，洗完澡衣裳早已暖烘烘的了。」", "「可不是嘛，出門吹夜風也不怕著涼！」"),
            ]
        } else if room.contains("診所") || room.contains("藥房") || room.contains("烘藥") {
            &[
                ("「這批百草細火焙過，止血水苔的藥性收斂得恰到好處。」", "「分包切片整齊些，巡坊隊跟出海船夫常備著救急呢。」"),
                ("「換季時節夜裡露水重，這幾天來抓避瘴清心草藥的人不少。」", "「慢火溫湯小銅爐一直沒熄火，隨時備著熱藥湯給街坊暖胃。」"),
            ]
        } else {
            &[
                ("「今天手頭這趟活計忙完，可得好好去南街市集逛逛。」", "「聽說今晚各家攤子都熱鬧，走走散步正好吹吹夜風。」"),
                ("「街坊近來大家都在聊那趟新開的貨運班次呢。」", "「路修得平整，往來半嶺跟三水口確實比早先省了一半腳程。」"),
            ]
        };

        let pick_idx = ENTRY_CHAT_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % dialogues.len();
        let (dlg_a, dlg_b) = dialogues[pick_idx];
        let line = match pick_idx % 6 {
            0 => format!("{name_a}停下手頭動作，側身對{name_b}言道：{dlg_a}\n{name_b}微微點頭，隨口應道：{dlg_b}"),
            1 => format!("{name_a}整理著手邊物事，向身側的{name_b}開口：{dlg_a}\n{name_b}抬眼笑應：{dlg_b}"),
            2 => format!("{name_a}望向四下街景，同{name_b}搭話：{dlg_a}\n{name_b}順著話頭接道：{dlg_b}"),
            3 => format!("{name_a}朝{name_b}低聲說道：{dlg_a}\n{name_b}神情專注，答道：{dlg_b}"),
            4 => format!("{name_a}手上活計未停，對{name_b}言道：{dlg_a}\n{name_b}語調平穩地回道：{dlg_b}"),
            _ => format!("{name_a}向{name_b}問起：{dlg_a}\n{name_b}略一思索，溫聲回道：{dlg_b}"),
        };

        crate::store::record_active_npc_dialogue(new_room, &a.id, &b.id, &line);
        conn.send_json(&NpcDialogueMsg {
            msg_type: "npc_dialogue".into(),
            npc_a: a.id.clone(),
            npc_b: b.id.clone(),
            text: line,
        });
        return;
    }

    // 3. 單一 NPC 舊有差事／打量 fallback
    if let Some(npc) = awake_npcs.first().or_else(|| npcs.first()) {
        let is_known = crate::store::player_knows_npc(player_id, &npc.id);
        let npc_name = get_display_name(npc);
        let active_ob = if is_known {
            crate::store::get_active_npc_obligation(&npc.id)
        } else {
            None
        };
        let personality = crate::db::get_personality_for_entity(&npc.id)
            .or_else(|| npc.soul_seed.map(crate::db::expand_soul_seed_to_personality))
            .unwrap_or(crate::db::Personality {
                boldness: 0.5,
                sensitivity: 0.5,
                orderliness: 0.5,
            });

        let ob_line = active_ob.filter(|(k, _)| {
            matches!(
                k.as_str(),
                "restock_goods" | "repair_tool" | "treat_injury" | "repay_debt" | "stranded_cargo"
            )
        }).map(|(kind, subject_ref)| {
            let desc = if !subject_ref.is_empty() {
                subject_ref
            } else {
                match kind.as_str() {
                    "restock_goods" => "鋪內現貨見底急需提貨補倉".to_string(),
                    "repair_tool" => "手頭工具耗損待修".to_string(),
                    "treat_injury" => "連日勞作氣息微滯需調理".to_string(),
                    "repay_debt" => "貨單票據核銷結算".to_string(),
                    _ => "手頭未了的差事".to_string(),
                }
            };
            format!("{npc_name}見你踏入此處，上前拱手低聲道：「近來正為【{desc}】發愁，不知可否請你搭把手？」")
        });

        if let Some(line) = ob_line {
            conn.send_json(&NpcAddressMsg {
                msg_type: "npc_address".into(),
                npc_id: npc.id.clone(),
                npc_name: npc_name,
                text: line,
                intent: "errand".into(),
            });
        } else {
            let line = if personality.boldness >= 0.65 {
                format!("{npc_name}聽見腳步聲，抬眼向你投來友善的招呼目光。")
            } else {
                format!("{npc_name}抬眼見你走近，微笑著朝你點了點頭。")
            };
            conn.send_json(&super::super::protocol::NarrateMsg {
                msg_type: "narrate".into(),
                text: line,
            });
        }
        return;
    }
}

    fn can_step_in_nine_grid(
        g: &crate::grid::SquareGrid,
        from: SquareCoord,
        to: SquareCoord,
    ) -> bool {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        if dx.abs() + dy.abs() != 1 {
            return false;
        }
        g.can_walk(from, to)
    }
/// 依人物的正式 place_id 重推玩家視野。

fn current_l3_scene_climate() -> Option<(String, i64)> {
    db::get_region_climate("default")
        .ok()
        .flatten()
        .map(|c| (c.event_type, c.prose_seed))
}


fn north_bank_phenomenon(game_hour: u32, climate_key: &str, seed: i64) -> &'static str {
    match climate_key {
        "rain" | "plum_rain" | "storm" => pick_scene_variant(
            &[
                "雨後的岸泥被踩得發亮，河霧貼著水面往踏板下鑽。",
                "濕氣從木板縫裡返上來，船繩和價牌都帶著一層潮意。",
            ],
            seed,
        ),
        "fog" => pick_scene_variant(
            &[
                "河霧沿水面壓上北岸，幾步外的人影只剩輪廓。",
                "霧把對岸收得很遠，連船腹拍水聲都像隔了一層布。",
            ],
            seed,
        ),
        "wind" => pick_scene_variant(
            &[
                "河風捲起濕氣，吹得價牌和船繩一齊輕響。",
                "風從河面推過來，市集棚布低低鼓了一下又垂回去。",
            ],
            seed,
        ),
        "snow" => pick_scene_variant(
            &[
                "冷氣壓在河面上，踏板邊緣泛著薄白。",
                "河霧混著寒意貼住木樁，等船人的袖口都收得更緊。",
            ],
            seed,
        ),
        _ => match scene_phase(game_hour) {
            ScenePhase::Dawn => pick_scene_variant(
                &[
                    "晨霧從水面慢慢抬起，先露出船繩，再露出踏板前的人影。",
                    "天光還薄，河霧貼著北岸不肯散，濕木的氣味比人聲先到。",
                ],
                seed,
            ),
            ScenePhase::Day => pick_scene_variant(
                &[
                    "日光落在河面上，霧氣退到水邊，只剩木板下方還濕亮。",
                    "河面被日色照開，踏板和市集之間的人影變得清楚。",
                ],
                seed,
            ),
            ScenePhase::Dusk => pick_scene_variant(
                &[
                    "暮色壓低水面，河霧又從踏板底下漫回來。",
                    "天色往暗處沉，市集的價牌先失去顏色，河面還反著一點光。",
                ],
                seed,
            ),
            ScenePhase::Night => pick_scene_variant(
                &[
                    "夜水反著微光，河霧把渡船邊緣抹得不太分明。",
                    "夜風貼著水面走，濕木踏板在暗處泛出一線冷光。",
                ],
                seed,
            ),
        },
    }
}

fn north_bank_presence(unnamed_total: usize, seed: i64) -> &'static str {
    if unnamed_total >= 12 {
        pick_scene_variant(
            &[
                "候渡的人群壓住岸邊空地，貨聲、問價聲和等船的低語混在一起。",
                "市集邊人影密了些，包袱和貨擔把通向踏板的空隙擠窄。",
            ],
            seed,
        )
    } else if unnamed_total >= 5 {
        pick_scene_variant(
            &[
                "幾撥等船的人散在茶桶和踏板之間，不時有人朝河面張望。",
                "有人守著包袱歇腳，有人往市集那頭問價，視線仍留著渡船方向。",
            ],
            seed,
        )
    } else if unnamed_total > 0 {
        pick_scene_variant(
            &[
                "零散的候渡者站在河風裡，說話聲比水聲低。",
                "岸邊只聚著少數等船的人，踏板前的腳步聲顯得更清楚。",
            ],
            seed,
        )
    } else {
        "人聲暫時退開，只剩水聲和濕木在近處。"
    }
}

fn north_bank_object_surface(seed: i64) -> &'static str {
    pick_scene_variant(
        &[
            "長凳旁的蘆叢和貨擔都貼著踏板邊等人伸手，熱湯攤分出滿碗與等分湯，人龍木牌把下一輪上船的人壓成一線。",
            "歇腳處留著半段乾位，湯桶熱氣貼著河霧上浮，船繩旁的貨擔把路口擠窄；谷地路口的泥印從人群後面斜出去。",
        ],
        seed,
    )
}

fn north_bank_ferry_presence(ferry: FerryPhase, seed: i64) -> &'static str {
    match ferry {
        FerryPhase::NorthDocked => pick_scene_variant(
            &[
                "渡船靠在北岸這側，船腹碰岸時，踏板前的人聲短短收住。",
                "船身貼著北岸木樁輕晃，濕繩繃在踏板邊，等船的人都看著船戶手勢。",
            ],
            seed,
        ),
        FerryPhase::CrossingSouth => pick_scene_variant(
            &[
                "渡船才離北岸不遠，撐篙在水面攪開一道還沒合攏的痕。",
                "船影正往南岸滑去，北岸踏板前的腳步暫時收住。",
            ],
            seed,
        ),
        FerryPhase::SouthDocked => pick_scene_variant(
            &[
                "渡船停在南岸那頭，北岸這邊的人聲便比水聲更急些。",
                "對岸船邊有人影起落，北岸踏板空著，只剩船繩在木樁旁垂著。",
            ],
            seed,
        ),
        FerryPhase::CrossingNorth => pick_scene_variant(
            &[
                "渡船正從南岸回頭，船頭在河霧裡一點一點放大。",
                "水面傳來撐篙聲，等船的人群跟著把視線轉回踏板前。",
            ],
            seed,
        ),
    }
}

fn north_bank_trace(
    recent_events: &[String],
    climate_key: &str,
    ferry: FerryPhase,
    unnamed_total: usize,
    seed: i64,
) -> &'static str {
    if let Some(trace) = trace_from_recent_room_events(recent_events) {
        return trace;
    }
    match climate_key {
        "rain" | "plum_rain" | "storm" => pick_scene_variant(
            &[
                "岸泥上新舊腳印疊在一起，靠近踏板的幾道還泛著水光。",
                "棚腳滴下的雨水把泥痕沖淡，又被新的腳步踩亂。",
            ],
            seed,
        ),
        "fog" => pick_scene_variant(
            &[
                "霧裡偶爾露出一串新踩出的濕腳印，往踏板方向斷斷續續。",
                "木樁旁的濕繩痕還清楚，像是被人重新繞緊過。",
            ],
            seed,
        ),
        _ => match ferry {
            FerryPhase::NorthDocked | FerryPhase::CrossingSouth => {
                "踏板邊讓出了一條縫，濕木上還留著一排急促腳印。"
            }
            FerryPhase::SouthDocked | FerryPhase::CrossingNorth if unnamed_total >= 5 => {
                "岸邊腳印交疊得密，幾道貨擔拖痕從市集斜斜壓向踏板前。"
            }
            _ => "",
        },
    }
}

fn trace_from_recent_room_events(recent_events: &[String]) -> Option<&'static str> {
    let joined = recent_events.join(" ");
    let text = joined.trim();
    if text.is_empty() {
        return None;
    }
    if text.contains("貨")
        || text.contains("箱")
        || text.contains("袋")
        || text.contains("搬")
        || text.contains("賣")
    {
        Some("泥地上留著拖過貨擔的新痕，從市集那頭一路壓向踏板前。")
    } else if text.contains("離")
        || text.contains("去")
        || text.contains("往")
        || text.contains("leave")
    {
        Some("踏板前空出一小塊地方，濕腳印沿著人群邊緣斷開。")
    } else if text.contains("到")
        || text.contains("來")
        || text.contains("抵")
        || text.contains("arrive")
    {
        Some("近處多了幾道新腳印，泥水還沒來得及把邊緣抹平。")
    } else if text.contains("說")
        || text.contains("問")
        || text.contains("談")
        || text.contains("低聲")
    {
        Some("市集和踏板之間還留著壓低的交談聲，像水聲一過就會散。")
    } else {
        None
    }
}

fn pick_scene_variant(pool: &'static [&'static str], seed: i64) -> &'static str {
    if pool.is_empty() {
        return "";
    }
    let idx = seed.unsigned_abs() as usize % pool.len();
    pool[idx]
}

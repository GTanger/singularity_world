fn clean_catalog_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 3 <= chars.len() {
            let prefix: String = chars[i..i + 3].iter().collect();
            if prefix == "WT-" || prefix == "WS-" {
                let mut j = i + 3;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                if j > i + 3 {
                    while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                        j += 1;
                    }
                    i = j;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}


fn apply_catalog_effects_part1(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now: i64,
    narrative: &mut String,
) -> bool {
    if (obj.id.starts_with("wt_stall_") || obj.id == "wt_street_roast_snack_cart") && action == "Look" {
        if let Some((_item_id, item_name, price_mg, stock)) =
            crate::server::action::trade::get_vendor_stall_stock(&obj.id)
        {
            narrative.push_str(&format!(
                "\n（攤上正販售【{item_name}】，每份售價 {price_mg} 鎂，目前現貨尚餘 {stock} 份。）"
            ));
        }
        return true;
    }
    if matches!(action, "Use" | "Operate" | "Taste" | "Wear")
        && let Some((tag, vit_gain, qi_gain, _consume, effect_narr)) = crate::game::shanhai_item_use_effect(&obj.id)
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + vit_gain).min(100);
                e.qi = (e.qi + qi_gain).min(100);
            });
        }
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        crate::pg::living_world::clear_entity_afflictions(pid, true, now);
        narrative.push_str(&format!(
            "\n【{tag}】{effect_narr}（山海本草化入肺腑，氣血隨之充盈、精神一振）"
        ));
        push_refresh(conn, pid);
        return true;
    }
    // 1) Property Deeds (obj.id == "wt_cat_0793" or obj.id == "wt_reg_capital_specialty" or obj.name.contains("紅印屋契"))
    if (obj.id == "wt_cat_0793"
        || obj.id == "wt_reg_capital_specialty"
        || obj.name.contains("紅印屋契"))
        && (action == "Acquire" || action == "Use" || action == "Operate")
    {
        let magnesium = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.magnesium)
            .unwrap_or(0);
        let has_deed = db::inventory_has_item(pid, "wt_cat_0793");
        if !has_deed {
            if magnesium >= 25 {
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -25);
                let _ = db::add_to_inventory(pid, "wt_cat_0793", 1);
                let _ = db::add_to_inventory(pid, "key_brass_home_lock", 1);
                let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "wt_deed_signed");
                narrative.push_str("\n【置產落籍】你交付 25 鎂契稅定金，正式簽立「公議堂紅印屋契」並領取一把「黃銅彈子門鎖鑰匙」收入行囊，在半島擁有了屬於自己的安身家屋。");
                push_refresh(conn, pid);
            } else {
                narrative.push_str("\n【置產提示】簽立正式紅印屋契需備妥 25 鎂契稅定金，你目前已先行翻閱契書條款並完成意向登記。");
            }
        } else {
            narrative.push_str("\n【置產落籍】你已持有「公議堂紅印屋契」，隨時可憑契文與門鎖鑰匙出入並安居於半島家屋。");
        }
        return true;
    }

    // 1.5) 文曜特色器物實質互動回饋
    // 講堂街三百六十星軌銅儀 (wt_cat_0734) - 觀星測韻
    if (obj.id == "wt_cat_0734" || obj.id == "wt_astronomical_armillary" || obj.name.contains("星軌銅儀"))
        && (action == "Operate" || action == "Read" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 15).min(100);
            });
        }
        let _ = event::append(now, pid, types::PLAYER_BODY, "astronomical_calibrated");
        narrative.push_str("\n【觀星測韻】你輕輕轉動三層黃銅星環，黃道規線與子午環在齒輪咬合間微調校準。望筒深處映現星軌明滅，清冷星光透體而過，你只覺靈台澄澈、元氣湧動，獲得星軌校準領悟。");
        append_somatic_awareness_note(pid, now, narrative);
        if let Ok((_mut_id, _mut_name, mut_narrative)) =
            crate::game::object_ecology::mutate_finished_item(pid, "auto", &obj.name, now)
        {
            narrative.push_str(&format!("\n【星軌氣脈共鳴異變】在星環轉動與星光垂照之下，{}", mut_narrative));
        }
        push_refresh(conn, pid);
        return true;
    }

    // 三水總商行鎂幣分格存摺櫃 / 存摺 (wt_cat_0790) - 商行票號
    if (obj.id == "wt_cat_0790" || obj.id == "wt_bank_ledger_drawer" || obj.name.contains("存摺") || obj.name.contains("票號"))
        && (action == "Operate" || action == "Read" || action == "Open" || action == "Use" || action == "Look" || action == "DepositBank" || action == "WithdrawBank")
    {
        let magnesium = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.magnesium)
            .unwrap_or(0);
        let bank_balance = crate::npc::causal_core::get_player_bank_balance(pid);

        if action == "DepositBank" || action == "Operate" {
            let (actual, new_pocket, new_bank) = crate::npc::causal_core::deposit_player_bank_magnesium(pid, 10);
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "bank_deposit");
            if actual > 0 {
                narrative.push_str(&format!(
                    "\n【存入票號妥當】商行掌事核對成色後將 {} 鎂幣入帳歸檔，並在你的分格存摺上用印簽章。\n本次存入：{} 鎂幣\n隨身攜帶資財：{} 鎂幣\n票號存摺存餘：{} 鎂幣",
                    actual, actual, new_pocket, new_bank
                ));
            } else {
                narrative.push_str(&format!(
                    "\n【存入未成】隨身囊中未有足額鎂幣可供存入。目前隨身持有：{} 鎂幣，票號存摺存餘：{} 鎂幣。",
                    new_pocket, new_bank
                ));
            }
        } else if action == "WithdrawBank" || action == "Use" {
            let (actual, new_pocket, new_bank) = crate::npc::causal_core::withdraw_player_bank_magnesium(pid, 10);
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "bank_withdraw");
            if actual > 0 {
                narrative.push_str(&format!(
                    "\n【憑摺支取完成】商行掌事驗看存摺印信後，自保險庫櫃中點出 {} 鎂幣如數交到你手中。\n本次支取：{} 鎂幣\n隨身攜帶資財：{} 鎂幣\n票號存摺存餘：{} 鎂幣",
                    actual, actual, new_pocket, new_bank
                ));
            } else {
                narrative.push_str(&format!(
                    "\n【支取未成】你在票號存摺中目前尚無結存資財。隨身持有：{} 鎂幣，票號存摺存餘：{} 鎂幣。",
                    new_pocket, new_bank
                ));
            }
        } else {
            // Look / Read / Open
            let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "bank_ledger_checked");
            narrative.push_str(&format!(
                "\n【三水商行・分格存摺簿】你翻開三水總商行分格存摺櫃內的通兌流水簿與防偽印摺，各鎮分號硃印嚴謹無訛。\n三水總商行通存通兌狀態：正常營運。\n你目前隨身攜帶資財：{} 鎂幣\n票號存摺累計存餘：{} 鎂幣\n憑此存摺印信，可在半島各重鎮商行隨時辦理【存入票號】或【憑摺支取】。",
                magnesium, bank_balance
            ));
        }
        push_refresh(conn, pid);
        return true;
    }

    // 剪剃鋪精鋼摺疊剃刀 / 溫水洗頭木躺椅 (wt_barber_folding_razor / wt_barber_copper_chair / wt_cat_0630 / wt_cat_0631)
    if (obj.id == "wt_barber_folding_razor"
        || obj.id == "wt_barber_copper_chair"
        || obj.id == "wt_cat_0630"
        || obj.id == "wt_cat_0631"
        || obj.name.contains("剃刀")
        || obj.name.contains("洗頭")
        || obj.name.contains("理髮"))
        && (action == "Operate" || action == "Use" || action == "Sit" || action == "WarmHands")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 10).min(100);
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::PLAYER_BODY, "barber_groomed");
        narrative.push_str("\n【修容舒爽】熱毛巾敷面溫潤身心，精鋼剃刀輕柔修整鬚角與碎髮，再經紫銅管溫水沖洗，整個人容光煥發、精神抖擻。");
        push_refresh(conn, pid);
        return true;
    }

    // 商算學舍烏木案與十三檔算盤 (wt_abacus_ebony_desk / wt_cat_0788)
    if (obj.id == "wt_abacus_ebony_desk"
        || obj.id == "wt_cat_0788"
        || obj.name.contains("算盤")
        || obj.name.contains("商算"))
        && (action == "Operate" || action == "Use" || action == "Read")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + 10).min(100);
            });
        }
        let _ = event::append(now, pid, types::PLAYER_BODY, "abacus_practiced");
        narrative.push_str("\n【珠算實習】你拂去烏木檔間微塵，凝神運指撥動竹骨算珠，隨口訣起落核算進出貨單，珠聲清脆如落玉，心神一片專注沉靜。");
        push_refresh(conn, pid);
        return true;
    }

    // 習武堂包革練習木刀 / 測紋卸力石 (wt_martial_padded_wooden_blade / wt_martial_force_stone / wt_cat_1015 / wt_cat_0846)
    if (obj.id == "wt_martial_padded_wooden_blade"
        || obj.id == "wt_martial_force_stone"
        || obj.id == "wt_cat_1015"
        || obj.id == "wt_cat_0846"
        || obj.name.contains("練習木刀")
        || obj.name.contains("卸力")
        || obj.name.contains("試招"))
        && (action == "Operate" || action == "Use")
    {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.vit = (e.vit + 12).min(100);
            });
        }
        let _ = event::append(now, pid, types::PLAYER_BODY, "martial_drilled");
        narrative.push_str("\n【試招切磋】你緊縛牛皮護腕，手執裹軟革的練習木刀擺開架式，依口令與石靶連續對拆劈刺卸力，步法靈動沉穩，筋骨徹底舒展，體魄更顯堅實。");
        push_refresh(conn, pid);
        return true;
    }

    // 攤位真交易（NPC↔攤販 / 玩家↔攤位物件）
    if (obj.id.starts_with("wt_stall_") || obj.id == "wt_street_roast_snack_cart")
        && matches!(action, "Taste" | "Use" | "Operate")
    {
        if let Some(outcome) = crate::server::action::trade::settle_live_vendor_npc_purchase(
            &obj.id,
            pid,
            player_room,
            now,
        ) {
            match outcome {
                crate::server::action::trade::VendorNpcPurchaseOutcome::Purchased {
                    item_name,
                    price_mg,
                    remaining_stock: _,
                    seller_mg_after: _,
                    ..
                } => {
                    if let Some(arc) = crate::store::get_store() {
                        let _ = arc.write().update_entity(pid, |e| {
                            e.vit = (e.vit + 2).min(100);
                            e.qi = (e.qi + 2).min(100);
                        });
                    }
                    if action == "Taste" {
                        narrative.push_str(&format!(
                            "\n你交付 {price_mg} 枚鎂幣向攤主買下「{item_name}」，當場品嚐幾口並將餘份妥善收入行囊。"
                        ));
                    } else {
                        *narrative = format!(
                            "你走向「{}」，支付 {price_mg} 鎂向攤主買下一份「{item_name}」收進背包，順道溫養氣血，熱騰騰的香氣令人精神一振。",
                            obj.name
                        );
                    }
                    push_refresh(conn, pid);
                    return true;
                }
                crate::server::action::trade::VendorNpcPurchaseOutcome::OutOfStock { item_name } => {
                    *narrative = format!(
                        "你走向「{}」準備購買「{item_name}」，攤主抱歉地擦了擦汗道：「真不巧，最後一份「{item_name}」賣完啦，鍋裡見底，我正準備去補料，勞駕稍候再來！」",
                        obj.name
                    );
                    push_refresh(conn, pid);
                    return true;
                }
                crate::server::action::trade::VendorNpcPurchaseOutcome::InsufficientFunds {
                    item_name,
                    price_mg,
                    buyer_mg,
                } => {
                    *narrative = format!(
                        "你走向「{}」想買一份「{item_name}」（每份 {price_mg} 鎂），摸了摸錢袋發現手頭僅有 {buyer_mg} 鎂，只好先作罷。",
                        obj.name
                    );
                    return true;
                }
            }
        }
    }

    // 修傘補靴攤長凳與修具 (wt_repair_umbrella_bench / wt_cat_0896 / wt_cat_0888 / wt_cat_0890)
    if (obj.id == "wt_repair_umbrella_bench"
        || obj.id == "wt_cat_0896"
        || obj.id == "wt_cat_0888"
        || obj.id == "wt_cat_0890"
        || obj.name.contains("修傘")
        || obj.name.contains("補靴")
        || obj.name.contains("修補攤"))
        && (action == "Operate" || action == "Use")
    {
        let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, "gear_repaired");
        narrative.push_str("\n【街坊巧藝】你坐在修造長凳旁，借熟鐵大剪與桐油膠漆仔細加固隨身防雨傘骨與行靴縫線，行裝完好紮實，不懼長途風雨。");
        push_refresh(conn, pid);
        return true;
    }

    // 2.1) Merchant Ledger Counter (厚樟木商契掌事櫃檯：商契蓋印與核銷)
    if (obj.id == "wt_merchant_ledger_counter" || obj.name.contains("商契掌事櫃台") || obj.name.contains("商契掌事櫃檯"))
        && (action == "Use" || action == "Operate")
    {
        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, 10);
        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str("\n【商契蓋印】你在厚樟木掌事櫃檯前出示商行契據，值班執事仔細核對無誤後在契本上重重蓋下硃砂印鑑，完成商契核銷與物資調撥紀錄（領取 10 鎂核銷酬勞）。");
        push_refresh(conn, pid);
        return true;
    }

    // 2.15) Caravan Transit Hub (商隊聯運票務櫃檯：跨聚落快速聯運)
    if (obj.id == "wt_caravan_ticket_counter"
        || obj.name.contains("聯運班次")
        || obj.name.contains("車票")
        || obj.name.contains("票務櫃檯")
        || obj.name.contains("票務櫃台"))
        && (action == "Use" || action == "Operate" || action == "Move")
    {
        const TRANSIT_STOPS: &[(&str, i32, i32, &str)] = &[
            ("文曜", 0, -1, "三水交匯的學風重鎮，車馬喧囂，書香與市井氣息交融"),
            ("水尾寮", 0, 125, "白塔北境清泉奔湧的堰頭樞紐，水汽氤氳，農人引水灌田"),
            ("半嶺鎮", 160, 210, "半島東北地勢起伏的丘陵山鎮，茶香撲鼻，山徑蜿蜒"),
            ("紅土崁", -180, -140, "西南灌溉渠網縱橫的沃土農莊，桑麻遍野，機杼聲聲"),
            ("南哨", 0, -220, "半島最南端風蝕礁岩林立的哨所防線，海風呼嘯，崗樓巍峨"),
            ("風舶角", -260, 160, "西北面臨怒海狂濤的深水海港，桅檣如林，鹽霧瀰漫"),
            ("霜林口", 190, 290, "北坡高聳入雲的雲杉林場聚落，水車排鋸轟鳴，松脂木香四溢"),
            ("下營", 0, -450, "半島極南險隘要衝，扼守山川門戶，商旅車隊往來絡繹"),
            ("烏石鼻", -260, 35, "黑曜玄武岩壁立的迎浪漁港，石階泊位井然，海風鹹鮮"),
        ];
        let ent = db::get_entity(pid).ok().flatten();
        let (cur_x, cur_y) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0));
        let magnesium = ent.as_ref().map(|e| e.magnesium).unwrap_or(0);

        if magnesium < 5 {
            narrative.push_str("\n【商隊聯運客運】值守票務執事翻看名冊後溫言道：「跨聚落商隊聯運每程需 5 鎂乘車費，手頭盤纏尚且不夠，待備妥路費再來搭車吧。」");
            push_refresh(conn, pid);
            return true;
        }

        // 尋找下一個目的地（若在某一聚落附近則循環前往下一站）
        let current_stop_idx = TRANSIT_STOPS.iter().position(|(_, x, y, _)| {
            (cur_x - *x).abs() <= 30 && (cur_y - *y).abs() <= 30
        });
        let next_idx = match current_stop_idx {
            Some(idx) => (idx + 1) % TRANSIT_STOPS.len(),
            None => 0,
        };
        let (dest_name, dest_x, dest_y, dest_desc) = TRANSIT_STOPS[next_idx];

        let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, -5);
        let _ = db::set_entity_grid(pid, dest_x, dest_y);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().move_entity(pid, &format!("grid:{dest_x}:{dest_y}"));
        }
        super::super::handler::send_place_view(conn, pid);
        push_refresh(conn, pid);

        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("caravan_transit|{}|{},{}", dest_name, dest_x, dest_y),
        );

        narrative.push_str(&format!(
            "\n【商隊聯運客運】你在聯運櫃檯點付 5 鎂車資，執事為你取來竹編乘輿牌。你登上厚棚雙輪大車，車伕一揚皮鞭，健碩的挽獸邁開大步，車輪在鋪石道與土徑上轆轆作響。行過半島層疊丘陵與原野，約莫半小時後順利抵達【{}】（{}）！",
            dest_name, dest_desc
        ));
        return true;
    }
    if obj.id == "wt_guild_commission_board"
        || obj.id == "wt_hall_bulletin_board"
        || obj.id == "wt_cat_0794"
        || obj.name.contains("委託")
        || obj.name.contains("告示")
        || obj.name.contains("公佈欄")
    {
        let room_name = resolve_room_name(player_room);
        let scarce_opt = crate::game::object_ecology::scarcest_item_for_settlement(&room_name);
        let supply_demand_report = crate::game::object_ecology::format_settlement_supply_demand_report(&room_name);

        let player_inv = db::get_entity(pid).ok().flatten().map(|player_ent| {
            db::get_inventory(&player_ent.inventory, player_ent.vit)
        });

        let active_pack = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
            &room_name,
            None,
            now,
        );
        let player_name = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| if !e.display_title.trim().is_empty() { e.display_title } else { e.id })
            .unwrap_or_else(|| "外來行客".to_string());

        let active_orders = crate::npc::causal_core::list_active_stranded_cargo_orders(now);
        let has_unclaimed = active_orders.iter().any(|o| o.claimed_by.is_none());
        let player_holds_stranded = active_orders
            .iter()
            .any(|o| db::inventory_has_item(pid, &o.item_id));

        // 1. 揭榜受滯貨單
        if action == "ClaimStrandedCargo"
            || (action == "Operate" && has_unclaimed && !player_holds_stranded)
        {
            if let Some(order) = crate::npc::causal_core::claim_stranded_cargo_for_player(
                pid,
                &player_name,
                "",
                &room_name,
                now,
            ) {
                let _ = db::add_to_inventory(pid, &order.item_id, order.qty.max(1));
                let _ = event::append(
                    now,
                    pid,
                    types::SCENE_OBJECT_ACTION,
                    &format!("{}|claim_stranded_cargo|{}", obj.id, order.order_id),
                );
                narrative.push_str(&format!(
                    "\n【封蠟行會受滯貨單・揭榜管運】\n你揭下告示板上的「{}」急遞單，接管了由行商【{}】託運的 {} 份【{}】！\n因途經險阻，貨物急需送達目的地【{}】公議堂／行會驗收。\n你將物資與封蠟單據妥善收入行囊，承諾護送前往交割；送達後將由目的聚落公庫足額核撥 {} 鎂託管賞格！",
                    order.order_id,
                    order.carrier_name,
                    order.qty.max(1),
                    order.item_name,
                    order.dest_settlement,
                    order.escrow_bounty_mg
                ));
                push_refresh(conn, pid);
                return true;
            } else if action == "ClaimStrandedCargo" {
                narrative.push_str("\n【行會提示】目前全島各驛站受滯貨單皆已由巡守隊或行客接洽接管，暫無待揭榜之貨單。");
                push_refresh(conn, pid);
                return true;
            }
        }

        // 2. 交割受滯貨單
        if action == "DeliverStrandedCargo"
            || ((action == "Deliver" || action == "Use") && player_holds_stranded)
        {
            if player_holds_stranded
                && let Some((order, bounty)) = crate::npc::causal_core::deliver_stranded_cargo_for_player(
                pid,
                &player_name,
                "",
                &room_name,
                now,
            ) {
                let _ = db::remove_from_inventory(pid, &order.item_id, order.qty.max(1));
                let _ = event::append(
                    now,
                    pid,
                    types::SCENE_OBJECT_ACTION,
                    &format!("{}|deliver_stranded_cargo|{}", obj.id, order.order_id),
                );
                narrative.push_str(&format!(
                    "\n【封蠟行會受滯貨單・交割銷單】\n你向告示板掌事書吏出示受滯貨單，將護送抵埠的 {} 份【{}】清點交付！\n目的地【{}】之物資緊缺之勢隨之大為舒緩，行商【{}】得訊後感激不盡，連聲道謝。\n公庫託管賞格 {} 鎂已足額核撥結算至你的隨身行囊！",
                    order.qty.max(1),
                    order.item_name,
                    order.dest_settlement,
                    order.carrier_name,
                    bounty
                ));
                push_refresh(conn, pid);
                return true;
            } else if action == "DeliverStrandedCargo" {
                narrative.push_str("\n【交割提示】你目前隨身未持有需在此聚落交割之受滯貨物，或尚未抵達貨單指定之目的聚落。");
                push_refresh(conn, pid);
                return true;
            }
        }

        // 3. 檢查玩家行囊是否備有本地急缺物資（僅在玩家主動交付/操作時觸發）
        let delivered_item = if action == "Use" || action == "Operate" || action == "Deliver" {
            player_inv.as_ref().and_then(|inv| {
                // 優先比對：本地最緊缺的供需短缺物產
                if let Some((s_id, s_name, _cur_stock, _tgt_stock, price)) = &scarce_opt {
                    if let Some(_it) = inv.items.iter().find(|i| &i.item_id == s_id && i.qty > 0) {
                        return Some((s_id.clone(), s_name.clone(), *price));
                    }
                }
                // 次要比對：當前街坊公議腳本之關鍵物產（key_objects 明確提及者）
                if let Some((ev_pack, _sc_pack)) = &active_pack {
                    let clean_keys = clean_catalog_codes(ev_pack.key_objects);
                    for it in &inv.items {
                        if it.qty <= 0 { continue; }
                        let item_name = db::get_item_def(&it.item_id)
                            .map(|d| d.name)
                            .or_else(|| {
                                crate::game::peninsula_catalog::find_catalog_entry(&it.item_id)
                                    .map(|c| c.name.to_string())
                            })
                            .filter(|n| !n.trim().is_empty())
                            .unwrap_or_else(|| it.item_id.clone());
                        if clean_keys.contains(&item_name) || ev_pack.key_objects.contains(&it.item_id) {
                            let base_price = crate::world::materials::trade_price_anchor_mg(&it.item_id)
                                .or_else(|| {
                                    crate::game::peninsula_catalog::find_catalog_entry(&it.item_id)
                                        .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
                                })
                                .unwrap_or(20);
                            return Some((it.item_id.clone(), item_name, base_price));
                        }
                    }
                }
                None
            })
        } else {
            None
        };

        if let Some((item_id, item_name, unit_price)) = delivered_item {
            if db::remove_from_inventory(pid, &item_id, 1).is_ok() {
                let bounty_mg = unit_price + 10;
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, &room_name, bounty_mg);
                let _ = crate::game::object_ecology::record_settlement_market_trade(
                    &room_name,
                    &item_id,
                    unit_price,
                    1,
                    "委託交割",
                );
                let _ = event::append(
                    now,
                    pid,
                    types::SCENE_OBJECT_ACTION,
                    &format!("{}|commission_delivery|{}", obj.id, item_id),
                );
                let (host_opt, _) = crate::game::peninsula_scripts::resolve_on_duty_actors_for_room(&room_name);
                let host_label = host_opt.unwrap_or_else(|| "本坊當值掌事".to_string());
                let mentor_key = format!("{room_name}{host_label}");
                let (imprint, newly_acquired) = crate::pg::living_world::record_mentor_imprint_progress(
                    pid,
                    &mentor_key,
                    "百工衡準訣",
                    "熟稔半島各坊物材成色與發力分寸，運器施作更為穩健",
                    now,
                );
                let imprint_note = if imprint.interaction_count == 1 {
                    format!(
                        "\n【百工指點】{host_label}見你交割物資成色地道，順口指點了你幾句《{}》（{}）的發力分寸。",
                        imprint.imprint_title,
                        imprint.imprint_prose
                    )
                } else if newly_acquired {
                    let _ = db::add_to_inventory(pid, "blank_manual", 1);
                    let mut hasher = 0xcbf29ce484222325u64;
                    for b in format!("{}_{}", imprint.mentor_name, imprint.imprint_title).bytes() {
                        hasher ^= b as u64;
                        hasher = hasher.wrapping_mul(0x100000001b3);
                    }
                    let manual_hash = format!("{:016x}", hasher);
                    let formula_word_ids = vec!["zhun".to_string(), "ding".to_string(), "he_tai".to_string()];
                    let formula_word_chars = vec!["準".to_string(), "定".to_string(), "合".to_string()];
                    let manual_rec = crate::pg::living_world::TranscribedManualRecord {
                        manual_hash: manual_hash.clone(),
                        author_id: imprint.mentor_name.clone(),
                        author_name: host_label.clone(),
                        formal_name: imprint.imprint_title.clone(),
                        slot_label: "百工衡準".to_string(),
                        carrier_code: "tool".to_string(),
                        word_ids: formula_word_ids.clone(),
                        word_chars: formula_word_chars.clone(),
                        origin_basis: imprint.imprint_prose.clone(),
                        created_at: now,
                    };
                    crate::pg::living_world::save_transcribed_manual(&manual_rec);
                    let manual_item_id = format!("manual_{manual_hash}");
                    let _ = db::add_to_inventory(pid, &manual_item_id, 1);

                    let word_id_refs: Vec<&str> = formula_word_ids.iter().map(|s| s.as_str()).collect();
                    crate::server::handler::inventory::grant_words_to_player_casket(pid, &word_id_refs);

                    format!(
                        "\n【百工師承・心領神會】{host_label}端詳物材成色，讚許點頭，贈予你一卷親筆手書的《{}》手札與空白手札，並手把手點撥你辨識各坊物性與運力分寸——領悟師承烙印【{}】！（{}）",
                        imprint.imprint_title,
                        imprint.imprint_title,
                        imprint.imprint_prose
                    )
                } else {
                    format!(
                        "\n【百工掌事指點】{host_label}稱善：「物材質地極佳，解了工坊燃眉之急！」其言簡意賅的指點令你對【{}】的體會又深了一層。",
                        imprint.imprint_title
                    )
                };
                narrative.push_str(&format!(
                    "\n【本坊急缺物資交割完成】你依行會告示板之急徵令，自隨身行囊交付一份急缺的【{item_name}】供本地工坊應急！\n{host_label}點收後蓋印銷單，當場核發交割懸賞 {bounty_mg} 鎂酬勞，本坊物資緊缺之勢隨之舒緩。{imprint_note}"
                ));
                push_refresh(conn, pid);
                return true;
            }
        }

        // 4. 未攜帶急缺物資：展示本坊街坊公議、即時供需短缺報告、受滯貨單與山道靖安通報
        let script_section = if let Some((ev, sp)) = &active_pack {
            let clean_objects = clean_catalog_codes(ev.key_objects);
            format!(
                "【街坊公議與物資徵集・{}】\n・主事場所：{}（{}）\n・徵集物產：{}\n・事由始末：{}\n・協力途徑：{}",
                ev.event_title, sp.location_context, sp.host_slot_actor, clean_objects, ev.trigger_and_causal_chain, sp.player_actions
            )
        } else {
            "【街區公務告示】目前各坊市當家正按各自班表出工，暫無加急公務告示。".to_string()
        };

        let shortage_notice = if let Some((_, scarce_name, _cur_stock, _tgt_stock, _unit_price)) = &scarce_opt {
            format!("【本坊急徵物資】目前急徵「{}」，行會特設專項酬勞廣納應急物產。\n{}", scarce_name, supply_demand_report)
        } else {
            format!("【本坊急徵物資】本地物產暫無嚴重缺口，常備物資平穩運轉中。\n{}", supply_demand_report)
        };

        let stranded_section = if active_orders.is_empty() {
            "【封蠟行會急遞・受滯貨單】\n目前全島商路平穩，各驛站暫無滯留貨單。".to_string()
        } else {
            let mut lines = vec!["【封蠟行會急遞・受滯貨單】".to_string()];
            for ord in active_orders.iter().take(3) {
                let claim_tag = match &ord.claimed_by {
                    Some(claimer) if claimer == pid => "【你管運中】",
                    Some(_) => "【已有行客管運】",
                    None => "【待揭榜承領】",
                };
                lines.push(format!(
                    "・{} 行商「{}」滯留於「{}」：急需送達「{}」之「{}」，公庫託管賞格 {} 鎂",
                    claim_tag,
                    ord.carrier_name,
                    ord.waystation_name,
                    ord.dest_settlement,
                    ord.item_name,
                    ord.escrow_bounty_mg
                ));
            }
            lines.join("\n")
        };

        const MAJOR_PASS_ZONES: &[&str] = &["赤石坡", "西麓林道", "清泉溪谷", "長風草甸", "古河巨木林"];
        let mut spillover_lines = Vec::new();
        for &zone_name in MAJOR_PASS_ZONES {
            let z_idx = crate::game::resolve_zone_idx_from_name_or_settlement(zone_name);
            if crate::game::is_zone_predator_spillover_active(z_idx) {
                spillover_lines.push(format!("・【{}】猛獸受驚越界伏行，山道行旅受阻，請往來商隊與巡行者多加戒備！", zone_name));
            }
        }
        let spillover_section = if spillover_lines.is_empty() {
            "【山道靖安通報】\n各主要關隘與山道巡防井然，暫無猛獸越界侵擾通報。".to_string()
        } else {
            format!("【山道靖安通報】\n{}", spillover_lines.join("\n"))
        };

        let _ = event::append(
            now,
            pid,
            types::SCENE_OBJECT_ACTION,
            &format!("{}|{}", obj.id, action),
        );
        narrative.push_str(&format!(
            "\n{}\n\n{}\n\n{}\n\n{}",
            script_section, shortage_notice, stranded_section, spillover_section
        ));
        push_refresh(conn, pid);
        return true;
    }

    // 海事承保櫃檯 (築港頭分級海事保單)
    if obj.id == "wt_harbor_insurance_desk"
        || obj.name.contains("海事承保")
        || obj.name.contains("海事保單")
    {
        let room_name = resolve_room_name(player_room);
        let weather = current_zone_weather(&room_name);
        let tide = current_zone_tide(&room_name);
        let (host_opt, _) = crate::game::peninsula_scripts::resolve_on_duty_actors_for_room(&room_name);
        let host_label = host_opt.unwrap_or_else(|| "海事櫃檯當值掌事".to_string());

        if action == "Look" || action == "Read" {
            let weather_desc = match weather {
                crate::game::ZoneWeatherPhase::RainMist => "煙雨空濛（風浪驟起、海潮翻湧）",
                crate::game::ZoneWeatherPhase::ClearBreeze => "微風拂面（碧波蕩漾、順風好行）",
                crate::game::ZoneWeatherPhase::DryWarm => "天青氣暖（海平浪靜、航路平穩）",
                crate::game::ZoneWeatherPhase::ColdFrost => "寒霜凜冽（洋流偏冷、海霧低垂）",
            };
            let tide_desc = match tide {
                crate::game::ZoneTidePhase::FloodTide => "滿潮（潮位高漲、順流起航）",
                crate::game::ZoneTidePhase::EbbTide => "退潮（落潮回流、航道平緩）",
                crate::game::ZoneTidePhase::SlackTide => "平潮（水勢止息、泊港安穩）",
            };
            narrative.push_str(&format!(
                "\n【築港頭海事承保櫃檯】\n碼頭棧橋旁的厚木櫃檯前擺放著分級海事保單與氣象籌板，由掌事{}親自主持水險承保。\n凡出海客商繳納 6 鎂契費，即可簽立一份防水桐油麻布「築港頭分級海事保單」；若航程遇狂風驟雨致使貨裝海損，抵埠憑單可自風舶角公庫足額領取 12 鎂理賠撫卹。\n當前外海水相氣候：{}\n當前港灣潮汐相位：{}",
                host_label, weather_desc, tide_desc
            ));
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|{}", obj.id, action));
            push_refresh(conn, pid);
            return true;
        }

        if action == "Use" || action == "Operate" || action == "Buy" {
            let magnesium = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.magnesium)
                .unwrap_or(0);
            if magnesium < 6 {
                narrative.push_str(&format!(
                    "\n【承保提示】簽立「築港頭分級海事保單」需 6 鎂契費，隨身錢資不足，承保掌事{}請你備妥盤纏再來投保。",
                    host_label
                ));
                push_refresh(conn, pid);
                return true;
            }
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, "風舶角", -6);
            let _ = db::add_to_inventory(pid, "wt_cat_0792", 1);
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|buy_marine_insurance", obj.id));
            narrative.push_str(&format!(
                "\n【簽立海事水險】你在掌事{}面前交付 6 鎂契費，於防水桐油麻布保單上簽章畫押，將一份「築港頭分級海事保單」妥善收妥入囊。\n{}蓋上硃砂大印叮嚀道：「若出海途中遭遇狂風暴雨造成海損，抵港憑單可自風舶角公庫領取 12 鎂理賠撫卹。」",
                host_label, host_label
            ));
            push_refresh(conn, pid);
            return true;
        }
    }

    // 順時針環島海運商船
    if obj.id == "wt_harbor_clockwise_ship"
        || obj.name.contains("順時針環島海船")
        || obj.name.contains("順時針環島海運商船")
        || obj.name.contains("遠洋順流商船")
    {
        const CLOCKWISE_PORTS: &[(&str, i32, i32, &str)] = &[
            ("烏石鼻", -260, 35, "西南玄武岩黑石岬角，黑潮分流繞行此處"),
            ("風櫃尾", -360, 95, "西海岸風櫃海潮怒吼之險峻海岬，水深流急"),
            ("風舶嶼", -330, 160, "西北離岸島嶼，海面開闊，商船泊靠候潮"),
            ("風舶角", -260, 160, "半島西北深水良港與海運樞紐，萬船雲集"),
            ("南哨", 0, -220, "正南邊陲海疆汛防海港，暖風習習"),
            ("水尾寮", -75, -280, "南端水岸渡口與水運集散良澳，舟楫往來"),
        ];

        let room_name = resolve_room_name(player_room);
        let weather = current_zone_weather(&room_name);
        let tide = current_zone_tide(&room_name);

        let ent = db::get_entity(pid).ok().flatten();
        let (cur_x, cur_y) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((-260, 160));

        let cur_port_idx = CLOCKWISE_PORTS.iter().position(|(name, x, y, _)| {
            room_name.contains(name) || ((cur_x - *x).abs() <= 40 && (cur_y - *y).abs() <= 40)
        });
        let next_idx = match cur_port_idx {
            Some(idx) => (idx + 1) % CLOCKWISE_PORTS.len(),
            None => 0,
        };
        let (next_name, next_x, next_y, next_desc) = CLOCKWISE_PORTS[next_idx];

        if action == "Look" || action == "Read" {
            let weather_desc = match weather {
                crate::game::ZoneWeatherPhase::RainMist => "煙雨空濛（風浪驟起、海潮翻湧）",
                crate::game::ZoneWeatherPhase::ClearBreeze => "微風拂面（碧波蕩漾、順風好行）",
                crate::game::ZoneWeatherPhase::DryWarm => "天青氣暖（海平浪靜、航路平穩）",
                crate::game::ZoneWeatherPhase::ColdFrost => "寒霜凜冽（洋流偏冷、海霧低垂）",
            };
            let tide_desc = match tide {
                crate::game::ZoneTidePhase::FloodTide => "滿潮（潮位高漲、順流起航）",
                crate::game::ZoneTidePhase::EbbTide => "退潮（落潮回流、航道平緩）",
                crate::game::ZoneTidePhase::SlackTide => "平潮（水勢止息、泊港安穩）",
            };
            narrative.push_str(&format!(
                "\n【順時針環島海運商船】\n高大結實的雙桅商船泊於碼頭石階旁，順著西岸暖流、北境白塔海角與東部黑潮，依序順時針巡航於全島各海港。\n下一停靠港口：【{}】（{}）\n當前海象氣候：{}\n當前潮汐相位：{}\n船資單程 8 鎂，順流平穩，客貨兩便。",
                next_name, next_desc, weather_desc, tide_desc
            ));
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &format!("{}|{}", obj.id, action));
            push_refresh(conn, pid);
            return true;
        }

        if action == "Use" || action == "Operate" || action == "Board" || action == "BoardCoastalShip" {
            let magnesium = ent.as_ref().map(|e| e.magnesium).unwrap_or(0);
            if action == "BoardCoastalShip" && (room_name.contains("文曜") || room_name.contains("三水")) {
                if magnesium < 4 {
                    narrative.push_str("\n【盤纏不足】於三水口渡埠登船巡航需 4 鎂船資，隨身盤纏不足，舟子溫言請你備妥船資再來登船。");
                    push_refresh(conn, pid);
                    return true;
                }
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, &room_name, -4);
                let _ = db::add_to_inventory(pid, "item_coastal_sea_chart", 1);
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(pid, |e| {
                        e.qi = (e.qi + 14).min(100);
                    });
                }
                crate::pg::living_world::record_breath_activity(
                    pid,
                    crate::pg::living_world::BreathHabitKind::TideContinuous,
                    3,
                    now,
                );
                let weather_desc = match weather {
                    crate::game::ZoneWeatherPhase::RainMist => "煙雨空濛、江海翻湧",
                    crate::game::ZoneWeatherPhase::ClearBreeze => "微風拂面、碧波蕩漾",
                    crate::game::ZoneWeatherPhase::DryWarm => "天青氣暖、江流平緩",
                    crate::game::ZoneWeatherPhase::ColdFrost => "寒霜凜冽、江霧低垂",
                };
                let tide_desc = match tide {
                    crate::game::ZoneTidePhase::FloodTide => "滿潮湧入、江水充沛",
                    crate::game::ZoneTidePhase::EbbTide => "退潮奔流、順流疾行",
                    crate::game::ZoneTidePhase::SlackTide => "平潮定波、水勢平穩",
                };
                narrative.push_str(&format!(
                    "\n【江海客船・黑潮巡航】你在三水口渡埠交付 4 鎂船資登上客船，隨江水巡航出海。舷外天候正逢「{}」，江海潮位正值「{}」。你手持航海木尺與羅盤，參照黑潮水流細細校核航道水深，收妥一份「黑潮順時針海圖摺頁」後乘順水穩步回泊渡埠，海潮綿息隨之凝練。",
                    weather_desc, tide_desc
                ));
                let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, "wenyao_coastal_cruise");
                push_refresh(conn, pid);
                return true;
            }
            if magnesium < 8 {
                narrative.push_str("\n【登船提示】搭乘順時針環島海運商船需 8 鎂船資，隨身盤纏不足，水手溫言請你備妥船資再來登船啟程。");
                push_refresh(conn, pid);
                return true;
            }
            if action == "BoardCoastalShip" {
                let _ = db::add_to_inventory(pid, "item_coastal_sea_chart", 1);
                crate::pg::living_world::record_breath_activity(
                    pid,
                    crate::pg::living_world::BreathHabitKind::TideContinuous,
                    3,
                    now,
                );
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(pid, |e| {
                        e.qi = (e.qi + 14).min(100);
                    });
                }
            }

            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, &room_name, -8);
            let _ = db::set_entity_grid(pid, next_x, next_y);
            if let Some(arc) = crate::store::get_store() {
                let _ = arc.write().move_entity(pid, &format!("grid:{next_x}:{next_y}"));
            }

            let is_stormy = weather == crate::game::ZoneWeatherPhase::RainMist
                || weather == crate::game::ZoneWeatherPhase::ColdFrost
                || crate::db::get_region_climate("bai_ta_peninsula")
                    .ok()
                    .flatten()
                    .map(|c| matches!(c.event_type.as_str(), "rain" | "plum_rain" | "storm" | "heavy_rain" | "snow"))
                    .unwrap_or(false);

            let has_insurance = db::inventory_has_item(pid, "wt_cat_0792");

            let insurance_note = if is_stormy && has_insurance {
                let _ = db::remove_from_inventory(pid, "wt_cat_0792", 1);
                let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, "風舶角", 12);
                "\n【海事水險理賠】航程中突遇狂浪顛簸，部分隨身貨裝受潮海損；所幸你備有「築港頭分級海事保單」，抵埠時掌事核銷單據，由風舶角公庫如數給付 12 鎂海事水險理賠撫卹！"
            } else if is_stormy {
                "\n【風浪顛簸】途中遭遇驟雨海霧，風浪顛簸，所幸掌舵老水手經驗老道，商船化險為夷安抵新港。"
            } else {
                "\n【順流航行】商船揚帆順流破浪，一路碧波萬頃，海風徐徐，十分順遂平穩。"
            };

            narrative.push_str(&format!(
                "\n【海運航行抵達】你支付 8 鎂船資登上商船，隨順時針黑潮與暖流航向【{}】（{}）。\n商船穩穩靠泊碼頭棧橋，你順著跳板踏上海港石階。{}",
                next_name, next_desc, insurance_note
            ));

            let _ = event::append(
                now,
                pid,
                types::SCENE_OBJECT_ACTION,
                &format!("clockwise_ship|{}|{},{}", next_name, next_x, next_y),
            );
            super::super::handler::send_place_view(conn, pid);
            push_refresh(conn, pid);
            return true;
        }
    }

    false
}

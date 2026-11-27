//! 全服單一 `UnifiedMindBody` 城鎮商鋪生計、二樓木箱、拾遺申報與複式銅幣守恆流轉（零靜態 `doc.residents` 依賴）。

use super::clock_and_lang::SolarClockSnapshot;
use super::explore_and_combat::resolve_skirmish_turn;
use super::mind::EventPublicity;
use super::movement::{add_carry_item, add_language_exposure, push_breath, take_carry_item};
use super::persona::{ValueAxis, derive_innate_persona};
use super::sim_tick::broadcast_scene_event;
use super::types::{CarryItem, IslandWorldDelta, PickupRecord, WorldStaticDoc};

/// 雙向記錄兩名 `UnifiedMindBody` 之間的守信交易證據（不分玩家或居民，100% 對稱）
fn record_mutual_trade_evidence(
    delta: &mut IslandWorldDelta,
    game_days: i64,
    actor_a_id: &str,
    actor_b_id: &str,
    summary_zh: &str,
) {
    if actor_a_id == actor_b_id {
        return;
    }
    let name_a = delta.actors.get(actor_a_id).map(|a| a.display_name.clone());
    let name_b = delta.actors.get(actor_b_id).map(|b| b.display_name.clone());
    let (Some(na), Some(nb)) = (name_a, name_b) else {
        return;
    };
    if let Some(a) = delta.actors.get_mut(actor_a_id) {
        a.mind
            .record_pact_or_trade_evidence(actor_b_id, &nb, game_days, true, false, summary_zh);
        a.mark_mutated();
    }
    if let Some(b) = delta.actors.get_mut(actor_b_id) {
        b.mind
            .record_pact_or_trade_evidence(actor_a_id, &na, game_days, true, false, summary_zh);
        b.mark_mutated();
    }
}

use super::scene_sockets::{find_same_node_counterparty, parse_counterparty_and_quote};

pub fn apply_world_action(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    action: &str,
    param: &str,
) -> Result<String, String> {
    let Some(aid) = delta.resolve_actor_id(actor_key) else {
        return Err("請先向港務官核對通行身分。".to_string());
    };
    let (setup_ok, in_skirmish, in_transit, node_id, ctrl_opt) = {
        let Some(a) = delta.actors.get_mut(&aid) else {
            return Err("請先向港務官核對通行身分。".to_string());
        };
        if a.controller_id.is_some() {
            a.last_active_unix = clock.real_unix;
        }
        (
            a.setup_done,
            a.active_skirmish.is_some(),
            a.active_transit.is_some(),
            a.node_id.clone(),
            a.controller_id.clone(),
        )
    };
    if !setup_ok {
        return Err("請先在港務登岸木冊留下你的名號與來歷。".to_string());
    }

    if action.starts_with("skirmish_") {
        return resolve_skirmish_turn(doc, delta, clock, &aid, action);
    }
    if in_skirmish {
        return Err("眼前正與敵手短兵相接，無暇分心做別的事！".to_string());
    }
    if in_transit
        && !matches!(
            action,
            "carriage_next" | "carriage_alight" | "transit_alight_next"
        )
    {
        return Err(
            "車船正在航道官道上行進，可選擇「下一站下車」或「中途下車」後再辦這件事。".to_string(),
        );
    }

    let Some(node) = doc.nodes.get(&node_id) else {
        return Err("四周霧氣迷濛，暫時辨不清方位。".to_string());
    };

    let (explicit_cp_tok, quoted_copper) = parse_counterparty_and_quote(param);
    let check_quote = |actual_price: i64| -> Result<(), String> {
        if let Some(q) = quoted_copper
            && q != actual_price
        {
            return Err(format!(
                "當面議定現價已變為 {} 枚銅幣，請重新確認報價。",
                actual_price
            ));
        }
        Ok(())
    };
    let find_cp_roles = |roles: &[&str]| {
        find_same_node_counterparty(
            delta,
            &aid,
            &node_id,
            clock.real_unix,
            roles,
            if action == "sell_specialty" {
                None
            } else {
                explicit_cp_tok
            },
        )
    };
    let smith_id = matches!(
        action,
        "forge_help"
            | "repair_weapon"
            | "engrave_rune"
            | "sell_specialty"
            | "commission_custom_vessel"
            | "collect_custom_vessel"
    )
    .then(|| find_cp_roles(&["鍛鐵匠", "鍛鐵匠學徒", "木匠"]))
    .flatten();
    let innkeeper_id = matches!(action, "redeem_voucher" | "rent_inn" | "buy_ration")
        .then(|| find_cp_roles(&["酒館掌櫃", "酒館掌櫃學徒"]))
        .flatten();
    let coach_id = (action.starts_with("board_carriage")
        || action.starts_with("carriage_")
        || action == "board_transit"
        || action == "transit_alight_next")
        .then(|| find_cp_roles(&["車行掌櫃"]))
        .flatten();
    let herb_id = (action == "bandage_wound" || action == "sell_specialty")
        .then(|| find_cp_roles(&["草藥師", "草藥師學徒"]))
        .flatten();
    let porter_id = (action == "work_dock" || action == "sell_specialty")
        .then(|| find_cp_roles(&["碼頭苦力", "港務記籌手"]))
        .flatten();
    let guard_id = (action == "declare_found_bag")
        .then(|| find_cp_roles(&["巡城衛兵", "關哨衛兵", "檢疫衛兵"]))
        .flatten();

    match action {
        "work_dock" => {
            if node.kind != "dock" {
                return Err("需在港務碼頭才能幫忙搬卸靠岸船貨。".to_string());
            }
            check_quote(4)?;
            super::live_tide::settle_work_dock_action(
                doc,
                delta,
                clock,
                &node_id,
                &aid,
                porter_id.as_deref(),
            )
        }
        "forge_help" => {
            if !node.name.contains("鍛鐵") && !node.name.contains("鐵匠") {
                return Err("需在鐵匠鋪爐邊才能幫忙拉風箱打下手。".to_string());
            }
            if clock.is_night {
                return Err("夜深鐵匠鋪已封爐落栓打烊，待破曉後晨鐘二響再開爐吧。".to_string());
            }
            let Some(ref sid) = smith_id else {
                return Err("石爐壓著微溫灰炭，師傅與學徒此刻不在爐邊。".to_string());
            };
            if super::live_commodities::is_smithy_forge_cold(delta, &node_id) {
                return Err(
                    "鍛爐裡的精煤與木炭已燒盡熄火，師傅正高價收購北林精煤、木炭或乾木重開爐火！"
                        .to_string(),
                );
            }
            check_quote(5)?;
            super::live_tide::check_forge_shift_eligible(delta, clock, &aid)?;
            let used_fuel =
                super::live_commodities::consume_smithy_fuel_one(delta, Some(sid), &node_id);
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "forge_help",
                sid,
                &aid,
                5,
                "鐵匠鋪拉風箱工錢".to_string(),
            ) {
                if let Some((holder, item)) = used_fuel {
                    super::live_commodities::restore_shop_stock_one(delta, &holder, &node_id, item);
                }
                return Err("鐵匠鋪手頭零錢暫時周轉不開。".to_string());
            }
            super::live_tide::record_forge_shift_claimed(delta, clock, &aid);
            record_mutual_trade_evidence(delta, clock.game_days, &aid, sid, "鐵匠鋪拉風箱工錢");
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.fatigue = (a.fatigue + 14).min(95);
            a.bound_weapon.grip_familiarity = a.bound_weapon.grip_familiarity.saturating_add(4);
            a.mind.stm.today_goal_progressed = true;
            let lang_note =
                add_language_exposure(a, clock.game_days, "dwarven", 6).unwrap_or_default();
            a.mark_mutated();
            let msg = format!(
                "你守在炙熱石爐旁拉動牛皮風箱、協助敲平粗鐵胚，掌中握錘愈發順手，師傅數了 5 枚銅幣遞給你。{}",
                lang_note
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "repair_weapon" => {
            if !node.name.contains("鍛鐵") && !node.name.contains("鐵匠") {
                return Err("需在鐵匠鋪內才能請師傅修補兵刃。".to_string());
            }
            if clock.is_night {
                return Err("夜深鐵匠鋪已封爐落栓打烊，待破曉後晨鐘二響再開爐吧。".to_string());
            }
            let Some(ref sid) = smith_id else {
                return Err(
                    "石爐壓著微溫灰炭，師傅與學徒此刻不在爐邊，暫無法開爐修刃。".to_string()
                );
            };
            if super::live_commodities::is_smithy_forge_cold(delta, &node_id) {
                return Err(
                    "鍛爐裡的精煤與木炭已燒盡熄火，師傅正高價收購北林精煤、木炭或乾木重開爐火！"
                        .to_string(),
                );
            }
            let price = super::live_commodities::quote_weapon_repair_copper(delta, &node_id);
            check_quote(price)?;
            let need_iron = delta
                .actors
                .get(&aid)
                .is_some_and(|a| a.bound_weapon.wear_stage >= 2);
            let used_fuel =
                super::live_commodities::consume_smithy_fuel_one(delta, Some(sid), &node_id);
            let used_iron = if need_iron {
                super::live_commodities::consume_shop_stock_one(
                    delta,
                    Some(sid),
                    &node_id,
                    &["iron_ore"],
                )
            } else {
                None
            };
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "repair_weapon",
                &aid,
                sid,
                price,
                "鐵匠鋪修補兵刃工資".to_string(),
            ) {
                if let Some((holder, item)) = used_fuel {
                    super::live_commodities::restore_shop_stock_one(delta, &holder, &node_id, item);
                }
                if let Some((holder, item)) = used_iron {
                    super::live_commodities::restore_shop_stock_one(delta, &holder, &node_id, item);
                }
                return Err(format!(
                    "修補隨身器物需要 {} 枚銅幣，你懷裡銅板還不夠。",
                    price
                ));
            }
            record_mutual_trade_evidence(delta, clock.game_days, &aid, sid, "鐵匠鋪修補兵刃工資");
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.bound_weapon.wear_stage = 0;
            a.mark_mutated();
            let msg = format!(
                "匠師添炭開爐接過你的【{}】燒紅校直、重新加固木柄，收下 {} 枚銅幣，錘刃恢復完好如初。",
                a.bound_weapon.name, price
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "engrave_rune" => {
            if !node.name.contains("鍛鐵") && !node.name.contains("鐵匠") {
                return Err("需在鐵匠鋪石爐旁才能鑲刻原石鐵紋。".to_string());
            }
            if clock.is_night {
                return Err("夜深鐵匠鋪已封爐落栓打烊，待破曉後晨鐘二響再開爐吧。".to_string());
            }
            let Some(ref sid) = smith_id else {
                return Err(
                    "石爐壓著微溫灰炭，師傅與學徒此刻不在爐邊，暫無法鑲刻原石。".to_string()
                );
            };
            if super::live_commodities::is_smithy_forge_cold(delta, &node_id) {
                return Err(
                    "鍛爐裡的精煤與木炭已燒盡熄火，師傅正高價收購北林精煤、木炭或乾木重開爐火！"
                        .to_string(),
                );
            }
            let price = super::live_commodities::quote_engrave_rune_copper(delta, &node_id);
            check_quote(price)?;
            let (has_stone, already_runed, pname, wname) = {
                let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
                (
                    a.backpack
                        .iter()
                        .any(|i| i.slug == "rune_stone" && i.count > 0 && i.bound_owner.is_none()),
                    a.bound_weapon.rune_mark.is_some(),
                    a.display_name.clone(),
                    a.bound_weapon.name.clone(),
                )
            };
            if already_runed {
                return Err("你掌中兵刃已刻有符紋，無須重複鑲刻損耗原石。".to_string());
            }
            if !has_stone {
                return Err("背囊裡沒有【古符文原石】，無法鑲刻鐵紋。".to_string());
            }
            let used_fuel =
                super::live_commodities::consume_smithy_fuel_one(delta, Some(sid), &node_id);
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "engrave_rune",
                &aid,
                sid,
                price,
                "鑲刻古符文原石鐵紋".to_string(),
            ) {
                if let Some((holder, item)) = used_fuel {
                    super::live_commodities::restore_shop_stock_one(delta, &holder, &node_id, item);
                }
                return Err(format!("鑲刻工資需要 {} 枚銅幣。", price));
            }
            record_mutual_trade_evidence(delta, clock.game_days, &aid, sid, "鑲刻古符文原石鐵紋");
            {
                let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                let _ = take_carry_item(&mut a.backpack, "rune_stone", 1);
                a.bound_weapon.rune_mark = Some("穩固鐵紋".to_string());
                a.bound_weapon.wear_stage = 0;
                a.mind.stm.today_goal_progressed = true;
                a.mark_mutated();
            }
            let _ = super::phase_and_vessels::record_craft_work_and_check_phase(
                doc,
                delta,
                clock,
                &aid,
                "engrave_rune",
                "forge",
                &node_id,
            );
            broadcast_scene_event(
                doc,
                delta,
                clock,
                &node_id,
                &aid,
                &pname,
                EventPublicity::PublicLoud,
                Some("harbor_trade"),
                74,
                48,
                ValueAxis::CraftPride,
                "engrave_rune",
                &format!("{}取來古符文原石請鐵匠鍛入【{}】", pname, wname),
            );
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let msg = format!(
                "匠師以重錘將溫潤原石鍛入【{}】脊部，鏨出細密沉穩的鐵紋——此後揮擊重甲異獸將格外沉猛且不易崩口！",
                a.bound_weapon.name
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "redeem_voucher" => {
            if node.kind != "shop" || !node.name.contains("酒館") {
                return Err("請在一樓酒館櫃台前出示船東抵宿券。".to_string());
            }
            if innkeeper_id.is_none() {
                return Err("櫃台後此刻暫無人看顧房牌，稍候掌櫃或學徒回來即可換領。".to_string());
            }
            {
                let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                if take_carry_item(&mut a.backpack, "lodging_voucher", 1).is_none() {
                    return Err("你身上並沒有船東抵宿券。".to_string());
                }
                a.inn_nights_left += 2;
                a.mark_mutated();
            }
            let seed_now = delta.actors.get(&aid).map_or(1, |a| a.soul_seed.max(1));
            let assigned = super::ongoing_activity::assign_numbered_inn_room(
                doc,
                delta,
                node.cell_id,
                seed_now,
            );
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let room_note = if let Some((rid, rname)) = assigned {
                a.assigned_inn_room = Some(rid);
                rname.split('・').last().unwrap_or(&rname).to_string()
            } else {
                "二樓客房".to_string()
            };
            let msg = format!(
                "櫃台核對完抵宿券上的船東印記，遞來二樓【{}】木牌：「房間替你留好兩宿，隨時能上樓推門歇息或把重貨鎖進床頭木箱。」",
                room_note
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "rent_inn" => {
            if node.kind != "shop" || !node.name.contains("酒館") {
                return Err("需在一樓酒館櫃台才能付錢投宿。".to_string());
            }
            let Some(ref ik) = innkeeper_id else {
                return Err(
                    "櫃台後此刻暫無人看顧房牌，稍候掌櫃或值夜學徒回來即可投宿。".to_string()
                );
            };
            check_quote(3)?;
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "rent_inn",
                &aid,
                ik,
                3,
                "續租二樓閣樓客房一宿".to_string(),
            ) {
                return Err("投宿一晚需要 3 枚銅幣。".to_string());
            }
            record_mutual_trade_evidence(delta, clock.game_days, &aid, ik, "續租客房一宿");
            let seed_now = delta.actors.get(&aid).map_or(1, |a| a.soul_seed.max(1));
            let assigned = super::ongoing_activity::assign_numbered_inn_room(
                doc,
                delta,
                node.cell_id,
                seed_now,
            );
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.inn_nights_left += 1;
            let room_note = if let Some((rid, rname)) = assigned {
                a.assigned_inn_room = Some(rid);
                format!(
                    "（分得二樓【{}】木牌）",
                    rname.split('・').last().unwrap_or(&rname)
                )
            } else {
                String::new()
            };
            a.mark_mutated();
            let msg = format!(
                "你付了 3 枚銅幣續訂一宿二樓獨立客房{}，可由樓梯上樓進房安歇與使用床頭厚木箱。",
                room_note
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "buy_ration" => {
            if node.kind != "shop" || !node.name.contains("酒館") {
                return Err("需在一樓酒館才能購買黑麥乾糧。".to_string());
            }
            let price = super::live_commodities::quote_tavern_ration_copper(
                doc,
                delta,
                node.cell_id,
                &node_id,
            );
            check_quote(price)?;
            let is_self_basket = explicit_cp_tok == Some("self_basket");
            let any_tavern_staff_present = find_same_node_counterparty(
                delta,
                &aid,
                &node_id,
                clock.real_unix,
                &["酒館掌櫃", "酒館掌櫃學徒"],
                None,
            )
            .is_some();
            if is_self_basket && any_tavern_staff_present {
                return Err("掌櫃或學徒正在櫃台當值，請直接向櫃台購買乾糧。".to_string());
            }
            if !is_self_basket && innkeeper_id.is_none() {
                return Err(
                    "掌櫃與學徒此刻暫不在櫃台後；若竹簍尚有存糧，可由櫃頭乾糧竹簍投幣自取。"
                        .to_string(),
                );
            }
            let (_, keeper_stock_id) =
                super::household_economy::find_town_tavern_and_keeper(doc, delta, node.cell_id);
            let Some(ref kid) = keeper_stock_id else {
                return Err("酒館糧櫃尚未備妥。".to_string());
            };
            if !super::household_economy::try_consume_tavern_stock_one(delta, kid, &node_id) {
                return Err(
                    "今日酒館竹籃裡的黑麥乾糧剛被買空，得等明日農漁戶送來新糧。".to_string()
                );
            }
            let payee = innkeeper_id.as_deref().unwrap_or(kid.as_str());
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "buy_ration",
                &aid,
                payee,
                price,
                if is_self_basket {
                    "酒館竹簍投幣自取黑麥乾糧".to_string()
                } else {
                    "購買酒館黑麥乾糧".to_string()
                },
            ) {
                super::household_economy::restore_tavern_stock_one(delta, kid, &node_id);
                return Err(format!("一份黑麥乾糧現價需要 {} 枚銅幣。", price));
            }
            if let Some(ref ik) = innkeeper_id {
                record_mutual_trade_evidence(delta, clock.game_days, &aid, ik, "購買酒館黑麥乾糧");
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            add_carry_item(
                &mut a.backpack,
                CarryItem {
                    slug: "rye_bread".to_string(),
                    name: "黑麥乾糧".to_string(),
                    count: 1,
                    weight_units: 1,
                    ..CarryItem::default()
                },
            );
            a.mark_mutated();
            let msg = if is_self_basket {
                format!(
                    "你往櫃頭誠實木筒裡投入 {} 枚銅幣，從竹簍取走一包烘得紮實的【黑麥乾糧】。",
                    price
                )
            } else {
                format!(
                    "你花 {} 枚銅幣向櫃台買下一包烘得紮實的【黑麥乾糧】，遠行充飢或遇上灰耳斥候都派得上用場。",
                    price
                )
            };
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "eat_ration" => {
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            if take_carry_item(&mut a.backpack, "rye_bread", 1).is_none() {
                return Err("背囊裡沒有黑麥乾糧可吃。".to_string());
            }
            a.fatigue = a.fatigue.saturating_sub(28);
            a.mind.stm.affect.hunger = a.mind.stm.affect.hunger.saturating_sub(40);
            a.mark_mutated();
            let msg = "你就著水壺嚼完一塊黑麥乾糧，腹中暖意升起，雙腿氣力恢復不少。".to_string();
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "buy_rescue_pact" => {
            let at_public_board =
                node.kind == "plaza" || (node.kind == "shop" && node.name.contains("酒館"));
            if !at_public_board {
                return Err("需在市集廣場或酒館的公櫃掛牌木欄前才能簽立逾時搜救契。".to_string());
            }
            check_quote(5)?;
            if delta.actors.get(&aid).is_some_and(|a| a.rescue_pact_active) {
                return Ok("你已簽有生效中的逾時搜救契，無須重複花錢。".to_string());
            }
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "buy_rescue_pact",
                &aid,
                "municipal_treasury",
                5,
                "公櫃簽立逾時搜救契".to_string(),
            ) {
                return Err("簽立逾時搜救契需要 5 枚銅幣。".to_string());
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.rescue_pact_active = true;
            a.mark_mutated();
            let msg = "你往公櫃錢匣投入 5 枚銅幣並在巡防木牌簽下行蹤——萬一在荒野或坑道力竭昏厥，巡林隊會把你直接抬回草藥堂敷藥。"
                .to_string();
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "rest_bed" => {
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let in_owned = a.owned_house_node.as_deref() == Some(node_id.as_str());
            if node.kind != "inn_room" && !in_owned {
                return Err("需在二樓宿房或自家屋舍內才能寬衣安睡。".to_string());
            }
            if node.kind == "inn_room" && a.inn_nights_left == 0 {
                return Err("你尚未向樓下酒館掌櫃換取或租下宿房木牌。".to_string());
            }
            if node.kind == "inn_room" {
                a.inn_nights_left -= 1;
            }
            a.fatigue = 0;
            let innate = derive_innate_persona(a.soul_seed.max(1));
            let cash_short = a.copper < 3;
            a.mind
                .run_sleep_consolidation(&innate, &mut a.drift, clock.game_days, cash_short);
            let _ =
                super::world_weave_bio::refresh_actor_biography_if_needed(doc, a, clock.game_days);
            a.mark_mutated();
            let msg =
                "你在乾燥溫暖的木床上沉沉睡了一覺，白日見聞在夢裡沉澱清晰，渾身疲乏盡數消散。"
                    .to_string();
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "store_item" => {
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let in_owned = a.owned_house_node.as_deref() == Some(node_id.as_str());
            if node.kind != "inn_room" && !in_owned {
                return Err("需在二樓宿房或自家木箱旁才能存放物品。".to_string());
            }
            if node.kind == "inn_room" && a.inn_nights_left == 0 {
                return Err(
                    "需先向樓下酒館掌櫃換取或租下宿房木牌，才能開啟床頭厚木箱。".to_string()
                );
            }
            let slug = param.trim();
            if slug == "lodging_voucher"
                || a.backpack
                    .iter()
                    .any(|i| i.slug == slug && i.bound_owner.is_some())
            {
                return Err(
                    "隨身記名憑據、借領工具與待送家書須隨身保管，不可鎖入木箱。".to_string()
                );
            }
            let Some(item) = take_carry_item(&mut a.backpack, slug, 1) else {
                return Err("背囊裡找不到該件物品。".to_string());
            };
            let iname = item.name.clone();
            let box_vec = a.inn_lockers.entry(node_id).or_default();
            add_carry_item(box_vec, item);
            a.mark_mutated();
            let msg = format!("你將【{}】鎖進床頭厚木箱妥善保管。", iname);
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "take_item" => {
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let in_owned = a.owned_house_node.as_deref() == Some(node_id.as_str());
            if node.kind != "inn_room" && !in_owned {
                return Err("需在二樓宿房或自家木箱旁才能取出物品。".to_string());
            }
            if node.kind == "inn_room" && a.inn_nights_left == 0 {
                return Err(
                    "需先向樓下酒館掌櫃換取或租下宿房木牌，才能開啟床頭厚木箱。".to_string()
                );
            }
            let slug = param.trim();
            let taken = a
                .inn_lockers
                .get_mut(&node_id)
                .and_then(|v| take_carry_item(v, slug, 1))
                .or_else(|| take_carry_item(&mut a.inn_locker, slug, 1));
            let Some(item) = taken else {
                return Err("這隻木箱裡沒有這件物品。".to_string());
            };
            let iname = item.name.clone();
            add_carry_item(&mut a.backpack, item);
            a.mark_mutated();
            let msg = format!("你打開銅鎖，從厚木箱取出【{}】收進隨身背囊。", iname);
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "bandage_wound" => {
            if !node.name.contains("草藥") {
                return Err("需在苦艾草藥堂才能請草藥師敷藥包紮。".to_string());
            }
            if clock.is_night {
                return Err("夜深草藥堂已落栓打烊，待破曉開門再來敷藥吧。".to_string());
            }
            let Some(ref hid) = herb_id else {
                return Err("藥臼旁暫無人看顧，草藥師與學徒此刻不在堂內。".to_string());
            };
            if delta
                .actors
                .get(&aid)
                .is_some_and(|a| a.wound_part.is_none())
            {
                return Ok("草藥師端詳了你的氣色：「筋骨完好無損，不用白花這筆藥錢。」".to_string());
            }
            if super::live_commodities::is_apothecary_out_of_herbs(delta, &node_id) {
                return Err(
                    "藥堂陶罐裡的苦艾與止血草已用罄，草藥師正急收山林草藥入臼熬膏！".to_string(),
                );
            }
            let price = super::live_commodities::quote_bandage_copper(delta, &node_id);
            check_quote(price)?;
            let used_herb =
                super::live_commodities::consume_apothecary_herb_one(delta, Some(hid), &node_id);
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "bandage_wound",
                &aid,
                hid,
                price,
                "草藥堂敷藥包紮傷處".to_string(),
            ) {
                if let Some((holder, item)) = used_herb {
                    super::live_commodities::restore_shop_stock_one(delta, &holder, &node_id, item);
                }
                return Err(format!("敷藥包紮需要 {} 枚銅幣。", price));
            }
            record_mutual_trade_evidence(delta, clock.game_days, &aid, hid, "草藥堂敷藥包紮傷處");
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let old_w = a.wound_part.take().unwrap_or_else(|| "舊傷".to_string());
            a.severe_wrist_warning = false;
            a.mark_mutated();
            let msg = format!(
                "堂內醫者取來苦艾煎膏替你敷妥【{}】並纏緊亞麻繃帶，收下 {} 枚銅幣，瘀痛頓時消減大半。",
                old_w, price
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "sell_specialty" => super::cell_resources::settle_sell_specialty_action(
            doc,
            delta,
            clock,
            &aid,
            node,
            param,
            smith_id.as_deref(),
            herb_id.as_deref(),
            porter_id.as_deref(),
            record_mutual_trade_evidence,
        ),
        "board_carriage_west"
        | "board_carriage_east"
        | "carriage_next"
        | "carriage_alight"
        | "board_transit"
        | "transit_alight_next" => super::transit_and_cargo::apply_carriage_action(
            doc,
            delta,
            clock,
            &aid,
            node,
            coach_id.as_deref(),
            action,
            param,
        ),
        "pickup_dropped_bag" => {
            let bag_opt = delta
                .dropped_loot_by_node
                .get_mut(&node_id)
                .and_then(|bags| {
                    if let Some(own_idx) = bags.iter().position(|b| {
                        b.owner_id == aid
                            || ctrl_opt.as_deref().is_some_and(|cid| b.owner_id == cid)
                    }) {
                        Some(bags.remove(own_idx))
                    } else {
                        bags.pop()
                    }
                });
            if delta
                .dropped_loot_by_node
                .get(&node_id)
                .is_some_and(Vec::is_empty)
            {
                delta.dropped_loot_by_node.remove(&node_id);
            }
            let Some(bag) = bag_opt else {
                return Err("此處地面沒有遺落的行囊。".to_string());
            };
            let is_own =
                bag.owner_id == aid || ctrl_opt.as_deref().is_some_and(|cid| bag.owner_id == cid);
            let mut restored_haul: Option<super::types::OngoingActivityState> = None;
            let mut has_non_dock_item = false;
            {
                let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                a.copper += bag.copper;
                for mut it in bag.items {
                    if it.slug == "dock_cargo_crate" {
                        if a.ongoing_activity.is_none() && restored_haul.is_none() {
                            restored_haul =
                                super::ongoing_activity::reconstruct_dock_haul_from_crate(
                                    doc, &node_id, &it,
                                );
                        }
                    } else {
                        has_non_dock_item = true;
                        if !is_own {
                            it.stolen_from = Some(bag.owner_id.clone());
                        }
                    }
                    add_carry_item(&mut a.backpack, it);
                }
                if let Some(st) = restored_haul.clone() {
                    a.ongoing_activity = Some(st);
                }
                a.mark_mutated();
            }
            delta.extras_dirty = true;
            if bag.copper > 0 {
                delta.push_ledger_entry(
                    clock.game_days,
                    clock.bell_index,
                    "pickup_loot_escrow",
                    "dropped_loot_escrow",
                    &aid,
                    bag.copper,
                    format!("於{}拾起遺落行囊內銅幣", node.name),
                );
            }
            if !is_own && (has_non_dock_item || bag.copper > 0) {
                delta.pickup_ledger.push(PickupRecord {
                    picker_id: aid.clone(),
                    original_owner: bag.owner_id.clone(),
                    node_id: node_id.clone(),
                    summary: format!("於{}拾獲遺落行囊（含 {} 枚銅幣）", node.name, bag.copper),
                    declared_to_guard: false,
                });
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let msg = if let Some(st) = restored_haul {
                format!(
                    "你彎腰重新扛起地上的港務封蠟貨箱（{}），送抵指定店鋪即可領取腳錢，或隨時放下。",
                    st.label_zh
                )
            } else if is_own {
                format!(
                    "你找回了自己先前遺落的行囊，清點取回 {} 枚銅幣與隨身貨品！",
                    bag.copper
                )
            } else {
                format!(
                    "你拾起前人遺落的行囊（內含 {} 枚銅幣與貨品）；若帶回城門向衛兵申報招領可光明正大留用，私下銷贓則會被壓價。",
                    bag.copper
                )
            };
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "declare_found_bag" => {
            if node.kind != "gate" && node.kind != "plaza" {
                return Err("請向城門當值衛兵或市集廣場公櫃木牌申報拾遺。".to_string());
            }
            if node.kind == "gate" && guard_id.is_none() {
                return Err(
                    "城門哨口此刻沒有衛兵在場核對章記，可稍候或改赴市集廣場公櫃登記。".to_string(),
                );
            }
            let mut declared_any = false;
            for rec in &mut delta.pickup_ledger {
                let match_picker = rec.picker_id == aid
                    || ctrl_opt.as_deref().is_some_and(|cid| rec.picker_id == cid);
                if match_picker && !rec.declared_to_guard {
                    rec.declared_to_guard = true;
                    declared_any = true;
                }
            }
            let pname = {
                let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                for it in &mut a.backpack {
                    if it.stolen_from.is_some() {
                        it.stolen_from = None;
                        declared_any = true;
                    }
                }
                if !declared_any {
                    return Ok("衛兵翻看登記簿：「你手頭沒有待申報的拾遺物件。」".to_string());
                }
                a.mark_mutated();
                a.display_name.clone()
            };
            broadcast_scene_event(
                doc,
                delta,
                clock,
                &node_id,
                &aid,
                &pname,
                EventPublicity::PublicLoud,
                Some("harbor_trade"),
                60,
                35,
                ValueAxis::PactIntegrity,
                "declare_found_bag",
                &format!("{}主動向衛兵報備拾遺物件，行事光明磊落", pname),
            );
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let msg = "衛兵核對完拾遺時日並蓋上港務章記：「既已主動報備，這批無主貨品便歸你正當處置了。」"
                .to_string();
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "buy_vacant_house" => {
            if node.kind != "house_vacant" {
                return Err("需站在掛牌待售的空屋內才能辦理購屋。".to_string());
            }
            if delta
                .actors
                .get(&aid)
                .is_some_and(|a| a.owned_house_node.is_some())
            {
                return Err("你在島上已置有一處安身屋舍，無須重複買屋。".to_string());
            }
            if delta.sold_houses.contains_key(&node_id) {
                return Err("這棟石木屋舍已有人買下房契了。".to_string());
            }
            if let Some(sq) = super::live_wilds::has_squatter_at_node(delta, &node_id) {
                return Err(format!(
                    "屋內斷牆正被【{}】盤踞作窩，先將牠們趕出清場再來辦理房契吧！",
                    sq.foe_name_zh
                ));
            }
            let base_price = doc
                .households
                .iter()
                .find(|h| h.node_id == node_id && h.vacant_for_sale)
                .map_or(360, |h| h.price_copper);
            let price = if super::live_land::house_is_ruined(delta, &node_id) {
                (base_price * 65) / 100
            } else {
                base_price
            };
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "buy_vacant_house",
                &aid,
                "municipal_treasury",
                price,
                format!("購置屋舍【{}】房契", node.name),
            ) {
                return Err(format!(
                    "買下這棟石木屋舍契紙需要 {} 枚銅幣，你目前手頭還差一些。",
                    price
                ));
            }
            delta.sold_houses.insert(node_id.clone(), aid.clone());
            delta.live_land.house_integrity.remove(&node_id);
            let pname = {
                let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                a.owned_house_node = Some(node_id.clone());
                a.home_node = node_id.clone();
                a.mind.stm.today_goal_progressed = true;
                a.mark_mutated();
                a.display_name.clone()
            };
            super::living_chronicler::record_vacant_house_bought(
                doc, delta, clock, &aid, &node_id, price,
            );
            broadcast_scene_event(
                doc,
                delta,
                clock,
                &node_id,
                &aid,
                &pname,
                EventPublicity::PublicLoud,
                Some("harbor_trade"),
                82,
                55,
                ValueAxis::KinshipLoyalty,
                "buy_house",
                &format!("{}交納銅幣買下【{}】正式落戶成為街坊鄰居", pname, node.name),
            );
            super::map_memory::record_local_witness_at_cur_node(doc, delta, &aid, clock.game_days);
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let msg = format!(
                "你交納 {} 枚銅幣領下市政廳房契，【{}】從此成為你在島上的安身居所，隨時可回屋安睡與儲物！",
                price, node.name
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "inquire_ferry" => {
            if !node_id.starts_with("oi:west_strait_ferry") {
                return Err("需在西峽渡口碼頭才能查看跨海公營渡輪船班。".to_string());
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let msg = if a.copper >= 25 {
                "西峽渡口掌纜官驗看過你懷裡的 25 枚銅幣，指著雙桅渡輪笑道：「船資齊備！待下班順風潮起，隨時能登船橫渡西海峽前往本土王國。」".to_string()
            } else {
                format!(
                    "雙桅公營渡輪正停在防波堤畔裝卸木桶——跨海船票定價 25 枚銅幣（你目前存有 {} 枚銅幣），也可在碼頭扛活或販售山海特產攢足船資。",
                    a.copper
                )
            };
            push_breath(a, msg.clone());
            a.mark_mutated();
            Ok(msg)
        }
        _ => {
            if let Some(res) = super::mesh_expedition::apply_expedition_action(
                doc, delta, clock, &aid, node, action,
            ) {
                return res;
            }
            if let Some(res) = super::live_land::try_apply_live_land_action(
                doc, delta, clock, &aid, node, action, param,
            ) {
                if res.is_ok() {
                    super::map_memory::record_local_witness_at_cur_node(
                        doc,
                        delta,
                        &aid,
                        clock.game_days,
                    );
                }
                return res;
            }
            if let Some(res) = super::live_wilds::try_apply_live_wilds_action(
                doc, delta, clock, &aid, node, action, param,
            ) {
                if res.is_ok() {
                    super::map_memory::record_local_witness_at_cur_node(
                        doc,
                        delta,
                        &aid,
                        clock.game_days,
                    );
                }
                return res;
            }
            if let Some(res) =
                super::live_lifecycle::try_apply_live_lifecycle_action(delta, &aid, node, action)
            {
                return res;
            }
            if action == "intervene_vignette" {
                return super::living_presence::apply_vignette_intervene(
                    doc, delta, clock, &aid, param,
                );
            }
            if action == "set_offline_policy" {
                return super::offline_autonomy::set_player_offline_policy(
                    delta, clock, &aid, param,
                );
            }
            if action == "dismiss_offline_recap" {
                return super::offline_autonomy::dismiss_player_offline_recap(delta, &aid);
            }
            if let Some(res) = super::ongoing_activity::try_dispatch_embodied_action(
                doc,
                delta,
                clock,
                &aid,
                node,
                smith_id.as_deref(),
                action,
            ) {
                return res;
            }
            let ext_res = super::unified_kernel::dispatch_extended_shared_verb(
                doc, delta, clock, &aid, action, param,
            );
            if ext_res.is_ok() {
                super::map_memory::record_local_witness_at_cur_node(
                    doc,
                    delta,
                    &aid,
                    clock.game_days,
                );
            }
            ext_res
        }
    }
}

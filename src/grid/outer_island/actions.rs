//! 全服單一 `UnifiedMindBody` 城鎮商鋪生計、二樓木箱、拾遺申報與複式銅幣守恆流轉（零靜態 `doc.residents` 依賴）。

use super::clock_and_lang::SolarClockSnapshot;
use super::explore_and_combat::resolve_skirmish_turn;
use super::mind::EventPublicity;
use super::movement::{
    add_carry_item, add_language_exposure, apply_carriage_action, push_breath, take_carry_item,
};
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

/// 優先自當前場景 `node_occupants` $O(1)$ 查找在場行當人物，其次僅查同聚落公共街區之 `node_occupants`（嚴守 $O(1)$ 城鎮局域查找，零全服 $O(N)$ 掃描、零 `doc.residents` 依賴！）
fn find_counterparty_actor(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    self_aid: &str,
    node_id: &str,
    cell_id: u32,
    real_unix: i64,
    role_name: &str,
) -> Option<String> {
    let can_transact = |a: &super::types::UnifiedMindBody| -> bool {
        a.alive
            && a.setup_done
            && a.role == role_name
            && (a.controller_id.is_none() || real_unix - a.last_active_unix >= 1800)
    };
    if let Some(occ) = delta.node_occupants.get(node_id)
        && let Some(found) = occ
            .iter()
            .find(|&oid| oid != self_aid && delta.actors.get(oid).is_some_and(can_transact))
    {
        return Some(found.clone());
    }
    for (nid, nd) in &doc.nodes {
        if nd.cell_id == cell_id
            && nid != node_id
            && let Some(occ) = delta.node_occupants.get(nid)
            && let Some(found) = occ
                .iter()
                .find(|&oid| oid != self_aid && delta.actors.get(oid).is_some_and(can_transact))
        {
            return Some(found.clone());
        }
    }
    None
}

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
        a.last_active_unix = clock.real_unix;
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
    if in_transit && action != "carriage_next" && action != "carriage_alight" {
        return Err("雙馬篷車正在大路上奔馳，需下車後才能辦這件事。".to_string());
    }

    let Some(node) = doc.nodes.get(&node_id) else {
        return Err("四周霧氣迷濛，暫時辨不清方位。".to_string());
    };

    let smith_id = match action {
        "forge_help" | "repair_weapon" | "engrave_rune" | "sell_specialty" => {
            find_counterparty_actor(
                doc,
                delta,
                &aid,
                &node_id,
                node.cell_id,
                clock.real_unix,
                "鍛鐵匠",
            )
        }
        _ => None,
    };
    let innkeeper_id = match action {
        "redeem_voucher" | "rent_inn" | "buy_ration" => find_counterparty_actor(
            doc,
            delta,
            &aid,
            &node_id,
            node.cell_id,
            clock.real_unix,
            "酒館掌櫃",
        ),
        _ => None,
    };
    let coach_id = if action.starts_with("board_carriage") || action.starts_with("carriage_") {
        find_counterparty_actor(
            doc,
            delta,
            &aid,
            &node_id,
            node.cell_id,
            clock.real_unix,
            "車行掌櫃",
        )
    } else {
        None
    };
    let herb_id = if action == "bandage_wound" || action == "sell_specialty" {
        find_counterparty_actor(
            doc,
            delta,
            &aid,
            &node_id,
            node.cell_id,
            clock.real_unix,
            "草藥師",
        )
    } else {
        None
    };
    let porter_id = if action == "work_dock" || action == "sell_specialty" {
        find_counterparty_actor(
            doc,
            delta,
            &aid,
            &node_id,
            node.cell_id,
            clock.real_unix,
            "碼頭苦力",
        )
    } else {
        None
    };

    match action {
        "work_dock" => {
            if node.kind != "dock" {
                return Err("需在港務碼頭才能幫忙扛貨賺取工錢。".to_string());
            }
            if delta.actors.get(&aid).is_some_and(|a| a.fatigue >= 85) {
                return Err("你雙肩痠沉氣喘吁吁，先吃口乾糧或回宿房歇息再扛重活吧。".to_string());
            }
            let paid_by_porter = if let Some(ref pid_npc) = porter_id {
                delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "work_dock",
                    pid_npc,
                    &aid,
                    4,
                    "港務碼頭扛貨工資".to_string(),
                )
            } else {
                false
            };
            if paid_by_porter {
                if let Some(ref pid_npc) = porter_id {
                    record_mutual_trade_evidence(
                        delta,
                        clock.game_days,
                        &aid,
                        pid_npc,
                        "港務碼頭扛貨工資",
                    );
                }
            } else if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "work_dock",
                "municipal_treasury",
                &aid,
                4,
                "港務碼頭扛貨工資".to_string(),
            ) {
                return Err("今日碼頭貨船已卸完，暫無零工可派。".to_string());
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.fatigue = (a.fatigue + 12).min(95);
            a.mind.stm.today_goal_progressed = true;
            a.mark_mutated();
            let msg = "你捲起衣袖幫著把幾隻鹽漬海獸皮箱扛上棧橋，領工點給你 4 枚沉甸甸的銅幣。"
                .to_string();
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "forge_help" => {
            if !node.name.contains("鍛鐵") && !node.name.contains("鐵匠") {
                return Err("需在鐵匠鋪或鐵匠街爐邊才能幫忙拉風箱打下手。".to_string());
            }
            if delta.actors.get(&aid).is_some_and(|a| a.fatigue >= 85) {
                return Err("你手臂發軟，歇口氣再拉風箱才不會失手燙傷。".to_string());
            }
            let Some(ref sid) = smith_id else {
                return Err("鐵匠師傅此刻不在爐邊。".to_string());
            };
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "forge_help",
                sid,
                &aid,
                5,
                "鐵匠鋪拉風箱工錢".to_string(),
            ) {
                return Err("鐵匠鋪手頭零錢暫時周轉不開。".to_string());
            }
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
            let payee = smith_id.as_deref().unwrap_or("municipal_treasury");
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "repair_weapon",
                &aid,
                payee,
                4,
                "鐵匠鋪修補兵刃工資".to_string(),
            ) {
                return Err("修補隨身器物需要 4 枚銅幣，你懷裡銅板還不夠。".to_string());
            }
            if let Some(ref sid) = smith_id {
                record_mutual_trade_evidence(
                    delta,
                    clock.game_days,
                    &aid,
                    sid,
                    "鐵匠鋪修補兵刃工資",
                );
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.bound_weapon.wear_stage = 0;
            a.mark_mutated();
            let msg = format!(
                "鐵匠接過你的【{}】燒紅校直、重新加固木柄，收下 4 枚銅幣，錘刃恢復完好如初。",
                a.bound_weapon.name
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "engrave_rune" => {
            if !node.name.contains("鍛鐵") && !node.name.contains("鐵匠") {
                return Err("需在鐵匠鋪石爐旁才能鑲刻原石鐵紋。".to_string());
            }
            let (has_stone, pname, wname) = {
                let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
                (
                    a.backpack
                        .iter()
                        .any(|i| i.slug == "rune_stone" && i.count > 0),
                    a.display_name.clone(),
                    a.bound_weapon.name.clone(),
                )
            };
            if !has_stone {
                return Err("背囊裡沒有【古符文原石】，無法鑲刻鐵紋。".to_string());
            }
            let payee = smith_id.as_deref().unwrap_or("municipal_treasury");
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "engrave_rune",
                &aid,
                payee,
                6,
                "鑲刻古符文原石鐵紋".to_string(),
            ) {
                return Err("鑲刻工資需要 6 枚銅幣。".to_string());
            }
            if let Some(ref sid) = smith_id {
                record_mutual_trade_evidence(
                    delta,
                    clock.game_days,
                    &aid,
                    sid,
                    "鑲刻古符文原石鐵紋",
                );
            }
            {
                let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                let _ = take_carry_item(&mut a.backpack, "rune_stone", 1);
                a.bound_weapon.rune_mark = Some("穩固鐵紋".to_string());
                a.bound_weapon.wear_stage = 0;
                a.mind.stm.today_goal_progressed = true;
                a.mark_mutated();
            }
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
                "鐵匠以重錘將溫潤原石鍛入【{}】脊部，鏨出細密沉穩的鐵紋——此後揮擊重甲異獸將格外沉猛且不易崩口！",
                a.bound_weapon.name
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "redeem_voucher" => {
            if !node.name.contains("酒館") {
                return Err("請在酒館櫃台前出示船東抵宿券。".to_string());
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            if take_carry_item(&mut a.backpack, "lodging_voucher", 1).is_none() {
                return Err("你身上並沒有船東抵宿券。".to_string());
            }
            a.inn_nights_left += 2;
            a.mark_mutated();
            let msg = "掌櫃核對完抵宿券上的船東印記，笑著遞來二樓閣樓木牌：「房間替你留好兩宿，隨時能上樓歇息或把重貨鎖進床頭木箱。」"
                .to_string();
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "rent_inn" => {
            if !node.name.contains("酒館") {
                return Err("需在酒館櫃台才能付錢投宿。".to_string());
            }
            let payee = innkeeper_id.as_deref().unwrap_or("municipal_treasury");
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "rent_inn",
                &aid,
                payee,
                3,
                "續租二樓閣樓客房一宿".to_string(),
            ) {
                return Err("投宿一晚需要 3 枚銅幣。".to_string());
            }
            if let Some(ref ik) = innkeeper_id {
                record_mutual_trade_evidence(
                    delta,
                    clock.game_days,
                    &aid,
                    ik,
                    "續租二樓閣樓客房一宿",
                );
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.inn_nights_left += 1;
            a.mark_mutated();
            let msg = "你付了 3 枚銅幣續訂一宿二樓閣樓客房，可由樓梯上樓安歇與使用床頭厚木箱。"
                .to_string();
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "buy_ration" => {
            if !node.name.contains("酒館") {
                return Err("需在酒館櫃台才能購買乾糧。".to_string());
            }
            let payee = innkeeper_id.as_deref().unwrap_or("municipal_treasury");
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "buy_ration",
                &aid,
                payee,
                2,
                "購買酒館黑麥乾糧".to_string(),
            ) {
                return Err("一份黑麥乾糧需要 2 枚銅幣。".to_string());
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
                    bound_owner: None,
                    stolen_from: None,
                    weight_units: 1,
                },
            );
            a.mark_mutated();
            let msg =
                "你花 2 枚銅幣買下一包烘得紮實的【黑麥乾糧】，遠行充飢或遇上灰耳斥候都派得上用場。"
                    .to_string();
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
            if !node.name.contains("酒館") {
                return Err("需在酒館櫃台才能簽立逾時搜救契。".to_string());
            }
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
                "簽立逾時搜救契".to_string(),
            ) {
                return Err("簽立逾時搜救契需要 5 枚銅幣。".to_string());
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.rescue_pact_active = true;
            a.mark_mutated();
            let msg = "你交出 5 枚銅幣在酒館木牌登記行蹤——萬一在荒野或坑道力竭昏厥，巡林隊會把你直接抬回草藥堂敷藥。"
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
            if slug == "lodging_voucher" {
                return Err("船東抵宿券是隨身住宿憑據，直接拿去樓下櫃台兌換即可。".to_string());
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
            if delta
                .actors
                .get(&aid)
                .is_some_and(|a| a.wound_part.is_none())
            {
                return Ok("草藥師端詳了你的氣色：「筋骨完好無損，不用白花這筆藥錢。」".to_string());
            }
            let payee = herb_id.as_deref().unwrap_or("municipal_treasury");
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "bandage_wound",
                &aid,
                payee,
                3,
                "草藥堂敷藥包紮傷處".to_string(),
            ) {
                return Err("敷藥包紮需要 3 枚銅幣。".to_string());
            }
            if let Some(ref hid) = herb_id {
                record_mutual_trade_evidence(
                    delta,
                    clock.game_days,
                    &aid,
                    hid,
                    "草藥堂敷藥包紮傷處",
                );
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let old_w = a.wound_part.take().unwrap_or_else(|| "舊傷".to_string());
            a.severe_wrist_warning = false;
            a.mark_mutated();
            let msg = format!(
                "草藥師以溫熱苦艾膏替你敷妥【{}】並纏緊亞麻繃帶，瘀痛頓時消減大半。",
                old_w
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "sell_specialty" => {
            if node.kind != "shop" && node.kind != "dock" && node.kind != "plaza" {
                return Err("需在商鋪、市集廣場或港務碼頭才能出售採集貨品。".to_string());
            }
            let sell_table = [
                ("rune_stone", 15_i64),
                ("spider_carapace", 12_i64),
                ("iron_ore", 5_i64),
                ("driftwood", 4_i64),
                ("herb_bundle", 4_i64),
            ];
            let chosen = {
                let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
                sell_table.iter().find_map(|(slug, price)| {
                    a.backpack
                        .iter()
                        .find(|i| i.slug == *slug && i.count > 0 && i.bound_owner.is_none())
                        .map(|i| {
                            (
                                (*slug).to_string(),
                                i.name.clone(),
                                *price,
                                i.stolen_from.is_some(),
                            )
                        })
                })
            };
            let Some((slug, iname, base_price, is_unclaimed)) = chosen else {
                return Err(
                    "背囊裡目前沒有可出售的山海貨品（如漂流木、止血草、粗鐵礦、巨蛛甲殼或原石）。"
                        .to_string(),
                );
            };
            let final_price = if is_unclaimed {
                (base_price / 2).max(1)
            } else {
                base_price
            };
            let buyer = if node.name.contains("草藥") {
                herb_id.as_deref().or(smith_id.as_deref())
            } else {
                smith_id
                    .as_deref()
                    .or(herb_id.as_deref())
                    .or(porter_id.as_deref())
            };
            let summary = format!("收購山海特產【{}】", iname);
            let paid_by_npc = if let Some(bid) = buyer {
                if delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "sell_specialty",
                    bid,
                    &aid,
                    final_price,
                    summary.clone(),
                ) {
                    record_mutual_trade_evidence(delta, clock.game_days, &aid, bid, &summary);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !paid_by_npc {
                let allow_public = node.kind == "dock" || node.kind == "plaza";
                if !allow_public
                    || !delta.transfer_copper(
                        clock.game_days,
                        clock.bell_index,
                        "sell_specialty",
                        "municipal_treasury",
                        &aid,
                        final_price,
                        summary,
                    )
                {
                    return Err("收貨商販手頭銅幣暫時不足。".to_string());
                }
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let _ = take_carry_item(&mut a.backpack, &slug, 1);
            a.mind.stm.today_goal_progressed = true;
            a.mark_mutated();
            let msg = format!(
                "你將【{}】過秤交予收貨行商，當面點收 {} 枚銅幣。",
                iname, final_price
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "board_carriage_west" | "board_carriage_east" | "carriage_next" | "carriage_alight" => {
            apply_carriage_action(doc, delta, clock, &aid, node, coach_id.as_deref(), action)
        }
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
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.copper += bag.copper;
            for mut it in bag.items {
                if !is_own {
                    it.stolen_from = Some(bag.owner_id.clone());
                }
                add_carry_item(&mut a.backpack, it);
            }
            a.mark_mutated();
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
            if !is_own {
                delta.pickup_ledger.push(PickupRecord {
                    picker_id: aid.clone(),
                    original_owner: bag.owner_id.clone(),
                    node_id: node_id.clone(),
                    summary: format!("於{}拾獲遺落行囊（含 {} 枚銅幣）", node.name, bag.copper),
                    declared_to_guard: false,
                });
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let msg = if is_own {
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
                return Err("請在城門衛兵或市集廣場公設木牌處申報拾遺。".to_string());
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
            let price = doc
                .households
                .iter()
                .find(|h| h.node_id == node_id && h.vacant_for_sale)
                .map_or(360, |h| h.price_copper);
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
            let pname = {
                let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                a.owned_house_node = Some(node_id.clone());
                a.home_node = node_id.clone();
                a.mind.stm.today_goal_progressed = true;
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
                82,
                55,
                ValueAxis::KinshipLoyalty,
                "buy_house",
                &format!("{}交納銅幣買下【{}】正式落戶成為街坊鄰居", pname, node.name),
            );
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
            Ok(msg)
        }
        _ => Err("此處暫無這項舉動。".to_string()),
    }
}

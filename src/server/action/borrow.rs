//! `do_action` — Borrow 分支：隨機機率的偷取/被發現。

use rand::Rng;

use crate::ai::decision_scorer::{NpcDecisionInput, NpcStanceChoice, score_npc_interaction_stance};
use crate::db::{
    self, FAV_BORROW_CAUGHT, FAV_BORROW_SUCCESS, add_to_inventory, adjust_favorability,
    get_disposition, get_entity_room, get_item_def, inventory_has_qty,
    pick_npc_trade_offer, remove_from_inventory,
};
use crate::entity::{Character, EntityKind};
use crate::event;
use crate::npc::npc_behavior_reaction_line;

use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::ActionResultMsg;

pub(super) fn do_entity_borrow(
    conn: &WsConnection,
    pid: &str,
    target_id: &str,
    target: &Character,
    now: i64,
) {
    let borrow_name = super::player_visible_target_name(pid, target);
    let pool_opt = crate::pg::pool();
    let player_room = get_entity_room(pid).unwrap_or_default();
    let room_title = db::get_room_name(&player_room).unwrap_or_else(|_| player_room.clone());

    let target_true_name = crate::store::resolve_npc_person_name(&target.id, &target.display_title, None);
    let match_name = if !target.display_title.is_empty() {
        if !target_true_name.is_empty()
            && target_true_name != "陌生人"
            && !target.display_title.contains(&target_true_name)
        {
            format!("{}・{}", target.display_title, target_true_name)
        } else {
            target.display_title.clone()
        }
    } else if !target_true_name.is_empty() && target_true_name != "陌生人" {
        target_true_name
    } else {
        target.id.clone()
    };
    let canon_item = canonical_or_venue_borrow_item(&match_name, &player_room);

    // 物歸原主檢查：若玩家行囊中已持有該場館或對象的可借器物，則執行歸還
    if let Some((c_id, c_name, _)) = canon_item {
        if let Ok(Some(player_ent)) = db::get_entity(pid)
            && inventory_has_qty(&player_ent.inventory, c_id, 1)
        {
            let _ = remove_from_inventory(pid, c_id, 1);
            let _ = add_to_inventory(target_id, c_id, 1);
            let vstate = crate::pg::living_world::get_vessel_state(pid, c_id);
            let (fav_delta, mut narrative) = if vstate.condition == crate::pg::living_world::VesselCondition::RolledEdge {
                crate::store::open_active_npc_obligation(target_id, "repair_tool", c_name);
                (
                    1,
                    format!(
                        "你將借用的「{c_name}」遞還給【{borrow_name}】。\n【{borrow_name}】接過一瞧，眉頭緊蹙：「你雖將『{c_name}』還了回來，可這刃口都捲缺成這樣……我還得送去鐵坊回火重修，下回借用可得惜物些！」"
                    ),
                )
            } else if vstate.condition == crate::pg::living_world::VesselCondition::Sharp && vstate.use_count >= 2 {
                let _ = crate::pg::living_world::record_mentor_imprint_progress(
                    pid,
                    &borrow_name,
                    "百工惜器訣",
                    "愛惜百工器物、用後推磨歸位，深得街坊匠人信重",
                    now,
                );
                (
                    8,
                    format!(
                        "你將借用的「{c_name}」擦拭推磨得雪亮，雙手歸還給【{borrow_name}】。\n【{borrow_name}】接過一瞧，雙眼一亮讚許道：「不僅完好歸還，連刃口都推磨得雪亮如新，真是惜物懂行之人！」"
                    ),
                )
            } else {
                (
                    5,
                    format!(
                        "你將借用的「{c_name}」擦拭妥當、完好無損地歸還給【{borrow_name}】。\n【{borrow_name}】欣然接過收好，溫和頷首道：「有借有還，果然是守信重諾之人，日後若有需要儘管開口。」"
                    ),
                )
            };
            let _ = adjust_favorability(target_id, pid, fav_delta);
            let _ = crate::npc::debt_interaction::settle_debt_for_actor(pid, 999);
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
                if fav_delta >= 5 { "return_sharp" } else { "return_damaged" },
                c_name,
                0,
                now,
            );
            if let Some(pool) = &pool_opt {
                let _ = crate::pg::living_world::resolve_obligations_by_kind(pool, pid, "repay_debt", now);
                let _ = crate::pg::living_world::record_relationship_delta(
                    pool,
                    target_id,
                    pid,
                    "helped",
                    1,
                    "borrow_returned",
                    now,
                );
                let _ = crate::pg::living_world::record_player_world_action(
                    pool,
                    pid,
                    "return_borrowed",
                    target_id,
                    &player_room,
                    "returned",
                    serde_json::json!({
                        "item_id": c_id,
                        "returned": true,
                    }),
                    now,
                );
            }
            if let Some((_pack_id, _step, ep_line)) = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(&room_title, c_name).or_else(|| crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(&player_room, c_name)) {
                narrative.push_str(&format!("\n{ep_line}"));
            }
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Borrow".into(),
                target_id: target.id.clone(),
                target_name: borrow_name,
                narrative,
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
            return;
        }
    }

    let personality = crate::db::get_personality_for_entity(target_id)
        .or_else(|| target.soul_seed.map(crate::db::expand_soul_seed_to_personality))
        .unwrap_or(crate::db::Personality {
            boldness: 0.5,
            sensitivity: 0.5,
            orderliness: 0.5,
        });

    let (rel, obligations, player_obligations) = if let Some(pool) = &pool_opt {
        let r = crate::pg::living_world::get_relationship_summary(pool, target_id, pid)
            .unwrap_or_default();
        let o = crate::pg::living_world::list_open_obligations_for_entity(pool, target_id)
            .unwrap_or_default();
        let po = crate::pg::living_world::list_open_obligations_for_entity(pool, pid)
            .unwrap_or_default();
        (r, o, po)
    } else {
        (
            crate::pg::living_world::RelationshipSummary::default(),
            Vec::new(),
            Vec::new(),
        )
    };

    let has_unreturned_borrow_from_target = player_obligations
        .iter()
        .any(|o| o.obligation_kind == "repay_debt" && o.subject_ref == target_id);
    if has_unreturned_borrow_from_target {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Borrow".into(),
            target_id: target.id.clone(),
            target_name: borrow_name.clone(),
            narrative: format!(
                "【{borrow_name}】搖了搖頭，溫言提醒道：「上次借你的器物還沒歸還呢，等物歸原主之後，有需要再借不遲。」"
            ),
            success: false,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }

    let obligation_descs: Vec<String> = obligations
        .iter()
        .map(|o| o.obligation_kind.clone())
        .collect();
    let current_activity = if target.current_activity.is_empty() {
        "佇立觀察四周"
    } else {
        &target.current_activity
    };

    let decision_input = NpcDecisionInput {
        npc_name: &borrow_name,
        boldness: personality.boldness,
        sensitivity: personality.sensitivity,
        orderliness: personality.orderliness,
        familiarity_score: rel.familiarity_score,
        balance_score: rel.balance_score,
        open_obligations: &obligation_descs,
        current_activity,
        action_kind: "Borrow",
        player_utterance: "暗中探手偷竊隨身物件",
    };
    let decision = score_npc_interaction_stance(&decision_input);
    let disp = get_disposition(target_id);

    // 熟識明借分支：若玩家已識名或關係良好且心緒不差，直接明借
    let is_acquainted_or_friendly = crate::store::player_knows_npc(pid, target_id)
        || rel.familiarity_score >= 1
        || rel.balance_score >= 1;
    if let Some((c_id, c_name, borrow_quote)) = canon_item {
        if is_acquainted_or_friendly && disp >= -35 && rel.balance_score >= -1 {
            let _ = remove_from_inventory(target_id, c_id, 1);
            let _ = add_to_inventory(pid, c_id, 1);
            let _ = event::append(now, pid, "borrow", target_id);
            if let Some(pool) = &pool_opt {
                let _ = crate::pg::living_world::open_obligation(
                    pool,
                    pid,
                    "repay_debt",
                    target_id,
                    Some(now + 7200),
                    "player_borrow",
                    now,
                );
                let _ = crate::pg::living_world::record_relationship_delta(
                    pool,
                    target_id,
                    pid,
                    "debt",
                    1,
                    "player_borrow",
                    now,
                );
                let _ = crate::pg::living_world::record_player_world_action(
                    pool,
                    pid,
                    "borrow",
                    target_id,
                    &player_room,
                    "success",
                    serde_json::json!({
                        "item_id": c_id,
                        "obligation": "repay_debt",
                        "due_in_secs": 7200,
                        "mode": "open_borrow",
                    }),
                    now,
                );
            }
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
                "borrow_item",
                c_name,
                0,
                now,
            );
            let mut narrative = format!(
                "你向【{borrow_name}】拱手開口商借「{c_name}」應急。\n【{borrow_name}】爽快取出一件「{c_name}」遞給你：{borrow_quote}"
            );
            if crate::pg::living_world::is_weapon_or_tool_item(c_id, c_name) {
                narrative.push_str("\n初握他人器物尚顯幾分生澀，須在實作中慢慢磨合重心。");
            }
            if let Some((_pack_id, _step, ep_line)) = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(&room_title, c_name).or_else(|| crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(&player_room, c_name)) {
                narrative.push_str(&format!("\n{ep_line}"));
            }
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Borrow".into(),
                target_id: target.id.clone(),
                target_name: borrow_name,
                narrative,
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
            return;
        }
    }
    if rel.balance_score < -1 || (disp < 40 && decision.choice == NpcStanceChoice::Refuse) {
        let narrative = format!("【{borrow_name}】早有防備，冷冷按住腰間行囊退開半步：「手腳放乾淨點，別打我身上東西的主意！」");
        if let Some(pool) = &pool_opt {
            let _ = crate::pg::living_world::record_player_world_action(
                pool,
                pid,
                "refused_borrow",
                target_id,
                &player_room,
                "refused",
                serde_json::json!({
                    "stance": "Refuse",
                    "balance_score": rel.balance_score,
                    "disposition": disp,
                    "directive": decision.prompt_directive,
                }),
                now,
            );
        }
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Borrow".into(),
            target_id: target.id.clone(),
            target_name: borrow_name,
            narrative,
            success: false,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }

    let (item_id, item_disp) = if let Some(i_id) = pick_npc_trade_offer(&target.inventory) {
        let name = get_item_def(&i_id)
            .map(|d| d.name)
            .or_else(|| {
                crate::world::materials::material_by_item_id(&i_id)
                    .map(|m| m.name.to_string())
            })
            .or_else(|| {
                crate::game::peninsula_catalog::find_catalog_entry(&i_id)
                    .map(|c| c.name.to_string())
            })
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| {
                if i_id.is_ascii() {
                    "隨身器物".to_string()
                } else {
                    i_id.clone()
                }
            });
        (i_id, name)
    } else if let Some((c_id, c_name, _)) = canon_item {
        (c_id.to_string(), c_name.to_string())
    } else {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Borrow".into(),
            target_id: target.id.clone(),
            target_name: borrow_name.clone(),
            narrative: format!("【{borrow_name}】身上並無可下手之物，你只得作罷。"),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    };
    let mut rng = rand::rng();
    let r: f64 = rng.random();
    let narrative = if r < 0.42 {
        let _ = remove_from_inventory(target_id, &item_id, 1);
        let _ = add_to_inventory(pid, &item_id, 1);
        let _ = event::append(now, pid, "borrow", target_id);
        if let Some(pool) = &pool_opt {
            let _ = crate::pg::living_world::open_obligation(
                pool,
                pid,
                "repay_debt",
                target_id,
                Some(now + 7200),
                "player_borrow",
                now,
            );
            let _ = crate::pg::living_world::record_relationship_delta(
                pool,
                target_id,
                pid,
                "debt",
                1,
                "player_borrow",
                now,
            );
            let _ = crate::pg::living_world::record_player_world_action(
                pool,
                pid,
                "borrow",
                target_id,
                &player_room,
                "success",
                serde_json::json!({
                    "item_id": item_id,
                    "obligation": "repay_debt",
                    "due_in_secs": 7200,
                }),
                now,
            );
        }
        let mut n = format!("你趁【{borrow_name}】不備，悄聲探手摸走了一件「{item_disp}」，收進隨身包袋。");
        if crate::pg::living_world::is_weapon_or_tool_item(&item_id, &item_disp) {
            n.push_str("\n初握他人器物尚顯幾分生澀，須在實作中慢慢磨合重心。");
        }
        if let Some((_pack_id, _step, ep_line)) = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(&room_title, &item_disp).or_else(|| crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(&player_room, &item_disp)) {
            n.push_str(&format!("\n{ep_line}"));
        }
        if target.kind == EntityKind::Npc {
            let _ = adjust_favorability(target_id, pid, FAV_BORROW_SUCCESS);
            let ex = npc_behavior_reaction_line(target_id, pid, "borrow_ok");
            if !ex.is_empty() {
                n.push('\n');
                let sanitized = if let Some(open) = ex.find('【') {
                    if let Some(close) = ex[open..].find('】') {
                        let after = &ex[open + close + '】'.len_utf8()..];
                        format!("【{}】{}", borrow_name, after)
                    } else {
                        ex
                    }
                } else {
                    ex
                };
                n.push_str(&sanitized);
            }
        }
        push_refresh(conn, pid);
        n
    } else if r < 0.78 {
        let mut n = format!("你探手行竊之際被【{borrow_name}】當場察覺！");
        if let Some(pool) = &pool_opt {
            let _ = crate::pg::living_world::record_relationship_delta(
                pool,
                target_id,
                pid,
                "grudge",
                1,
                "player_borrow_caught",
                now,
            );
            let _ = crate::pg::living_world::record_player_world_action(
                pool,
                pid,
                "borrow",
                target_id,
                &player_room,
                "caught",
                serde_json::json!({
                    "item_id": item_id,
                    "caught": true,
                }),
                now,
            );
        }
        if target.kind == EntityKind::Npc {
            let _ = adjust_favorability(target_id, pid, FAV_BORROW_CAUGHT);
            let ex = npc_behavior_reaction_line(target_id, pid, "borrow_caught");
            if !ex.is_empty() {
                n.push('\n');
                let sanitized = if let Some(open) = ex.find('【') {
                    if let Some(close) = ex[open..].find('】') {
                        let after = &ex[open + close + '】'.len_utf8()..];
                        format!("【{}】{}", borrow_name, after)
                    } else {
                        ex
                    }
                } else {
                    ex
                };
                n.push_str(&sanitized);
            }
        }
        let _ = event::append(now, pid, "borrow_fail", target_id);
        n
    } else {
        if let Some(pool) = &pool_opt {
            let _ = crate::pg::living_world::record_player_world_action(
                pool,
                pid,
                "borrow",
                target_id,
                &player_room,
                "missed",
                serde_json::json!({
                    "item_id": item_id,
                    "caught": false,
                }),
                now,
            );
        }
        let _ = event::append(now, pid, "borrow_fail", target_id);
        "你試探了一番，未能得手，悻悻收回。".into()
    };
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "Borrow".into(),
        target_id: target.id.clone(),
        target_name: borrow_name,
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}
pub(super) fn canonical_or_venue_borrow_item(
    target_name: &str,
    room_name: &str,
) -> Option<(&'static str, &'static str, &'static str)> {
    // 1. 優先查詢房間物件化契約（RoomVenueObject）之常備外借器物插座
    if let Some(venue) = crate::grid::resolve_room_venue_object(room_name, target_name) {
        if let Some(tool) = venue.lendable_tool {
            return Some(tool);
        }
    }

    // 2. 泛用工坊後備（若未對應專屬場館契約但屬工坊性質）
    if room_name.contains("工坊") || target_name.contains("工坊") {
        return Some((
            "wt_cat_0956",
            "柯氏松木家用工具箱",
            "「工坊手頭正忙，這口家用工具箱你先拿去應急修整，用完記得歸還。」",
        ));
    }

    // 3. 戶外／街巷行進中之職司人物：依其真實職銜對接 VocationalVesselSpec 之千物總典 catalog_id
    let clean_role = crate::npc::causal_core::strip_trailing_person_name_from_role(target_name);
    let has_explicit_vocation = [
        "學徒", "家眷", "內當家", "持家", "執事", "主事", "帳房", "商行", "公所", "調撥",
        "食堂", "廚", "茶", "膳", "掌事", "麵", "攤", "書", "學院", "講", "印", "學者",
        "學子", "觀星", "藥", "醫", "草", "丹", "診", "水輪", "石磨", "磨坊", "碾",
        "鐵", "鍛", "爐", "鑄", "礦", "石", "坑", "鑿", "木", "竹", "鋸", "船", "刨",
        "織", "布", "服", "染", "縫", "靴", "鞋", "傘", "皮", "革", "理髮", "剪剃", "剃",
        "漁", "港", "渡", "海", "舵", "巡", "衛", "哨", "武", "教頭", "獵", "採", "樵",
        "行旅", "押貨", "挑夫",
    ]
    .iter()
    .any(|kw| clean_role.contains(kw));
    if has_explicit_vocation {
        let vessel = crate::npc::causal_core::resolve_vocational_vessel(clean_role);
        if let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(vessel.catalog_id) {
            let quote = if vessel.is_bladed {
                "「這件隨身刃器是我平日趁手之物，借你應急使喚，切莫磕碰捲了刃口，用畢早些歸還。」"
            } else {
                "「這件隨身器物是我當差常備之物，你先拿去應急，用完記得完好歸還。」"
            };
            return Some((cat.id, cat.name, quote));
        }
    }

    // 4. 尋常街坊通用品 (Fallback)
    Some((
        "wt_cat_0781",
        "竹綿紙牛皮記事手札",
        "「出門在外少不得備忘記帳，這本隨身手札你先拿去用，記好各坊交代的事情，莫耽誤了行程。」",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::peninsula_catalog::find_catalog_entry;

    #[test]
    fn test_canonical_borrow_items_exist_and_match_catalog() {
        let test_cases = [
            ("當值醫師", "文曜・百草診所", "wt_cat_1057"),
            ("工坊木匠", "白塔・木作傢俱工坊", "wt_cat_0956"),
            ("鍛造學徒", "老堰頭・鐵匠鋪", "wt_cat_0957"),
            ("測紋主事", "分家巷・量測舍", "wt_cat_0735"),
            ("司鐘人", "白塔・傳訊鐘樓", "wt_cat_0729"),
            ("藏書閣主編", "文曜・知微書局", "wt_cat_0784"),
            ("商算主事", "文曜・百工商算學舍", "wt_cat_0788"),
            ("蒙學先生", "文曜・學堂講習舍", "wt_cat_0782"),
            ("補靴匠", "西門・修傘補靴攤", "wt_cat_0896"),
            ("雜貨主事", "老街・柑仔店", "wt_cat_0898"),
            ("當值大廚", "三水大食堂", "wt_cat_0897"),
            ("茶館當家", "東橋・茶屋歇宿部", "wt_cat_0723"),
            ("商行掌事", "三水總商行公議堂", "wt_cat_0816"),
            ("司天觀測員", "峰頂・觀星台", "wt_cat_0732"),
            ("武館教頭", "城西・習武堂", "wt_cat_1015"),
            ("理容師傅", "巷內・剪剃鋪", "wt_cat_0630"),
            ("浴場掌事", "山下・泉香溫水浴場", "wt_cat_0619"),
            ("成衣裁縫", "織造服飾坊", "wt_cat_0835"),
            ("車隊夥計", "西門・車馬聯運總站", "wt_cat_0891"),
            ("無名路人", "尋常巷弄", "wt_cat_0781"),
            ("合社獵手・林山海", "尋常巷弄", "wt_cat_1021"),
            ("紅土崁礦工", "尋常巷弄", "wt_cat_0964"),
            ("水尾寮漁夫", "尋常巷弄", "wt_cat_1006"),
        ];
        for (target, room, expected_id) in test_cases {
            let result = canonical_or_venue_borrow_item(target, room);
            assert!(result.is_some(), "Borrow target {} in {} should yield item", target, room);
            let (item_id, item_name, _) = result.unwrap();
            assert_eq!(item_id, expected_id, "Target {} should borrow {}", target, expected_id);

            let cat_entry = find_catalog_entry(item_id)
                .unwrap_or_else(|| panic!("Catalog entry {} must exist", item_id));

            assert_eq!(
                cat_entry.name, item_name,
                "Borrowed item {} name '{}' must exactly match catalog entry '{}'",
                item_id, item_name, cat_entry.name
            );
        }
    }
}

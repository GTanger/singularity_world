//! `do_action` — Borrow 分支：隨機機率的偷取/被發現。

use rand::Rng;

use crate::ai::decision_scorer::{NpcDecisionInput, NpcStanceChoice, score_npc_interaction_stance};
use crate::db::{
    self, FAV_BORROW_CAUGHT, FAV_BORROW_SUCCESS, add_to_inventory, adjust_favorability,
    get_disposition, get_entity, get_entity_room, get_item_def, inventory_has_qty,
    pick_npc_trade_offer, remove_from_inventory,
};
use crate::entity::{Character, EntityKind};
use crate::event;
use crate::npc::npc_behavior_reaction_line;

use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::ActionResultMsg;
use super::display_target_name;

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

    let target_true_name = crate::store::resolve_npc_person_name(&target.id, &target.display_title, None);
    let match_name = if !target_true_name.is_empty() && target_true_name != "陌生人" {
        target_true_name
    } else if !target.display_title.is_empty() {
        target.display_title.clone()
    } else {
        target.id.clone()
    };
    let canon_item = canonical_or_venue_borrow_item(&match_name, &player_room);

    // 物歸原主檢查：若玩家行囊中已持有該 NPC 的正典可借器物，則執行歸還
    if let Some((c_id, c_name, _)) = canon_item {
        if let Ok(Some(player_ent)) = db::get_entity(pid)
            && inventory_has_qty(&player_ent.inventory, c_id, 1)
        {
            let _ = remove_from_inventory(pid, c_id, 1);
            let _ = add_to_inventory(target_id, c_id, 1);
            let vstate = crate::pg::living_world::get_vessel_state(pid, c_id);
            let (fav_delta, narrative) = if vstate.condition == crate::pg::living_world::VesselCondition::RolledEdge {
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

    let (rel, obligations) = if let Some(pool) = &pool_opt {
        let r = crate::pg::living_world::get_relationship_summary(pool, target_id, pid)
            .unwrap_or_default();
        let o = crate::pg::living_world::list_open_obligations_for_entity(pool, target_id)
            .unwrap_or_default();
        (r, o)
    } else {
        (crate::pg::living_world::RelationshipSummary::default(), Vec::new())
    };

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

    // 熟識正典明借分支：若玩家已識名或關係良好且心緒不差，直接明借
    let is_acquainted_or_friendly = crate::store::player_knows_npc(pid, target_id)
        || rel.familiarity_score >= 1
        || rel.balance_score >= 0;
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
            let mut narrative = format!(
                "你向【{borrow_name}】拱手開口商借「{c_name}」應急。\n【{borrow_name}】爽快取出一件「{c_name}」遞給你：{borrow_quote}"
            );
            if crate::pg::living_world::is_weapon_or_tool_item(c_id, c_name) {
                narrative.push_str("\n初握他人器物尚顯幾分生澀，須在實作中慢慢磨合重心。");
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
        let mut n = format!("你探手行竊的瞬間被【{borrow_name}】當場察覺！");
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
    let name_part = target_name.rsplit(['|', '｜']).next().unwrap_or(target_name).trim();
    let has = |key: &str| -> bool {
        target_name.contains(key) || name_part == key || name_part.contains(key)
    };

    // 1. 溫氏脈絡診所（溫懷瑾、溫晴）
    if has("溫懷瑾") || has("溫晴") || room_name.contains("診所") || room_name.contains("醫館") {
        return Some((
            "wt_cat_1058",
            "溫氏導能溫石百草溫灸盒",
            "「這百草溫灸盒裡備著烘乾艾絨與溫石，行路疲乏時扣在穴位上溫敷片刻即可緩解。用完記得原樣送回。」",
        ));
    }

    // 2. 木作傢俱工坊（柯成器）
    if has("柯成器") || room_name.contains("木作") || room_name.contains("傢俱") {
        return Some((
            "wt_cat_0957",
            "柯氏松木家用工具箱",
            "「工具箱裡的角尺與小錘都是老頭子吃飯的傢伙，你拿去修整門戶用，用妥善了仔細收好歸還。」",
        ));
    }

    // 3. 岑氏第一鍛造工坊（岑守岳、岑拓）
    if has("岑守岳") {
        return Some((
            "wt_cat_0801",
            "三號坑精鍛羊角錘",
            "「這把精鍛羊角錘鋼口極韌，起釘敲打最是順手。拿去使吧，別拿去砸生鐵硬碰硬就好！」",
        ));
    }
    if has("岑拓") {
        return Some((
            "wt_cat_0736",
            "分家巷導能測紋石板",
            "「這塊測紋石板剛校正過靈動刻度，你拿去測驗器具導能走向，切記輕拿輕放，別摔著了。」",
        ));
    }
    if room_name.contains("鍛造") || room_name.contains("鐵匠") || room_name.contains("打鐵") {
        return Some((
            "wt_cat_0801",
            "三號坑精鍛羊角錘",
            "「工坊手頭正忙，這把精鍛羊角錘你先借去用，起釘打料小心著點，用完及時歸還。」",
        ));
    }

    // 4. 傳訊鐘樓（林時中）
    if has("林時中") || room_name.contains("鐘樓") || room_name.contains("司鐘") {
        return Some((
            "wt_cat_0730",
            "三水口校準隨身黃銅懷錶",
            "「這枚黃銅懷錶每日依鐘樓大鐘校時，走時極準。半島行程看重時刻節奏，你先拿去對準時刻，明日此時送回。」",
        ));
    }

    // 5. 藏書閣與書局（陸知微、沈硯舟）
    if has("陸知微") || has("沈硯舟") || room_name.contains("藏書閣") || room_name.contains("書局") {
        return Some((
            "wt_cat_0785",
            "《白塔風物誌》竹綿紙精印本",
            "「這本《白塔風物誌》詳細載有四方山川與港口水程，你行路時或有參照。翻閱時請保持手掌乾淨，勿近湯水燭火。」",
        ));
    }

    // 6. 百工商算學舍（秦明誠）
    if has("秦明誠") || room_name.contains("算學") || room_name.contains("商算") || room_name.contains("會計") {
        return Some((
            "wt_cat_0788",
            "烏木邊框便攜計數板",
            "「這塊計數板邊框結實、滑珠俐落，你拿去盤算核對帳目。核算務求分毫不差，用畢敬請歸位。」",
        ));
    }

    // 7. 學院蒙學堂講習舍（陸敬軒）
    if has("陸敬軒") || room_name.contains("蒙學") || room_name.contains("學堂") || room_name.contains("國小") {
        return Some((
            "wt_cat_0783",
            "國小基礎國語課本",
            "「讀書重在基礎紮實、字音端正。這本《國小基礎國語課本》你拿去細細溫讀，若有不懂的字句，隨時來教室問我。」",
        ));
    }

    // 8. 西門修傘補靴攤（馬補靴）
    if has("馬補靴") || room_name.contains("修傘") || room_name.contains("補靴") {
        return Some((
            "wt_cat_0608",
            "韌竹骨桐油棉紙摺疊晴雨傘",
            "「外頭天色說變就變，這把雙層桐油竹骨傘遮風避雨最牢靠。出門帶在身邊，別淋著了身子。」",
        ));
    }

    // 9. 分家巷柑仔店（許阿甘）
    if has("許阿甘") || room_name.contains("柑仔店") || room_name.contains("雜貨") {
        return Some((
            "wt_cat_0645",
            "防潮紅磷火柴盒與艾草蚊香",
            "「後生拿著！這盒火柴防潮做得好，夜裡點個蚊香防叮咬。阿婆這裡多得是，儘管拿去用。」",
        ));
    }

    // 10. 三水大食堂（陳萬福）
    if has("陳萬福") || room_name.contains("大食堂") || room_name.contains("食堂") || room_name.contains("廚房") {
        return Some((
            "wt_cat_0535",
            "雙層烏木內膽隨身保溫瓶",
            "「這雙層保溫瓶裡灌滿了滾燙的甘泉水，路上渴了隨時喝兩口暖胃。出門在外，可不能喝生水鬧肚子！」",
        ));
    }

    // 11. 東橋旅社與山風茶屋（沈清河）
    if has("沈清河") || room_name.contains("旅社") || room_name.contains("茶屋") || room_name.contains("茶館") {
        return Some((
            "wt_cat_0724",
            "旋蓋夜光貝母調光台燈",
            "「這盞貝母提燈光暈柔和不刺眼，夜裡照路或在案頭閱卷皆宜。旋暗燈罩即可熄光，好生收著。」",
        ));
    }

    // 12. 三水總商行與公議堂（陸秉衡、陸長貴、陸承禮）
    if has("陸秉衡") || has("陸長貴") || has("陸承禮") || room_name.contains("總商行") || room_name.contains("公議堂") || room_name.contains("契書") {
        return Some((
            "wt_cat_0817",
            "雙面雕刻銅製驗金天平",
            "「這具銅天平附有瑪瑙刀口，專用於勘驗散鎂與鎂晶碎屑分量。行商往來最講分寸，拿去照章稱量就好。」",
        ));
    }

    // 13. 觀星台（趙觀星）
    if has("趙觀星") || room_name.contains("觀星台") || room_name.contains("司天") {
        return Some((
            "wt_cat_0735",
            "黃銅隨身星象指北羅盤",
            "「此羅盤刻有黃道度量規線，荒原夜行可指引方位星位。切勿靠近強磁之物，以免指南失準。」",
        ));
    }

    // 14. 習武堂（岑雲野）
    if has("岑雲野") || room_name.contains("習武堂") || room_name.contains("武館") {
        return Some((
            "wt_cat_1015",
            "包革練習重木刀",
            "「練武當先正身心。這柄練習木刀配重沉穩，拿去熟練拔刀與卸力架勢，切記不可好勇鬥狠。」",
        ));
    }

    // 15. 剪剃鋪（周一刀）
    if has("周一刀") || room_name.contains("剪剃鋪") || room_name.contains("理容") {
        return Some((
            "wt_cat_0630",
            "精鋼摺疊修容剃刀",
            "「這把摺疊剃刀剛蕩過刀布，鋒利無比。拿去修面削木皆順手，用時腕子要放平，千萬別劃著手。」",
        ));
    }

    // 16. 泉香溫水浴場（白泉生）
    if has("白泉生") || room_name.contains("浴場") || room_name.contains("溫泉") {
        return Some((
            "wt_cat_0619",
            "大圳長絨蜂格棉面巾",
            "「這條大圳長絨面巾吸水極快，剛在香柏木桶蒸洗過。拿去擦汗淨面，周身清爽。」",
        ));
    }

    // 17. 織造服飾坊（蘇錦心）
    if has("蘇錦心") || room_name.contains("織造") || room_name.contains("成衣") || room_name.contains("染坊") {
        return Some((
            "wt_cat_0561",
            "大圳細棉麻透氣常服",
            "「這身細棉麻短衣針腳細密，吸汗透氣。換上趕路輕便舒坦，辦完事回來洗淨還我就好。」",
        ));
    }

    // 18. 西門車馬聯運總站（韓鐵山）
    if has("韓鐵山") || room_name.contains("聯運") || room_name.contains("貨運") || room_name.contains("車隊") {
        return Some((
            "wt_cat_0605",
            "雙肩厚帆布遠行背囊",
            "「這背囊用的是粗帆布雙道走線，能裝耐磨。出門在外背起行囊，腳步踩實了，一路平安！」",
        ));
    }

    // 聚落通用品 (Fallback)
    Some((
        "wt_cat_0782",
        "竹綿紙牛皮記事手札",
        "「出門在外少不得備忘記帳，這本隨身手札你先拿去用，記好各坊交代的事情，莫耽誤了行程。」",
    ))
}

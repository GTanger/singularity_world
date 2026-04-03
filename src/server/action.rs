//! WebSocket `do_action` 主幹（對齊 Go `server/handler_action.go` 與 `action_*`）。

use std::collections::HashSet;

use rand::Rng;

use crate::ai::{call_ai_talk, call_openai_compatible_chat, player_npc_talk_build_prompts};
use crate::combat::{resolve_v2, CombatOpt};
use crate::config::Server;
use crate::db::{
    self, add_to_inventory, adjust_disposition, adjust_favorability, default_trade_ask_mg,
    entity_in_venue_at_room, get_assignments_for_entity, get_disposition,
    get_item_def, get_object_and_room, get_object_by_id_in_room,
    get_object_by_name_in_room, get_room_ids_for_venue, get_room_name, get_sockets_for_npc,
    is_default_socket, log_npc_event, object_has_socket, object_response, pick_npc_trade_offer,
    remove_assignments_for_entity, remove_from_inventory, remove_schedule_for_entity,
    search_archival_for_player_talk, pick_style_examples, set_entity_hex, terrain_from_room,
    trade_floor_from_ask, trade_offer_clear, trade_offer_get, trade_offer_set,
    transfer_magnesium, update_vit, FAV_BORROW_CAUGHT, FAV_BORROW_SUCCESS, FAV_SLAY, FAV_SUBDUE,
    FAV_TALK, DISP_SUBDUED, EVT_DEATH, TradePending,
};
use crate::entity::{self, Character, EntityKind, Verb};
use crate::event::{self, types};
use crate::game;
use crate::gametext;
use crate::hex::{hex_room_id_from_coord, parse_hex_room_id};
use crate::npc::npc_behavior_reaction_line;

use super::broadcast::{refresh_room_views_for_room, send_narrate_to_room, send_room_view_to_session};
use super::conversation::flush_conversation_and_append;
use super::handler::{player_id, WsConnection};
use super::protocol::{ActionResultMsg, ClientMsg, MovedMsg};
use super::session::SessionStore;

/// 與目標是否在同一可互動位置（六角重合優先，否則比對 `entity_rooms` 字串）。
fn entities_share_playable_cell(player: &Character, target: &Character, player_room: &str) -> bool {
    if let (Some(pq), Some(pr), Some(tq), Some(tr)) =
        (player.hex_q, player.hex_r, target.hex_q, target.hex_r)
        && pq == tq && pr == tr
    {
        return true;
    }
    let Ok(tr) = db::get_entity_room(&target.id) else {
        return false;
    };
    !player_room.is_empty() && db::location_keys_equivalent(player_room, &tr)
}

/// 分派 `do_action`：先試同房實體，再試房間物件。
pub fn handle_do_action(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let target_id = msg.entity_id.clone();
    let mut action = msg.action.clone();
    if action == "Attack" {
        action = "Slay".into();
    }
    if target_id.is_empty() || action.is_empty() {
        conn.send_error(gametext::client("missing_target_or_action"));
        return;
    }
    if target_id == pid {
        conn.send_error(gametext::client("cannot_self_action"));
        return;
    }
    let Ok(Some(player_ch)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("get_self_failed"));
        return;
    };
    let Ok(player_room) = db::get_entity_room(&pid) else {
        conn.send_error(gametext::client("no_current_room"));
        return;
    };
    if player_room.is_empty() {
        conn.send_error(gametext::client("no_current_room"));
        return;
    }

    if try_entity_branch(conn, msg, &pid, &player_ch, &player_room, &target_id, &action) {
        return;
    }
    do_object_branch(conn, &pid, &player_room, &target_id, &action);
}

fn try_entity_branch(
    conn: &WsConnection,
    msg: &ClientMsg,
    pid: &str,
    player: &Character,
    player_room: &str,
    target_id: &str,
    action: &str,
) -> bool {
    let Ok(target_opt) = db::get_entity(target_id) else {
        return false;
    };
    let Some(target) = target_opt else {
        return false;
    };
    if !entities_share_playable_cell(player, &target, player_room) {
        conn.send_error(gametext::client("target_not_same_room"));
        return true;
    }
    if !entity_socket_ok(conn, &target, target_id, player_room, action) {
        return true;
    }
    let now = game::now_unix();
    match action {
        "Look" => do_entity_look(conn, pid, now, target_id, &target),
        "Talk" => do_entity_talk(conn, msg, pid, player_room, target_id, &target, now),
        "Subdue" | "Slay" => {
            do_entity_combat(conn, pid, player_room, target_id, action, &target, now);
        }
        "Borrow" => do_entity_borrow(conn, pid, target_id, &target, now),
        "Trade" => {
            let _ = do_entity_trade(conn, msg, pid, target_id, &target, now);
        }
        _ => conn.send_error(client_err_fmt("unknown_action_fmt", action)),
    }
    true
}

fn client_err_fmt(key: &str, arg: &str) -> String {
    gametext::client(key).replacen("%s", arg, 1)
}

fn entity_socket_ok(
    conn: &WsConnection,
    target: &Character,
    target_id: &str,
    player_room: &str,
    action: &str,
) -> bool {
    if target.kind == EntityKind::Npc {
        let sockets = get_sockets_for_npc(target_id, player_room);
        if !sockets.iter().any(|s| s == action) {
            conn.send_error(client_err_fmt("cannot_action_fmt", action));
            return false;
        }
        if !is_default_socket(action) {
            let in_venue = entity_in_venue_at_room(target_id, player_room).unwrap_or(false);
            if !in_venue {
                conn.send_error(client_err_fmt("target_not_at_workplace_fmt", action));
                return false;
            }
        }
        return true;
    }
    let Some(v) = verb_from_action(action) else {
        conn.send_error(client_err_fmt("unknown_action_fmt", action));
        return false;
    };
    if !entity::has_socket(&Character::sockets(), &v) {
        conn.send_error(client_err_fmt("cannot_action_fmt", action));
        return false;
    }
    true
}

fn verb_from_action(action: &str) -> Option<Verb> {
    match action {
        "Talk" => Some(Verb::Talk),
        "Borrow" => Some(Verb::Borrow),
        "Subdue" => Some(Verb::Subdue),
        "Slay" => Some(Verb::Slay),
        "Look" => Some(Verb::Look),
        "Trade" => Some(Verb::Trade),
        _ => None,
    }
}

fn display_target_name(target: &Character) -> String {
    if !target.display_title.is_empty() {
        target.display_title.clone()
    } else {
        target.id.clone()
    }
}

fn do_entity_look(conn: &WsConnection, pid: &str, now: i64, target_id: &str, target: &Character) {
    let narrative = build_look_narrative(target);
    let _ = event::append(now, pid, types::OBSERVED, target_id);
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "Look".into(),
        target_id: target.id.clone(),
        target_name: display_target_name(target),
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
    });
}

fn build_look_narrative(target: &Character) -> String {
    let name = display_target_name(target);
    let pronoun = match target.gender {
        Some(crate::entity::Gender::F) => "她",
        _ => "他",
    };
    let physique = match target.vit {
        v if v >= 20 => "體格異常魁梧",
        v if v >= 15 => "體格健壯",
        v if v >= 10 => "身材勻稱",
        _ => "身形消瘦",
    };
    let agility = match target.dex {
        v if v >= 20 => "舉止間透著驚人的敏捷",
        v if v >= 15 => "動作輕靈",
        v if v >= 10 => "步履平穩",
        _ => "行動略顯遲緩",
    };
    let qi_presence = match target.qi {
        v if v >= 20 => "，周身隱隱有氣勁流轉",
        v if v >= 15 => "，氣息沉穩",
        v if v >= 10 => "",
        _ => "，氣息微弱",
    };
    let mut desc = format!("你仔細打量了【{name}】。{pronoun}{physique}，{agility}{qi_presence}。");
    if !target.equipment_slots.is_empty() {
        let names = db::get_item_names(&target.equipment_slots);
        let pieces: Vec<String> = names.into_values().take(3).collect();
        if !pieces.is_empty() {
            desc.push_str(" 身上穿戴著");
            desc.push_str(&pieces.join("、"));
            desc.push('。');
        }
    }
    if !target.current_activity.is_empty() {
        desc.push_str(&format!(" {pronoun}目前{act}。", act = target.current_activity));
    }
    desc
}

fn do_entity_combat(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    target_id: &str,
    action: &str,
    target: &Character,
    now: i64,
) {
    let Ok(Some(attacker)) = db::get_entity(pid) else {
        conn.send_error(gametext::client("get_self_failed"));
        return;
    };
    let subdue = action == "Subdue";
    let (narrative, fight_winner, attacker_hp, defender_hp) =
        build_attack_narrative(player_room, &attacker, target, subdue);
    let mut narrative = narrative;
    if target.kind == EntityKind::Npc {
        let extra = if subdue {
            match fight_winner.as_str() {
                "attacker" => npc_behavior_reaction_line(target_id, pid, "subdue_victim"),
                "defender" => npc_behavior_reaction_line(target_id, pid, "lost_subdue"),
                _ => String::new(),
            }
        } else {
            String::new()
        };
        if !extra.is_empty() {
            narrative.push('\n');
            narrative.push_str(&extra);
        }
    }
    let out_action = if subdue { "Subdue" } else { "Slay" };
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: out_action.into(),
        target_id: target.id.clone(),
        target_name: display_target_name(target),
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
    });
    let _ = event::append(now, pid, types::COMBAT, target_id);
    let _ = update_vit(pid, attacker_hp);
    let _ = update_vit(target_id, defender_hp);
    let _ = event::append(now, pid, types::VIT, &attacker_hp.to_string());
    let _ = event::append(now, target_id, types::VIT, &defender_hp.to_string());
    if target.kind == EntityKind::Npc {
        if subdue {
            let _ = adjust_favorability(target_id, pid, FAV_SUBDUE);
            if fight_winner == "attacker" {
                let _ = adjust_disposition(target_id, DISP_SUBDUED);
            }
        } else {
            let _ = adjust_favorability(target_id, pid, FAV_SLAY);
        }
    }
    if target.kind == EntityKind::Npc && !subdue && defender_hp <= 0 {
        broadcast_npc_death(conn, player_room, target_id, &display_target_name(target), now);
        let _ = remove_assignments_for_entity(target_id);
        let _ = remove_schedule_for_entity(target_id);
    }
}

fn build_attack_narrative(
    room_id: &str,
    attacker: &Character,
    defender: &Character,
    subdue: bool,
) -> (String, String, i32, i32) {
    let mut opt = CombatOpt {
        subdue,
        ..Default::default()
    };
    let t = terrain_from_room(room_id);
    if !t.is_empty() {
        opt.terrain = t;
    }
    // 接入 soul_seed 戰鬥係數
    if let Some(seed) = attacker.soul_seed {
        let (a, _b, g) = db::expand_soul_seed_to_combat_axes(seed);
        opt.alpha = a;
        opt.gamma = g;
    }
    if let Some(seed) = defender.soul_seed {
        let (_a, _b, g) = db::expand_soul_seed_to_combat_axes(seed);
        opt.gamma = (opt.gamma + g) / 2.0;
    }
    // 計算有效屬性 (vit, qi, dex, atk_bonus)
    let (a_vit, _a_qi, a_dex, a_atk) = db::effective_stats(attacker);
    let (d_vit, _d_qi, d_dex, d_atk) = db::effective_stats(defender);
    // atk_bonus 疊加到 alpha 倍率，不灌 HP
    if a_atk > 0 {
        opt.alpha += a_atk as f64 * 0.1;
    }
    if d_atk > 0 {
        opt.alpha += d_atk as f64 * 0.05; // 守方裝備也微弱影響戰場強度
    }

    let (winner, raw_log, a_hp, d_hp) = resolve_v2(
        a_vit,
        a_dex,
        d_vit,
        d_dex,
        Some(&opt),
    );
    let mut a_name = display_target_name(attacker);
    let mut d_name = display_target_name(defender);
    if a_name.is_empty() {
        a_name.clone_from(&attacker.id);
    }
    if d_name.is_empty() {
        d_name.clone_from(&defender.id);
    }
    let log = raw_log
        .replace("攻方", &format!("【{a_name}】"))
        .replace("守方", &format!("【{d_name}】"));
    let prefix = if subdue {
        format!("你對【{d_name}】出手，意在留人！")
    } else {
        format!("你對【{d_name}】出手，意在送行！")
    };
    let suffix = if winner == "attacker" {
        if subdue {
            "\n你留住了對方。"
        } else {
            "\n你取得了勝利。"
        }
    } else {
        "\n你敗下陣來。"
    };
    (format!("{prefix}{log}{suffix}"), winner, a_hp, d_hp)
}

fn broadcast_npc_death(conn: &WsConnection, death_room: &str, npc_id: &str, name: &str, now: i64) {
    let payload = format!("在{death_room}倒下");
    log_npc_event(now, npc_id, EVT_DEATH, &payload);
    let narrate_text = format!("傳來消息：【{name}】倒下了。");
    let mut rooms: HashSet<String> = HashSet::new();
    rooms.insert(death_room.to_string());
    db::with_room_graph(|g| {
        for nb in g.neighbors(death_room) {
            rooms.insert(nb);
        }
    });
    if let Ok(assignments) = get_assignments_for_entity(npc_id) {
        for a in assignments {
            if let Ok(Some(vr)) = get_room_ids_for_venue(&a.venue_id) {
                for rid in vr {
                    rooms.insert(rid);
                }
            }
        }
    }
    let store_ref: &SessionStore = &conn.sessions;
    for rid in rooms {
        send_narrate_to_room(store_ref, &rid, &narrate_text);
    }
}

fn do_entity_borrow(conn: &WsConnection, pid: &str, target_id: &str, target: &Character, now: i64) {
    let borrow_name = display_target_name(target);
    let Some(item_id) = pick_npc_trade_offer(&target.inventory) else {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Borrow".into(),
            target_id: target.id.clone(),
            target_name: borrow_name.clone(),
            narrative: format!("【{borrow_name}】身無長物可借，你只得作罷。"),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
        });
        return;
    };
    let item_disp = get_item_def(&item_id)
        .map(|d| d.name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| item_id.clone());
    let mut rng = rand::rng();
    let r: f64 = rng.random();
    let narrative = if r < 0.42 {
        let _ = remove_from_inventory(target_id, &item_id, 1);
        let _ = add_to_inventory(pid, &item_id, 1);
        let _ = event::append(now, pid, "borrow", target_id);
        let mut n = format!("你悄聲「借」得一物——「{item_disp}」已入你手。");
        if target.kind == EntityKind::Npc {
            let _ = adjust_favorability(target_id, pid, FAV_BORROW_SUCCESS);
            let ex = npc_behavior_reaction_line(target_id, pid, "borrow_ok");
            if !ex.is_empty() {
                n.push('\n');
                n.push_str(&ex);
            }
        }
        super::handler::push_refresh(conn, pid);
        n
    } else if r < 0.78 {
        let mut n = format!("你伸手的瞬間被【{borrow_name}】察覺。");
        if target.kind == EntityKind::Npc {
            let _ = adjust_favorability(target_id, pid, FAV_BORROW_CAUGHT);
            let ex = npc_behavior_reaction_line(target_id, pid, "borrow_caught");
            if !ex.is_empty() {
                n.push('\n');
                n.push_str(&ex);
            }
        }
        let _ = event::append(now, pid, "borrow_fail", target_id);
        n
    } else {
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
    });
}

fn is_trade_reject_input(s: &str) -> bool {
    matches!(
        s.trim().to_lowercase().as_str(),
        "拒絕" | "取消" | "算了" | "不要" | "no" | "n"
    )
}

/// 回傳 true 表示已 sendError，呼叫方應停止。
fn do_entity_trade(
    conn: &WsConnection,
    msg: &ClientMsg,
    pid: &str,
    target_id: &str,
    target: &Character,
    now: i64,
) -> bool {
    let trade_target_name = display_target_name(target);
    let player_trade_input = msg.player_input.trim();
    if is_trade_reject_input(player_trade_input) {
        trade_offer_clear(pid, target_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: format!("你中止了與【{trade_target_name}】的交易。"),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
        });
        return false;
    }
    let pending = match trade_offer_get(pid, target_id) {
        Some(p) => p,
        None => {
            if !player_trade_input.is_empty() {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: "對方尚未開價。請先點【交易】取得報價，再在輸入欄填寫出價（鎂）。".into(),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                });
                return false;
            }
            let Some(item_id) = pick_npc_trade_offer(&target.inventory) else {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!("你向【{trade_target_name}】提出交易，對方表示目前暫無可交易之物。"),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                });
                return false;
            };
            let ask = default_trade_ask_mg(&item_id);
            let floor = trade_floor_from_ask(ask);
            let item_name = get_item_def(&item_id)
                .map(|d| d.name)
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| item_id.clone());
            trade_offer_set(
                pid,
                TradePending {
                    npc_id: target_id.to_string(),
                    item_id: item_id.clone(),
                    item_qty: 1,
                    ask_mg: ask,
                    floor_mg: floor,
                    expires_unix: 0,
                },
            );
            let narrative = format!(
                "【{trade_target_name}】願意向你賣出「{item_name}」一份，開價 {ask} 鎂（議價底線約 {floor} 鎂）。請再點【交易】，在欄位輸入你願付的鎂數；達開價則一口價成交，介於底線與開價之間則以你的出價成交。輸入「拒絕」可取消。"
            );
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "Trade".into(),
                target_id: target.id.clone(),
                target_name: trade_target_name,
                narrative,
                success: true,
                actions: vec![],
                move_target_id: String::new(),
            });
            return false;
        }
    };
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
        });
        return false;
    };
    if buyer.magnesium < paid {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "你的鎂不足，無法成交。".into(),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
        });
        return false;
    }
    if transfer_magnesium(pid, target_id, paid).is_err() {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "成交時鎂轉帳失敗，請稍後再試。".into(),
            success: false,
            actions: vec![],
            move_target_id: String::new(),
        });
        return false;
    }
    let _ = add_to_inventory(pid, &pending.item_id, pending.item_qty);
    let _ = remove_from_inventory(target_id, &pending.item_id, pending.item_qty);
    trade_offer_clear(pid, target_id);
    let item_name = get_item_def(&pending.item_id)
        .map(|d| d.name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| pending.item_id.clone());
    let narrative = if offer >= pending.ask_mg {
        format!("你以開價 {paid} 鎂買下「{item_name}」。")
    } else {
        format!("一番議價後，你以 {paid} 鎂買下「{item_name}」。")
    };
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
    });
    super::handler::push_refresh(conn, pid);
    false
}

fn current_game_hour(cfg: &Server) -> i32 {
    if cfg.game_time_epoch_unix == 0 {
        return 12;
    }
    let (_, h, _, _) = game::game_time_now(
        game::now_unix(),
        cfg.game_time_epoch_unix,
        cfg.game_time_scale,
    );
    h
}

fn talk_sensitivity_hint(target: &Character) -> String {
    let Some(seed) = target.soul_seed else {
        return String::new();
    };
    let p = db::expand_soul_seed_to_personality(seed);
    let mut s = String::new();
    if p.sensitivity > 0.6 {
        s.push_str("此角色較熱絡，可多說一兩句。");
    } else if p.sensitivity < 0.3 {
        s.push_str("此角色較冷淡，回覆簡短。");
    }
    s
}

fn talk_disposition_hint(target_id: &str) -> String {
    let disp = get_disposition(target_id);
    if disp < -30 {
        "此角色心境低落，語氣較冷漠沉鬱。".into()
    } else if disp > 30 {
        "此角色心情愉快，語氣較活潑熱情。".into()
    } else {
        String::new()
    }
}

fn build_talk_fallback(target: &Character, player_input: &str) -> (String, String) {
    let name = display_target_name(target);
    let pool = [
        "「你好，有什麼事嗎？」",
        "「這裡最近不太平靜，你小心點。」",
        "「嗯？」",
        "「有事快說，我還有活要幹。」",
        "「沒見過你，新來的？」",
        "「天快黑了，早點找地方落腳。」",
        "「……你誰啊？」",
        "「路過就路過，別瞎瞧。」",
    ];
    let mut h = 0_i32;
    for c in target.id.chars() {
        h += c as i32;
    }
    let seed = game::now_unix().saturating_mul(1000).saturating_add(i64::from(h));
    let mut idx = (seed as usize) % pool.len();
    if let Some(seed) = target.soul_seed {
        let p = db::expand_soul_seed_to_personality(seed);
        let shift = (p.boldness * (pool.len() as f64 / 2.0)) as i32;
        let mut adj = shift as isize;
        if p.sensitivity > 0.6 {
            adj += (pool.len() / 4) as isize;
        } else if p.sensitivity < 0.3 {
            adj -= (pool.len() / 4) as isize;
        }
        idx = (idx as isize + adj).rem_euclid(pool.len() as isize) as usize;
    }
    let line = pool[idx];
    let narrative = format!("你向【{name}】搭話。{name}說道：{line}");
    let npc_reply = if player_input.trim().is_empty() || player_input == "（搭話）" {
        line.to_string()
    } else {
        narrative.clone()
    };
    (narrative, npc_reply)
}

fn do_entity_talk(
    conn: &WsConnection,
    msg: &ClientMsg,
    pid: &str,
    player_room: &str,
    target_id: &str,
    target: &Character,
    now: i64,
) {
    let mut player_input = msg.player_input.clone();
    if player_input.trim().is_empty() {
        player_input = "（搭話）".into();
    }
    let backstory = db::build_identity(target_id);
    let snippets = search_archival_for_player_talk(target_id, &player_input, 5);
    let style_examples = pick_style_examples(target_id, 3);
    let mut sensitivity = talk_sensitivity_hint(target);
    sensitivity.push_str(&talk_disposition_hint(target_id));
    let mut room_display_name = get_room_name(player_room).unwrap_or_default();
    if room_display_name.trim().is_empty() {
        room_display_name = player_room.to_string();
    }
    let (system_prompt, user_msg) = player_npc_talk_build_prompts(
        &player_input,
        &backstory,
        &snippets,
        &style_examples,
        &sensitivity,
        &room_display_name,
    );
    let talk_name = display_target_name(target);
    let (narrative, npc_reply) = if conn.cfg.player_talk_uses_web_api() {
        match call_openai_compatible_chat(
            &conn.cfg.player_talk_api_base_url,
            &conn.cfg.player_talk_api_key,
            &conn.cfg.player_talk_api_model,
            &system_prompt,
            &user_msg,
        ) {
            Ok(reply) => (
                format!("你向【{talk_name}】搭話。{talk_name}說道：「{reply}」"),
                reply,
            ),
            Err(_) => build_talk_fallback(target, &player_input),
        }
    } else if !conn.cfg.ollama_base_url.is_empty() && !conn.cfg.ollama_model.is_empty() {
        match call_ai_talk(
            &conn.cfg.ollama_base_url,
            &conn.cfg.ollama_model,
            &player_input,
            &backstory,
            &snippets,
            &style_examples,
            &sensitivity,
            &room_display_name,
        ) {
            Ok(reply) => (
                format!("你向【{talk_name}】搭話。{talk_name}說道：「{reply}」"),
                reply,
            ),
            Err(_) => build_talk_fallback(target, &player_input),
        }
    } else {
        build_talk_fallback(target, &player_input)
    };
    let _ = event::append(now, pid, "talk", target_id);
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "Talk".into(),
        target_id: target.id.clone(),
        target_name: talk_name.clone(),
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
    });
    conn.sessions
        .record_player_talk_for_room_echo(pid, player_room, &msg.player_input);
    flush_conversation_and_append(pid, target_id, &player_input, &npc_reply, now);
    let _ = adjust_favorability(target_id, pid, FAV_TALK);
}

fn resolve_room_object(room_id: &str, target_id: &str) -> Option<crate::model::RoomObject> {
    get_object_by_id_in_room(room_id, target_id)
        .or_else(|| get_object_by_name_in_room(room_id, target_id))
        .or_else(|| {
            get_object_and_room(target_id).and_then(|(o, r)| (r == room_id).then_some(o))
        })
}

fn do_object_branch(conn: &WsConnection, pid: &str, player_room: &str, target_id: &str, action: &str) {
    let Some(obj) = resolve_room_object(player_room, target_id) else {
        conn.send_error(gametext::client("target_not_found"));
        return;
    };
    if !object_has_socket(&obj, action) {
        conn.send_error(client_err_fmt("cannot_action_fmt", action));
        return;
    }
    let mut narrative = object_response(&obj, action);
    if narrative.is_empty() {
        narrative = format!(
            "你對【{}】執行了「{action}」，但似乎沒有什麼特別的反應。",
            obj.name
        );
    }
    let now = game::now_unix();
    let _ = event::append(now, pid, types::OBSERVED, &obj.id);
    if !apply_object_move(conn, pid, player_room, &obj, action) {
        return;
    }
    let (others, move_target_id) = object_followup_buttons(&obj, player_room, action);
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: action.into(),
        target_id: obj.id.clone(),
        target_name: obj.name.clone(),
        narrative,
        success: true,
        actions: others,
        move_target_id,
    });
}

fn object_followup_buttons(
    obj: &crate::model::RoomObject,
    player_room: &str,
    action: &str,
) -> (Vec<String>, String) {
    let mut others: Vec<String> = obj
        .sockets
        .iter()
        .filter(|s| *s != action)
        .cloned()
        .collect();
    let mut move_target_id = String::new();
    if action == "Look" && others.is_empty() && object_response(obj, "Look").is_empty() {
        let Ok(objs) = db::get_objects_in_room(player_room) else {
            return (others, move_target_id);
        };
        let idx = objs.iter().position(|o| o.id == obj.id);
        if let Some(i) = idx {
            for range in [(i + 1..objs.len()), (0..i)] {
                for j in range {
                    let o = &objs[j];
                    if !o.move_to_room_id.is_empty() && object_has_socket(o, "Move") {
                        move_target_id.clone_from(&o.id);
                        break;
                    }
                }
                if !move_target_id.is_empty() {
                    break;
                }
            }
        } else {
            for o in &objs {
                if !o.move_to_room_id.is_empty() && object_has_socket(o, "Move") {
                    move_target_id.clone_from(&o.id);
                    break;
                }
            }
        }
        if !move_target_id.is_empty() {
            others = vec!["Move".into()];
        }
    }
    (others, move_target_id)
}

fn apply_object_move(
    conn: &WsConnection,
    pid: &str,
    _player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
) -> bool {
    if action != "Move" || obj.move_to_room_id.is_empty() {
        return true;
    }
    let Some((q, r)) = parse_hex_room_id(&obj.move_to_room_id) else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    let gh = current_game_hour(&conn.cfg);
    let Ok(view_opt) = game::get_hex_room_view(pid, q, r, gh) else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    let Some(view) = view_opt else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    if set_entity_hex(pid, q, r).is_err() {
        conn.send_error(gametext::client("move_failed"));
        return false;
    }
    let Some(session) = conn.sessions.get(pid) else {
        conn.send_error(gametext::client("move_failed"));
        return false;
    };
    send_room_view_to_session(&session, &view, pid, &conn.cfg);
    let rid = hex_room_id_from_coord(q, r);
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: pid.to_string(),
        room_id: rid.clone(),
        room_name: view.room.name.clone(),
    };
    if let Ok(bytes) = serde_json::to_vec(&moved) {
        conn.hub.broadcast(bytes);
    }
    refresh_room_views_for_room(&conn.sessions, &conn.cfg, &rid);
    true
}

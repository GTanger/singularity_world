//! `do_action` — Borrow 分支：隨機機率的偷取/被發現。

use rand::Rng;

use crate::db::{
    add_to_inventory, adjust_favorability, get_item_def, pick_npc_trade_offer,
    remove_from_inventory, FAV_BORROW_CAUGHT, FAV_BORROW_SUCCESS,
};
use crate::entity::{Character, EntityKind};
use crate::event;
use crate::npc::npc_behavior_reaction_line;

use super::super::handler::{push_refresh, WsConnection};
use super::super::protocol::ActionResultMsg;
use super::display_target_name;

pub(super) fn do_entity_borrow(
    conn: &WsConnection,
    pid: &str,
    target_id: &str,
    target: &Character,
    now: i64,
) {
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
        push_refresh(conn, pid);
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

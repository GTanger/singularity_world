//! `do_action` — Trade 分支：NPC 報價、玩家議價、鎂轉帳 + inventory 轉移。

use crate::db::{
    self, TradePending, default_trade_ask_mg, get_item_def, inventory_has_qty,
    pick_npc_trade_offer, settle_inventory_trade, trade_floor_from_ask, trade_offer_clear,
    trade_offer_get, trade_offer_set,
};
use crate::entity::Character;
use crate::event::{self, types};
use crate::gametext;
use crate::world::materials;

use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::{ActionResultMsg, ClientMsg};
use super::display_target_name;

fn is_trade_reject_input(s: &str) -> bool {
    matches!(
        s.trim().to_lowercase().as_str(),
        "拒絕" | "取消" | "算了" | "不要" | "no" | "n"
    )
}

/// 回傳 true 表示已 sendError，呼叫方應停止。
pub(super) fn do_entity_trade(
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
            open_trade: false,
            suggested_action: String::new(),
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
                    narrative: "對方尚未開價。請先點【交易】取得報價，再在輸入欄填寫出價（鎂）。"
                        .into(),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                });
                return false;
            }
            let Some(item_id) = pick_npc_trade_offer(&target.inventory) else {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "Trade".into(),
                    target_id: target.id.clone(),
                    target_name: trade_target_name.clone(),
                    narrative: format!(
                        "你向【{trade_target_name}】提出交易，對方表示目前暫無可交易之物。"
                    ),
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                });
                return false;
            };
            let ask = default_trade_ask_mg(&item_id);
            let floor = trade_floor_from_ask(ask);
            let item_name = get_item_def(&item_id)
                .map(|d| d.name)
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| item_id.clone());
            let origin = materials::material_origin_label(&item_id)
                .filter(|s| !s.is_empty())
                .unwrap_or("來源未明");
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
                "【{trade_target_name}】願意向你賣出「{item_name}」一份，來源是{origin}，開價 {ask} 鎂（議價底線約 {floor} 鎂）。請在交易欄位輸入你願付的鎂數；達開價則一口價成交，介於底線與開價之間則以你的出價成交。輸入「拒絕」可取消。"
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
                open_trade: false,
                suggested_action: String::new(),
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
            open_trade: false,
            suggested_action: String::new(),
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
            open_trade: false,
            suggested_action: String::new(),
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
            open_trade: false,
            suggested_action: String::new(),
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
            open_trade: false,
            suggested_action: String::new(),
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
            open_trade: false,
            suggested_action: String::new(),
        });
        return false;
    }
    let Ok(Some(seller)) = db::get_entity(target_id) else {
        trade_offer_clear(pid, target_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: format!("【{trade_target_name}】已不在可交易狀態，這筆交易作廢。"),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
        });
        return false;
    };
    if !inventory_has_qty(&seller.inventory, &pending.item_id, pending.item_qty) {
        trade_offer_clear(pid, target_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: format!("【{trade_target_name}】手上的貨已不在，這筆交易作廢。"),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
        });
        return false;
    }
    if settle_inventory_trade(pid, target_id, &pending.item_id, pending.item_qty, paid).is_err() {
        trade_offer_clear(pid, target_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Trade".into(),
            target_id: target.id.clone(),
            target_name: trade_target_name.clone(),
            narrative: "成交時交易結算失敗，請重新確認對方貨物與你的鎂。".into(),
            success: false,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
        });
        return false;
    }
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
        open_trade: false,
        suggested_action: String::new(),
    });
    push_refresh(conn, pid);
    false
}

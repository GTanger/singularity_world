//! `do_action` — Trade 分支：NPC 報價、玩家議價、鎂轉帳 + inventory 轉移。

use std::collections::HashMap;
use std::sync::Mutex;
use crate::db::{
    self, TradePending, default_trade_ask_mg, get_entity_room, get_item_def, inventory_has_qty,
    mirror_market_trade_settlement, pick_entity_trade_offer, settle_inventory_trade,
    trade_floor_from_ask, trade_offer_clear, trade_offer_get, trade_offer_set,
};
use crate::entity::Character;
use crate::event::{self, types};
use crate::gametext;

use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::{ActionResultMsg, ClientMsg};


/// 回傳 true 表示已 sendError，呼叫端應停止。
pub(super) fn do_entity_trade(
    conn: &WsConnection,
    msg: &ClientMsg,
    pid: &str,
    target_id: &str,
    target: &Character,
    now: i64,
) -> bool {
    let trade_target_name = super::player_visible_target_name(pid, target);
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
            tactical_choices: vec![],
        });
        return false;
    }
    if handle_craft_mutate_sell_trade(
        conn,
        pid,
        target_id,
        target,
        now,
        &trade_target_name,
        player_trade_input,
    ) {
        return false;
    }
    let pending = match trade_offer_get(pid, target_id) {
        Some(p) => p,
        None => {
            return handle_trade_quotation(
                conn,
                pid,
                target_id,
                target,
                now,
                &trade_target_name,
                player_trade_input,
            );
        }
    };
    handle_trade_settlement(
        conn,
        pid,
        target_id,
        target,
        now,
        trade_target_name,
        player_trade_input,
        pending,
    )
}

include!("trade/vendor.rs");
include!("trade/quote.rs");
include!("trade/settlement.rs");

//! 玩家狀態與離線行事查詢處理子模組。

use super::super::protocol::{ActionResultMsg, ClientMsg, StatusMsg};
use super::inventory::handle_get_entity_status;
use super::{WsConnection, player_id};

/// 取得或建構玩家狀態訊息。
#[allow(dead_code)]
pub fn build_status_msg(ent: &crate::entity::Character, is_self: bool) -> StatusMsg {
    super::inventory::entity_status_from_character(ent, is_self)
}

/// 處理狀態查詢請求。
pub fn handle_get_status(conn: &WsConnection, msg: &ClientMsg) {
    handle_get_entity_status(conn, msg);
}

/// 處理變更玩家離線行事方針。
pub fn handle_set_offline_directive(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error("請先登入");
        return;
    };
    let code = if !msg.action.is_empty() {
        msg.action.as_str()
    } else if !msg.player_input.is_empty() {
        msg.player_input.as_str()
    } else if !msg.target_id.is_empty() {
        msg.target_id.as_str()
    } else {
        "inn_rest"
    };

    let dir = crate::npc::causal_core::OfflineAutonomyDirective::from_code(code);
    crate::npc::causal_core::set_player_offline_directive(&pid, dir);

    let confirm_narrative = format!("已將離線行事方針囑託為「{}」：{}", dir.label(), dir.desc());
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "set_offline_directive".into(),
        target_id: dir.code().into(),
        target_name: dir.label().into(),
        narrative: confirm_narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });

    handle_get_status(conn, msg);
}

/// 處理設定或清除玩家個人 OpenRouter API 金鑰。
pub fn handle_set_openrouter_key(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error("請先登入");
        return;
    };
    let is_clear = msg.action == "clear";
    let is_silent_sync = msg.action == "sync";
    let raw_key = if is_clear {
        ""
    } else if !msg.player_input.is_empty() {
        msg.player_input.as_str()
    } else {
        msg.password.as_str()
    };
    let has_key = crate::ai::set_player_openrouter_key(&pid, raw_key);
    if !is_silent_sync {
        let confirm_narrative = if has_key {
            format!(
                "已儲存個人 OpenRouter 金鑰（***）。此後你與人物交談將優先由 OpenRouter（{}）驅動；若網路或金鑰異常則自動回退本機 Gemma-4-26B-A4B 模型。",
                crate::ai::DEFAULT_OPENROUTER_TALK_MODEL
            )
        } else {
            "已清除個人 OpenRouter 金鑰。此後你與人物交談將使用本機 Gemma-4-26B-A4B 模型。".to_string()
        };
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "set_openrouter_key".into(),
            target_id: if has_key { "openrouter" } else { "local" }.into(),
            target_name: if has_key {
                crate::ai::DEFAULT_OPENROUTER_TALK_MODEL
            } else {
                crate::ai::LOCAL_FAST_GPU_MODEL
            }
            .into(),
            narrative: confirm_narrative,
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
    }
    handle_get_status(conn, msg);
}

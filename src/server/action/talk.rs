//! `do_action` — Talk 分支：LLM 對話（走 Ollama 或 OpenAI-compatible），fallback pool。

use crate::ai::{call_ai_talk, call_openai_compatible_chat, player_npc_talk_build_prompts};
use crate::db::{
    self, adjust_favorability, get_disposition, get_room_name, pick_style_examples,
    search_archival_for_player_talk, FAV_TALK,
};
use crate::entity::Character;
use crate::event;
use crate::game;

use super::super::conversation::flush_conversation_and_append;
use super::super::handler::WsConnection;
use super::super::protocol::{ActionResultMsg, ClientMsg};
use super::display_target_name;

pub(super) fn talk_sensitivity_hint(target: &Character) -> String {
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

pub(super) fn talk_disposition_hint(target_id: &str) -> String {
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

pub(super) fn do_entity_talk(
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

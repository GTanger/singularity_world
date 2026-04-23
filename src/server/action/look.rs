//! `do_action` — Look 分支：打量目標，輸出外觀敘述。

use crate::db;
use crate::entity::Character;
use crate::event::{self, types};

use super::super::handler::WsConnection;
use super::super::protocol::ActionResultMsg;
use super::display_target_name;

pub(super) fn do_entity_look(
    conn: &WsConnection,
    pid: &str,
    now: i64,
    target_id: &str,
    target: &Character,
) {
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

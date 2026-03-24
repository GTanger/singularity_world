// NPC identity 字串組裝（對齊 Go `db/backstory.go` `BuildIdentity` 精簡版）。

use crate::store;

use super::{get_npc_person_display_name, get_personality_for_entity};

/// 組裝 NPC 給 LLM 用的身份描述（事件／摘要等可後續補齊）。
#[must_use]
pub fn build_identity(entity_id: &str) -> String {
    let mut person = get_npc_person_display_name(entity_id).unwrap_or_default();
    if person.is_empty() {
        person = entity_id.to_string();
    }
    let occ = super::assignment::get_npc_title_from_assignments(entity_id);
    let mut result = format!("你是{person}。");
    let Some(arc) = store::get_store() else {
        return append_personality(result, entity_id);
    };
    let s = arc.read().unwrap();
    let assignments = s.get_assignments_for_entity(entity_id);
    if let Some(a) = assignments.first()
        && let Some(venue) = s.get_venue(&a.venue_id)
    {
        if !occ.is_empty() {
            result = format!("你是{person}，職稱是{occ}，在{}工作。", venue.name);
        } else {
            result = format!("你是{person}，在{}相關場合活動。", venue.name);
        }
    }
    drop(s);
    append_personality(result, entity_id)
}

fn append_personality(mut result: String, entity_id: &str) -> String {
    if let Some(p) = get_personality_for_entity(entity_id) {
        result.push_str(&personality_to_sentence(&p));
    }
    result
}

fn personality_to_sentence(p: &super::Personality) -> String {
    let mut parts = Vec::new();
    if p.boldness > 0.6 {
        parts.push("你性格大膽、敢衝");
    } else if p.boldness < 0.3 {
        parts.push("你性格謹慎、怕事");
    }
    if p.orderliness > 0.6 {
        parts.push("做事守規矩");
    } else if p.orderliness < 0.3 {
        parts.push("不太受規矩約束");
    }
    if p.sensitivity > 0.6 {
        parts.push("對人較敏感、口吻較熱絡");
    } else if p.sensitivity < 0.3 {
        parts.push("較冷淡、話少");
    }
    if parts.is_empty() {
        return String::new();
    }
    parts.join("，") + "。"
}

// NPC identity 字串組裝（對齊既有 `db/backstory` `BuildIdentity`）。

use crate::store;

use super::{
    get_active_thoughts, get_npc_person_display_name, get_personality_for_entity, get_recent_events,
};

/// 組裝 NPC 給 LLM 用的身份描述：真名、職稱、場所、性格、心境、最近事件、摘要（對齊既有 `BuildIdentity`）。
#[must_use]
pub fn build_identity(entity_id: &str) -> String {
    let mut person = get_npc_person_display_name(entity_id).unwrap_or_default();
    if person.is_empty() {
        person = entity_id.to_string();
    }
    let occ = super::assignment::get_npc_title_from_assignments(entity_id);
    let mut result = format!("你是{person}。");
    let mut personality_opt = None;

    let (ent_opt, venue_name_opt, live_act, npc_summary) = if let Some(arc) = store::get_store() {
        let s = arc.read();
        let ent_opt = s.get_entity(entity_id);
        let assignments = s.get_assignments_for_entity(entity_id);
        let venue_name_opt = assignments
            .first()
            .and_then(|a| s.get_venue(&a.venue_id))
            .map(|v| v.name.clone());
        let live_act = ent_opt
            .as_ref()
            .map(|e| s.effective_npc_activity(e))
            .unwrap_or_default();
        let summary = s.get_npc_summary(entity_id);
        (ent_opt, venue_name_opt, live_act, summary)
    } else {
        (None, None, String::new(), String::new())
    };

    if let Some(ent) = ent_opt {
        let gender = match ent.gender.as_str() {
            "F" | "female" | "女" | "女性" => "女性",
            _ => "男性",
        };
        let role_label = crate::store::npc_persistent_role_label(entity_id, &ent.display_title);
        let role = if !role_label.is_empty() {
            role_label
        } else if !occ.is_empty() {
            occ.clone()
        } else {
            "居民".to_string()
        };

        if let Some(v_name) = venue_name_opt {
            result = format!("你是{person}，{gender}，職稱為{role}，在{v_name}工作。");
        } else {
            result = format!("你是{person}，{gender}，職稱為{role}。");
        }

        let slot = crate::store::wenyao_household_slot_for(entity_id);
        result.push_str(&format!(
            "你的穿著外觀：{}。住所為【{}】。職務為【{}】（目的：{}；常駐行為：{}）。日常雜務往來【{}】（目的：{}；常駐動靜：{}）。",
            slot.attire_line,
            slot.home_name,
            slot.work_name,
            slot.work_purpose,
            slot.work_stay_activity,
            slot.errand_name,
            slot.errand_purpose,
            slot.errand_stay_activity,
        ));

        if !live_act.is_empty() {
            result.push_str(&format!("你當前的具體活動：{}。", live_act));
        }

        let inv_snap = super::get_inventory(&ent.inventory, ent.vit);
        let inv_desc: Vec<String> = inv_snap
            .items
            .iter()
            .map(|it| {
                let stage = crate::game::object_ecology::classify_item_stage(&it.item_id);
                let stage_label = match stage {
                    "raw" => "原料",
                    "mutated" => "異變器物",
                    _ => "成品",
                };
                format!("{} ×{}（{}）", it.name, it.qty, stage_label)
            })
            .collect();
        if inv_desc.is_empty() {
            result.push_str(&format!("隨身持有 {} 鎂幣，行囊目前未置他物。", ent.magnesium));
        } else {
            result.push_str(&format!(
                "隨身持有 {} 鎂幣，行囊中攜有：{}。",
                ent.magnesium,
                inv_desc.join("、")
            ));
        }

        let disp = ent.disposition;
        let mood_desc = if disp >= 40 {
            "心情甚佳、神采飛揚"
        } else if disp >= 15 {
            "心情平順舒暢"
        } else if disp > -15 {
            "心緒平靜中立"
        } else if disp > -40 {
            "略有煩心焦慮"
        } else {
            "心緒躁悶防備"
        };
        result.push_str(&format!("當前情緒心境：{}（心境指標：{}）。", mood_desc, disp));

        personality_opt = get_personality_for_entity(entity_id).or_else(|| {
            if let Some(seed) = ent.soul_seed
                && seed != 0
            {
                Some(super::expand_soul_seed_to_personality(seed))
            } else {
                None
            }
        });

        if let Some((ob_kind, ob_subject)) = crate::store::get_active_npc_obligation(entity_id) {
            result.push_str(&format!("你當前手頭有待辦的義務或牽掛：{}（{}）。", ob_kind, ob_subject));
        }
    } else if let Some(v_name) = venue_name_opt {
        if !occ.is_empty() {
            result = format!("你是{person}，職稱是{occ}，在{v_name}工作。");
        } else {
            result = format!("你是{person}，在{v_name}相關場合活動。");
        }
    }

    if personality_opt.is_none() {
        personality_opt = get_personality_for_entity(entity_id);
    }
    if let Some(p) = personality_opt {
        result.push_str(&personality_to_sentence(&p));
    }
    let thoughts = get_active_thoughts(entity_id, 5);
    if !thoughts.is_empty() {
        result.push_str("你最近的心境念頭：");
        for t in &thoughts {
            if let Some(f) = &t.flavor
                && !f.is_empty()
                && !f.contains("青籠關")
                && !f.contains("林緣聚地")
            {
                result.push_str(f);
                result.push('；');
            }
        }
    }
    let evts = get_recent_events(entity_id, 5);
    if !evts.is_empty() {
        result.push_str("你最近的經歷動向：");
        for e in evts {
            if !e.payload.contains("青籠關") && !e.payload.contains("林緣聚地") {
                result.push_str(&e.payload);
                result.push('；');
            }
        }
    }
    let pulse = crate::game::object_ecology::market_pulse_summary_for_prompt();
    if !pulse.is_empty() {
        result.push_str(&format!("當前坊市經濟脈動：{}。", pulse));
    }
    if !npc_summary.is_empty() && !npc_summary.contains("青籠關") {
        result.push_str("與來客過往互動印象：");
        result.push_str(&npc_summary);
    }
    result
}

fn append_tail(mut result: String, entity_id: &str) -> String {
    if let Some(p) = get_personality_for_entity(entity_id) {
        result.push_str(&personality_to_sentence(&p));
    }
    result
}

fn personality_to_sentence(p: &super::Personality) -> String {
    let mut parts = Vec::new();
    if p.boldness > 0.6 {
        parts.push("性格大膽豪爽敢衝");
    } else if p.boldness < 0.3 {
        parts.push("行事慎重戒備");
    } else {
        parts.push("作風沉著隨和");
    }
    if p.orderliness > 0.6 {
        parts.push("做事一絲不苟講究規矩");
    } else if p.orderliness < 0.3 {
        parts.push("性情隨性不喜條規約束");
    } else {
        parts.push("進退知曉分寸靈活應對");
    }
    if p.sensitivity > 0.6 {
        parts.push("心思細膩善於察言觀色、口吻熱絡");
    } else if p.sensitivity < 0.3 {
        parts.push("言語務實精簡、情緒內斂");
    } else {
        parts.push("待人客氣得體");
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("你的性格秉性為：{}。", parts.join("，"))
}

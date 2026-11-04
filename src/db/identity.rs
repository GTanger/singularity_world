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

    let mut soul_seed_val = None;
    let mut effective_occupation = if !occ.is_empty() {
        occ.clone()
    } else {
        "居民".to_string()
    };

    if let Some(ent) = ent_opt {
        soul_seed_val = ent.soul_seed;
        let is_female = crate::store::npc_effective_is_female(entity_id, Some(&ent.gender));
        let gender = if is_female { "女性" } else { "男性" };
        person = crate::store::resolve_npc_person_name(entity_id, &ent.display_title, Some(&ent.gender));
        let role_label = crate::store::npc_persistent_role_label(entity_id, &ent.display_title);
        let role = if !role_label.is_empty() {
            role_label
        } else if !occ.is_empty() {
            occ.clone()
        } else {
            "居民".to_string()
        };
        effective_occupation = role.clone();
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
            let ob_label = match ob_kind.as_str() {
                "restock_goods" => "鋪子補貨",
                "repair_tool" | "maintenance_recovery" | "tool_repair" => "器物修整",
                "stranded_cargo" | "market_delivery" | "delivery" => "貨件調度",
                "craft_material" | "material_shortage" | "MaterialShortage" | "supply_shortage" => {
                    "資材補備"
                }
                "fuel_shortage" => "爐灶柴炭添補",
                "treat_injury" | "medical_help" | "MedicalHelp" => "傷病調理",
                "repay_debt" | "debt_settlement" => "帳目結清",
                "rest_recovery" => "歇息養神",
                _ => "手頭差事",
            };
            result.push_str(&format!(
                "你當前手頭有待辦的義務或牽掛：{}（{}）。",
                ob_label, ob_subject
            ));
        }
    } else if let Some(v_name) = venue_name_opt {
        if !occ.is_empty() {
            result = format!("你是{person}，職稱是{occ}，在{v_name}工作。");
        } else {
            result = format!("你是{person}，在{v_name}相關場合活動。");
        }
    }

    let seed = crate::npc::causal_core::deterministic_resident_seed(
        entity_id,
        soul_seed_val,
    );
    if personality_opt.is_none() {
        personality_opt = get_personality_for_entity(entity_id);
    }
    if let Some(p) = personality_opt {
        result.push_str(&personality_to_sentence(&p));
    }
    let myriad_phase = crate::npc::causal_core::describe_npc_myriad_personality_phase(
        entity_id,
        &effective_occupation,
        seed,
    );
    if !myriad_phase.is_empty() {
        result.push_str(&myriad_phase);
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
        let mark = result.len();
        result.push_str("你最近的經歷動向：");
        let header_len = result.len();
        for e in evts {
            if e.payload.is_empty()
                || e.payload.chars().all(|c| c.is_ascii_digit() || c == '-')
                || e.payload.contains("npc_")
                || e.payload.contains("wt-")
                || e.payload.contains("wt_")
                || e.payload.contains("grid:")
                || e.payload.contains("ambient_")
                || e.payload.contains("shanhai_")
                || e.payload.contains("青籠關")
                || e.payload.contains("林緣聚地")
            {
                continue;
            }
            let sanitized = e.payload.replace(['|', '｜'], "・");
            result.push_str(&sanitized);
            result.push('；');
        }
        if result.len() == header_len {
            result.truncate(mark);
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


fn personality_to_sentence(p: &super::Personality) -> String {
    let bold_desc = if p.boldness < 0.28 {
        "行事極度沉斂慎重、戒備心重"
    } else if p.boldness < 0.42 {
        "行事謹慎戒備、不輕易涉險"
    } else if p.boldness < 0.58 {
        "作風沉著穩妥、不溫不火"
    } else if p.boldness < 0.72 {
        "性格爽朗果斷、勇於任事"
    } else {
        "氣魄大膽豪邁、敢衝敢闖"
    };

    let ord_desc = if p.orderliness < 0.28 {
        "天性不拘小節、隨遇而安不喜拘束"
    } else if p.orderliness < 0.42 {
        "處世率性自然、靈活變通不受條規約束"
    } else if p.orderliness < 0.58 {
        "進退知曉分寸、圓融通達"
    } else if p.orderliness < 0.72 {
        "辦事一絲不苟、遵循章法規矩"
    } else {
        "恪守綱紀度數、言行嚴謹端方"
    };

    let sens_desc = if p.sensitivity < 0.28 {
        "情緒極度內斂深沉、言語精簡冷靜"
    } else if p.sensitivity < 0.42 {
        "待人言辭務實簡明、情緒內斂少語"
    } else if p.sensitivity < 0.58 {
        "待人客氣得體、應對合宜"
    } else if p.sensitivity < 0.72 {
        "心思敏銳細膩、善於察言觀色"
    } else {
        "心神極其敏銳洞察、口吻溫暖熱絡"
    };

    // 主次張力平衡描述
    let tension = if p.boldness >= p.sensitivity && p.boldness >= p.orderliness {
        if p.sensitivity >= p.orderliness {
            "在勇毅任事與敏銳體察間相輔相成，不拘泥於死板條規"
        } else {
            "敢作敢當而綱紀嚴明，剛直中見方圓"
        }
    } else if p.sensitivity >= p.boldness && p.sensitivity >= p.orderliness {
        if p.boldness >= p.orderliness {
            "心性洞察入微且不乏果敢斷腕之氣，重情亦重義"
        } else {
            "心思極其細密周延，謹言慎行、顧全大局"
        }
    } else {
        if p.boldness >= p.sensitivity {
            "恪守規矩綱紀之餘兼具魄力，行止有節、雷厲風行"
        } else {
            "條理井然且體恤人微，方正不阿中懷有一份溫柔"
        }
    };

    format!("你的性格秉性為：{}，{}，{}。心性維度守衡，{}。", bold_desc, ord_desc, sens_desc, tension)
}

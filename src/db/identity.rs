pub fn is_routine_physiological_thought(flavor: &str) -> bool {
    const ROUTINE_PATTERNS: &[&str] = &[
        "飲過清冽甘水",
        "飲過茶酒歇腳",
        "用過熱騰騰的茶飯",
        "買碗清湯粗麵",
        "買兩個粗麥餅",
        "買碗草木涼茶",
        "灌大碗解暑粗茶",
        "灌滿隨身竹水筒",
        "採買米菜自炊熱粥",
        "取用清冽涼水",
        "掬清泉冷水",
        "舀半瓢涼水",
        "舀了半瓢",
        "閉目歇腳喘口氣",
        "坐街邊樹蔭木椅擦汗歇腿",
        "在屋舍吹燈安穩睡一覺",
        "於居所木榻和衣閉目歇息",
        "洗淨擦拭、重新紮緊",
        "在案頭做常規修整校準",
        "請醫工切脈調理氣息",
        "順口向路過街坊打聽",
        "和熟人應對幾句日常瑣務",
        "同熟識的鄰里寒暄幾句家常",
    ];
    ROUTINE_PATTERNS.iter().any(|p| flavor.contains(p))
}

// NPC identity 字串組裝（對齊既有 `db/backstory` `BuildIdentity`）。

use crate::store;

use super::{
    get_active_thoughts, get_npc_person_display_name, get_personality_for_entity, get_recent_events,
};

fn format_npc_event_for_identity_prompt(event_type: &str, raw_payload: &str) -> Option<String> {
    let p = raw_payload.trim();
    if p.is_empty()
        || p.contains('|')
        || p.contains('｜')
        || p.contains(':')
        || p.contains('_')
        || p.chars().any(|c| c.is_ascii_alphabetic())
        || p.chars().all(|c| c.is_ascii_digit() || c == '-')
        || p.contains("青籠關")
        || p.contains("林緣聚地")
    {
        return None;
    }
    match event_type {
        "trade" | "work" | "contact" => Some(p.to_string()),
        "talk" => Some(format!("與{p}寒暄交談")),
        "npc_workshop_craft"
        | "refine_fiber"
        | "refine_hide_fish"
        | "refine_herb"
        | "refine_mineral" => Some(format!("親手製成「{p}」")),
        "npc_workshop_mutation" | "craft_mutation" => Some(format!("造物凝出「{p}」")),
        _ => None,
    }
}

/// 組裝 NPC 給 LLM 用的身份描述：真名、職稱、場所、性格、心境、最近事件、摘要（對齊既有 `BuildIdentity`）。
#[must_use]
pub fn build_identity(entity_id: &str) -> String {
    let now_unix = crate::game::now_unix();
    let (_, game_h, game_m, _, _) = crate::store::current_game_clock_full();
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
        let is_peninsula = crate::server::grid_manager::get_runtime_square_grid()
            .is_some_and(|g| g.has_peninsula_wilderness());
        let slot = crate::store::wenyao_household_slot_for(entity_id);
        let sched_home_work = crate::server::grid_manager::resident_schedule_home_and_work(entity_id);
        let role_label = crate::store::npc_persistent_role_label(entity_id, &ent.display_title);
        let role = if !role_label.is_empty() {
            role_label
        } else if is_peninsula && !slot.brief_tag.is_empty() && slot.brief_tag != "街坊居民" {
            slot.brief_tag.to_string()
        } else if !occ.is_empty() && !occ.contains("服務生") {
            occ.clone()
        } else {
            "行旅".to_string()
        };
        effective_occupation = role.clone();

        let work_name = if let Some((_, work_room)) = sched_home_work {
            work_room.to_string()
        } else if is_peninsula && !slot.work_name.is_empty() {
            slot.work_name.to_string()
        } else if let Some(v_name) = venue_name_opt.as_ref() {
            if !role.contains("服務生") && !occ.contains("服務生") {
                v_name.clone()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if !work_name.is_empty() {
            result = format!("你是{person}，{gender}，職稱為{role}，在{work_name}任事。");
        } else {
            result = format!("你是{person}，{gender}，身分為{role}。");
        }

        if is_peninsula {
            let home_name = if let Some((h_room, _)) = sched_home_work {
                h_room
            } else {
                slot.home_name
            };
            result.push_str(&format!(
                "你的穿著外觀：{}。平日住所為【{}】，任職單位為【{}】（職司：{}）。",
                slot.attire_line,
                home_name,
                work_name,
                slot.work_purpose,
            ));
        }
        if !live_act.is_empty() {
            result.push_str(&format!("你當前的具體活動：{}。", live_act));
            let (start_hour, end_hour) = if game_h >= slot.morning_hour && game_h < slot.evening_hour {
                (slot.morning_hour.clamp(0, 23) as u8, slot.evening_hour.clamp(0, 24) as u8)
            } else {
                (slot.evening_hour.clamp(0, 23) as u8, slot.morning_hour.clamp(0, 24) as u8)
            };
            result.push_str(&format!("{}。", crate::store::format_schedule_window_sense(start_hour, end_hour, game_h, game_m)));
            if let Some(spatial_sense) = crate::store::describe_npc_spatial_temporal_state(entity_id) {
                result.push_str(&spatial_sense);
            }
        }

        let inv_snap = super::get_inventory(&ent.inventory, ent.vit);
        let inv_desc: Vec<String> = inv_snap
            .items
            .iter()
            .filter(|it| it.item_id != "blank_manual" && it.item_id != "wt_blank_manual")
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
        let is_peninsula = crate::server::grid_manager::get_runtime_square_grid()
            .is_some_and(|g| g.has_peninsula_wilderness());
        let slot = crate::store::wenyao_household_slot_for(entity_id);
        let sched_hw = crate::server::grid_manager::resident_schedule_home_and_work(entity_id);
        let effective_occ = if !occ.is_empty() && !occ.contains("服務生") {
            occ.clone()
        } else if is_peninsula && !slot.brief_tag.is_empty() && slot.brief_tag != "街坊居民" {
            slot.brief_tag.to_string()
        } else {
            "居民".to_string()
        };
        if is_peninsula {
            let (sched_home, sched_work) = match sched_hw {
                Some((h, w)) => (h, w),
                None => (slot.home_name, slot.work_name),
            };
            let effective_work = if !sched_work.is_empty() {
                sched_work
            } else if !effective_occ.contains("服務生") {
                &v_name
            } else {
                ""
            };
            let effective_home = if !sched_home.is_empty() {
                sched_home
            } else {
                slot.home_name
            };
            if let Some((h_room, w_room)) = sched_hw {
                let settlement_prefix = h_room.split('・').next().unwrap_or("文曜");
                result = format!("{settlement_prefix}的{effective_occ}，平素於【{w_room}】當值，居於【{h_room}】。");
            } else if !effective_work.is_empty() {
                result = format!("你是{person}，職稱是{effective_occ}，在{effective_work}工作。住所為【{effective_home}】。");
            } else {
                result = format!("你是{person}，住所為【{effective_home}】。");
            }
        } else {
            result = format!("你是{person}，身處【{v_name}】。");
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
    let thoughts = get_active_thoughts(entity_id, 25);
    if !thoughts.is_empty() {
        let mut added = 0usize;
        let mut buf = String::new();
        for t in &thoughts {
            if let Some(f) = &t.flavor {
                let trimmed = f.trim();
                if !trimmed.is_empty()
                    && !trimmed.contains("青籠關")
                    && !trimmed.contains("林緣聚地")
                    && !is_routine_physiological_thought(trimmed)
                {
                    let tag = crate::store::format_short_relative_time(t.created_at, now_unix);
                    buf.push_str(&format!("{tag}{trimmed}；"));
                    added += 1;
                    if added >= 3 {
                        break;
                    }
                }
            }
        }
        if !buf.is_empty() {
            result.push_str("你最近的心境念頭：");
            result.push_str(&buf);
        }
    }
    let evts = get_recent_events(entity_id, 5);
    if !evts.is_empty() {
        let mark = result.len();
        result.push_str("你最近的經歷動向：");
        let header_len = result.len();
        for e in evts {
            let Some(formatted) = format_npc_event_for_identity_prompt(&e.event_type, &e.payload) else {
                continue;
            };
            let tag = crate::store::format_short_relative_time(e.at, now_unix);
            result.push_str(&format!("{tag}{formatted}；"));
        }
        if result.len() == header_len {
            result.truncate(mark);
        }
    }
    if crate::server::grid_manager::get_runtime_square_grid()
        .is_some_and(|g| g.has_peninsula_wilderness())
    {
        let pulse = crate::game::object_ecology::market_pulse_summary_for_prompt();
        if !pulse.is_empty() {
            result.push_str(&format!("當前坊市經濟脈動：{}。", pulse));
        }
        let facts = crate::pg::living_world::recent_resident_knowledge_facts(entity_id, 3);
        if !facts.is_empty() {
            result.push_str(&format!("\n【個人親歷與坊間熟知】：{}", facts.join("；")));
        }
        let s = npc_summary.trim();
        if !s.is_empty() && (s.contains("開譜曆") || s.contains("白塔曆")) {
            result.push_str(&format!("\n近期交談摘要：{}", s));
        }
        let slot = crate::store::wenyao_household_slot_for(entity_id);
        let sched_hw = crate::server::grid_manager::resident_schedule_home_and_work(entity_id);
        let (vessel_id, vessel_name) = if sched_hw.is_some() {
            let spec = crate::npc::causal_core::resolve_vocational_vessel(&effective_occupation);
            (spec.catalog_id, spec.display_name)
        } else {
            let (vid, vname) = crate::store::held_vessel(slot);
            if vname.is_empty() || vid.is_empty() {
                let spec = crate::npc::causal_core::resolve_vocational_vessel(&effective_occupation);
                (spec.catalog_id, spec.display_name)
            } else {
                (vid, vname)
            }
        };
        let (final_home, final_work) = if let Some((h, w)) = sched_hw {
            (h, w)
        } else {
            (slot.home_name, slot.work_name)
        };
        result.push_str(&format!(
            "\n【閉世界事實白名單（僅准引用以下真實外鍵，嚴禁無中生有）】住處：【{}】（{},{}）；本職場所：【{}】（{},{}）；常去採買／交接場所：【{}】（{},{}）；職司隨身器物：【{}】（{}）。對話提及之店鋪、街巷或器物必須 100% 限於上述白名單、隨身行囊或現場四鄰實景；未列於白名單之店鋪或物品代表不存在，必須回答「不曉得」或「這附近沒有」！",
            final_home, slot.home_x, slot.home_y,
            final_work, slot.work_x, slot.work_y,
            slot.errand_name, slot.errand_x, slot.errand_y,
            vessel_name, vessel_id
        ));
    }
    result
}

fn personality_descriptors(p: &super::Personality) -> (&'static str, &'static str, &'static str, &'static str) {
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

    (bold_desc, ord_desc, sens_desc, tension)
}

fn personality_to_sentence(p: &super::Personality) -> String {
    let (bold_desc, ord_desc, sens_desc, tension) = personality_descriptors(p);
    format!("你的性格秉性為：{}，{}，{}。心性維度守衡，{}。", bold_desc, ord_desc, sens_desc, tension)
}

fn personality_to_third_person_sentence(p: &super::Personality) -> String {
    let (bold_desc, ord_desc, sens_desc, tension) = personality_descriptors(p);
    format!("性格秉性：{}，{}，{}。心性維度守衡，{}。", bold_desc, ord_desc, sens_desc, tension)
}

/// 為 NPC↔NPC 雙人對話組裝嚴格第三人稱角色卡（徹底隔離玩家專屬來客印象與第二人稱角色糾纏）。
#[must_use]
pub fn build_third_person_actor_card(
    entity_id: &str,
    role_tag: &str,
    peer_name: &str,
) -> String {
    let mut name = get_npc_person_display_name(entity_id).unwrap_or_default();
    if name.is_empty() {
        name = entity_id.to_string();
    }
    let occ = super::assignment::get_npc_title_from_assignments(entity_id);

    let (ent_opt, live_act, npc_summary) = if let Some(arc) = store::get_store() {
        let s = arc.read();
        let ent_opt = s.get_entity(entity_id);
        let live_act = ent_opt
            .as_ref()
            .map(|e| s.effective_npc_activity(e))
            .unwrap_or_default();
        let summary = s.get_npc_summary(entity_id);
        (ent_opt, live_act, summary)
    } else {
        (None, String::new(), String::new())
    };

    let mut gender = "男";
    let mut soul_seed_val = None;
    let mut role_label = String::new();
    if let Some(ent) = &ent_opt {
        soul_seed_val = ent.soul_seed;
        let is_female = crate::store::npc_effective_is_female(entity_id, Some(&ent.gender));
        gender = if is_female { "女性" } else { "男性" };
        name = crate::store::resolve_npc_person_name(entity_id, &ent.display_title, Some(&ent.gender));
        role_label = crate::store::npc_persistent_role_label(entity_id, &ent.display_title);
    }

    let is_peninsula = crate::server::grid_manager::get_runtime_square_grid()
        .is_some_and(|g| g.has_peninsula_wilderness());
    let slot = crate::store::wenyao_household_slot_for(entity_id);
    let sched_hw = crate::server::grid_manager::resident_schedule_home_and_work(entity_id);
    let job_label = if !role_label.is_empty() {
        role_label
    } else if is_peninsula && !slot.brief_tag.is_empty() && slot.brief_tag != "街坊居民" {
        slot.brief_tag.to_string()
    } else if !occ.is_empty() && !occ.contains("服務生") {
        occ
    } else {
        "行旅".to_string()
    };

    let mut card = if is_peninsula {
        let clothing_desc = slot.attire_line;
        let (home_name, work_name) = if let Some((h, w)) = sched_hw {
            (h, w)
        } else {
            (slot.home_name, slot.work_name)
        };
        let work_purpose = slot.work_purpose;
        format!(
            "【{role_tag}：{name}】（只可自稱「我」，不可自呼「{name}」或自身外號；對面交談對象是「{peer_name}」）｜性別：{gender}｜職稱：{job_label}｜穿戴：{clothing_desc}｜住所：【{home_name}】｜任事場所：【{work_name}】（{work_purpose}）"
        )
    } else {
        format!(
            "【{role_tag}：{name}】（只可自稱「我」，不可自呼「{name}」或自身外號；對面交談對象是「{peer_name}」）｜性別：{gender}｜身分：{job_label}"
        )
    };
    let personality_opt = get_personality_for_entity(entity_id).or_else(|| {
        if let Some(seed) = soul_seed_val
            && seed != 0
        {
            Some(super::expand_soul_seed_to_personality(seed))
        } else {
            None
        }
    });
    if let Some(p) = personality_opt {
        card.push('\n');
        card.push_str(&personality_to_third_person_sentence(&p));
    }

    let seed = crate::npc::causal_core::deterministic_resident_seed(
        entity_id,
        soul_seed_val,
    );
    let myriad_phase = crate::npc::causal_core::describe_npc_myriad_personality_phase(
        entity_id,
        &job_label,
        seed,
    );
    if !myriad_phase.is_empty() {
        card.push('\n');
        card.push_str(&format!("心性相貌：{}。", myriad_phase));
    }

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
        card.push('\n');
        card.push_str(&format!(
            "當前手頭待辦義務或牽掛：{}（{}）。",
            ob_label, ob_subject
        ));
    }

    let facts = crate::pg::living_world::recent_resident_knowledge_facts(entity_id, 2);
    if !facts.is_empty() {
        card.push('\n');
        card.push_str(&format!("個人親歷與坊間熟知：{}。", facts.join("；")));
    }

    let thoughts = get_active_thoughts(entity_id, 25);
    let mut clean_thoughts = Vec::new();
    let card_now_unix = crate::game::now_unix();
    for t in &thoughts {
        if let Some(f) = &t.flavor {
            let trimmed = f.trim();
            if !trimmed.is_empty()
                && !trimmed.contains("青籠關")
                && !trimmed.contains("林緣聚地")
                && !trimmed.contains("早先")
                && !is_routine_physiological_thought(trimmed)
            {
                let tag = crate::store::format_short_relative_time(t.created_at, card_now_unix);
                clean_thoughts.push(format!("{tag}{trimmed}"));
                if clean_thoughts.len() >= 3 {
                    break;
                }
            }
        }
    }
    if !clean_thoughts.is_empty() {
        card.push('\n');
        card.push_str(&format!("近期心境念頭：{}。", clean_thoughts.join("；")));
    }
    let s = npc_summary.trim();
    if !s.is_empty() && (s.contains("開譜曆") || s.contains("白塔曆")) {
        card.push_str(&format!("\n近期交談摘要：{}", s));
    }

    if is_peninsula {
        let work_name = sched_hw.map(|(_, w)| w).unwrap_or(slot.work_name);
        let effective_live_act = if live_act.is_empty() || is_routine_physiological_thought(&live_act) {
            slot.work_stay_activity
        } else {
            live_act.as_str()
        };
        if !effective_live_act.is_empty() {
            card.push('\n');
            card.push_str(&format!(
                "當前手頭活計：{}（嚴守本職【{}】，切勿自稱在其他診所或非自身場所當差）。",
                effective_live_act, work_name
            ));
        }
    } else if !live_act.is_empty() {
        card.push('\n');
        card.push_str(&format!("當前手頭活動：{}。", live_act));
    }
    card
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_routine_physiological_thoughts_but_keeps_expedition_and_work() {
        assert!(is_routine_physiological_thought(
            "在文曜・三水口水岸公井飲過清冽甘水，喉間乾渴頓消，精神一振"
        ));
        assert!(is_routine_physiological_thought(
            "在南哨・脈絡解毒診所飲過茶酒歇腳，喉間舒爽，燥意盡消"
        ));
        assert!(is_routine_physiological_thought(
            "在南哨・脈絡解毒診所把隨身的「紫竹家常提籃」洗淨擦拭、重新紮緊，器況煥然一新，用起來順手多了"
        ));
        assert!(!is_routine_physiological_thought(
            "隨領隊【廖佩玉】合社獵團將圍獵所獲之【深潭野生老鱉】運抵風舶角・海事總商行與船貨契據所過磅交割，分得紅利 36 枚鎂幣"
        ));
        assert!(!is_routine_physiological_thought(
            "手頭鎂幣漸緊，得盤算多接幾趟差事"
        ));
    }

    #[test]
    fn test_build_identity_filters_blank_manual_from_npc_inventory() {
        let inv_raw = r#"[{"item_id":"blank_manual","qty":2},{"item_id":"wt_blank_manual","qty":1},{"item_id":"mat_pine_plank","qty":5}]"#;
        let snap = super::super::get_inventory(inv_raw, 10);
        let filtered: Vec<_> = snap
            .items
            .iter()
            .filter(|it| it.item_id != "blank_manual" && it.item_id != "wt_blank_manual")
            .map(|it| it.item_id.clone())
            .collect();
        assert_eq!(filtered, vec!["mat_pine_plank"]);
    }
}

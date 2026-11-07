//! `do_action` — Look 分支：打量目標，輸出外觀敘述。

use crate::db;
use crate::entity::Character;
use crate::event::{self, types};
use crate::game::final_map_observation_template_for_room;

use super::super::handler::WsConnection;
use super::super::protocol::ActionResultMsg;

pub(super) fn do_entity_look(
    conn: &WsConnection,
    pid: &str,
    now: i64,
    target_id: &str,
    target: &Character,
) {
    let room_name = super::resolve_action_room_name(pid, target);
    let target_name = super::player_visible_target_name(pid, target);
    let mut narrative = build_look_narrative(target, &target_name, &room_name, Some(pid));

    let active_dialogue = crate::store::get_active_npc_dialogue(target_id);

    if let Some(active) = active_dialogue {
        if !narrative.contains(&active.text) {
            narrative.push_str(&format!("\n\n【正在交談】\n{}", active.text));
        }
    }

    let actions = followup_actions(pid, target);
    if target_id != pid
        && !pid.starts_with("npc_")
        && !target_id.starts_with("herb_")
        && !target_id.starts_with("pine_")
        && !target_id.starts_with("spring_")
        && !target_id.starts_with("wild_")
        && !target_id.starts_with("ambient_")
        && !target_id.starts_with("shanhai_")
    {
        let _ = event::append(now, pid, types::OBSERVED, &target_name);
    }
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "Look".into(),
        target_id: target.id.clone(),
        target_name,
        narrative,
        success: true,
        actions,
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}


/// 過濾 NPC 當前行動中的 Azgaar 方位與地貌括號片段（如 `（北側 · 水澤林）` 或 `（西側・凍原）`）
fn sanitize_activity_for_display(act: &str) -> String {
    let mut result = String::with_capacity(act.len());
    let mut chars = act.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == '（' || ch == '(' {
            let closer = if ch == '（' { '）' } else { ')' };
            let rest = &act[idx..];
            if let Some(close_offset) = rest.find(closer) {
                let inside = &rest[..close_offset + closer.len_utf8()];
                if inside.contains("側 · ") || inside.contains("側・") || inside.contains("WS-") || inside.contains("日常劇本") {
                    // 跳過整個括號區間
                    let skip_bytes = inside.len();
                    let target_pos = idx + skip_bytes;
                    while let Some(&(next_idx, _)) = chars.peek() {
                        if next_idx < target_pos {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    continue;
                }
            }
        }
        result.push(ch);
    }
    result
        .replace("茶肆", "茶屋")
        .replace("客棧", "旅社")
        .replace("熟絡", "熱絡")
        .replace("床榻", "床鋪")
        .replace("榻上", "床鋪上")
}
fn format_activity_sentence(pronoun: &str, act: &str) -> String {
    let trimmed = act.trim().trim_end_matches(['。', '！', '？', '，', ' ']);
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("正在") || trimmed.starts_with("正") {
        format!("{pronoun}{trimmed}。")
    } else {
        format!("{pronoun}目前{trimmed}。")
    }
}

fn build_look_narrative(
    target: &Character,
    name: &str,
    room_name: &str,
    viewer_pid: Option<&str>,
) -> String {
    if target.id.starts_with("ambient_") || target.id.starts_with("shanhai_") {
        if let Some((_loot_id, loot_name, shanhai_note)) =
            crate::game::ambient_creature_loot(&target.display_title)
        {
            let stat_prose = if target.vit >= 100 {
                "觀其體態氣機，體魄雄渾厚重、周身威壓沉凝，透出鎮懾整片山野之威勢。"
            } else if target.vit >= 50 {
                if target.dex >= 25 {
                    "觀其體態氣機，體魄壯碩矯捷、身法迅猛如電，隱隱透出凜然凶悍之勢。"
                } else {
                    "觀其體態氣機，體魄雄厚粗壯、步伐沉重有力，透出凶悍野性之勢。"
                }
            } else if target.vit >= 25 {
                if target.dex >= 20 {
                    "觀其體態氣機，骨骼健拔勻稱、奔走機警自如，氣血沉穩內斂。"
                } else {
                    "觀其體態氣機，身軀敦實安詳、進退沉穩從容，生機豐沛自得。"
                }
            } else if target.dex >= 26 {
                "觀其體態，羽翼輕靈矯健、動靜飄忽迅疾，氣息平緩自然。"
            } else {
                "觀其體態，身形輕靈小巧、舉止機警敏捷，氣息平緩自然。"
            };
            let mut desc = format!(
                "你屏息凝神，仔細觀察眼前的山海生靈【{}】。{}\n{}",
                target.display_title, shanhai_note, stat_prose
            );
            let act_clean = target.current_activity.trim().trim_end_matches(['。', '！', '？', '，', ' ']);
            if !act_clean.is_empty() {
                desc.push_str(&format!(" 牠{}。", act_clean));
            }
            let target_coord = (target.grid_x.unwrap_or(0), target.grid_y.unwrap_or(0));
            {
                let hunting_parties = crate::store::LIVE_HUNTING_PARTIES.read();
                if let Some(party) = hunting_parties
                    .values()
                    .find(|p| p.prey_coord == target_coord || p.prey_name == target.display_title)
                {
                    desc.push_str(&format!(
                        "\n【合社獵團合圍】領隊【{}】正率 {} 名獵手四面合圍此獸，戰況緊繃！",
                        party.leader_name,
                        party.member_ids.len()
                    ));
                }
            }
            desc.push_str(&format!(
                "\n【山海物產】若將其制服或斬殺，可剝取珍材【{}】或分解溫熱殘軀。",
                loot_name
            ));
            return desc;
        }
    }
    let is_stranger = name.starts_with("陌生人");
    let has_resident_desc = if is_stranger {
        None
    } else {
        resident_appearance_and_demeanor(target, room_name)
    };
    let mut is_known_resident_desc = false;
    let sensory_line = if target.kind == crate::entity::EntityKind::Npc {
        crate::store::npc_sensory_appearance_line(&target.id, &target.display_title)
    } else {
        String::new()
    };
    let combined_name_check = format!("{}|{}", name, target.display_title);
    let is_female_name = [
        "娜", "梅", "芳", "娟", "霞", "婷", "玲", "敏", "靜", "淑", "秀", "麗", "燕", "蘭",
        "珍", "珠", "琴", "萍", "娥", "雅", "佳", "怡", "詩", "涵", "萱", "彤", "晴", "蓁",
        "婉", "芬", "筱", "妤", "彩", "潔", "郁", "馨", "微", "美", "慧", "雪", "月", "雲",
        "蓮", "薇", "珊", "貞", "嬌", "媚", "柔", "蓉", "菲", "芸", "琪", "瑤", "娘", "姑",
        "婆", "嫂", "姐", "妹", "罔市", "秋月",
    ]
    .iter()
    .any(|kw| !combined_name_check.starts_with("陌生人") && combined_name_check.contains(kw));
    let is_female_sensory = sensory_line.contains("裙")
        || sensory_line.contains("簪")
        || sensory_line.contains("髮髻")
        || sensory_line.contains("娘")
        || sensory_line.contains("小姐")
        || sensory_line.contains("婦")
        || sensory_line.contains("溫婉")
        || sensory_line.contains("靈秀")
        || sensory_line.contains("紅絲帶");
    let pronoun = if is_female_sensory || is_female_name || target.gender == Some(crate::entity::Gender::F) {
        "她"
    } else {
        "他"
    };
    let mut desc = if let Some(rdesc) = has_resident_desc.as_ref() {
        is_known_resident_desc = true;
        let final_resident_desc = if name == "陌生人" && !target.display_title.is_empty() {
            rdesc.replace(&target.display_title, "眼前之人")
        } else {
            rdesc.clone()
        };
        format!("你仔細打量了【{name}】。{final_resident_desc}")
    } else if !sensory_line.trim().is_empty() {
        let build_bearing = if sensory_line.contains("溫婉")
            || sensory_line.contains("靈秀")
            || sensory_line.contains("裙")
            || sensory_line.contains("纖細")
            || sensory_line.contains("素雅")
            || sensory_line.contains("秀氣")
        {
            "身段勻稱，步履輕巧從容"
        } else if sensory_line.contains("書卷")
            || sensory_line.contains("斯文")
            || sensory_line.contains("防墨")
            || sensory_line.contains("算盤")
            || sensory_line.contains("長衫")
            || sensory_line.contains("輕柔")
        {
            "身形修長，舉止沉穩有度"
        } else if sensory_line.contains("雷厲風行")
            || sensory_line.contains("勁裝")
            || sensory_line.contains("皮靴")
            || sensory_line.contains("磨刀")
            || sensory_line.contains("短打")
        {
            "體態精悍，步履矯健利落"
        } else if sensory_line.contains("鐵")
            || sensory_line.contains("鎚")
            || sensory_line.contains("石")
            || sensory_line.contains("木")
            || sensory_line.contains("壯")
        {
            "體格健壯，步履沉穩有力"
        } else {
            "身材勻稱，步履平穩，精神飽滿"
        };
        format!("你仔細打量了【{name}】。{pronoun}{build_bearing}。")
    } else {
        let physique = match target.vit {
            v if v >= 115 => "體格異常魁梧",
            v if v >= 95 => "體格健壯",
            v if v >= 12 => "身材勻稱",
            _ => "身形清瘦",
        };
        let agility = match target.dex {
            d if d >= 85 => "舉止間透著驚人的敏捷",
            d if d >= 65 => "步履輕快矯健",
            d if d >= 10 => "步履平穩",
            _ => "行動沉緩",
        };
        let aura = match target.qi {
            q if q >= 95 => "，神采奕奕、氣息沉穩悠長",
            q if q >= 12 => "，精神飽滿",
            _ => "",
        };
        format!("你仔細打量了【{name}】。{pronoun}{physique}，{agility}{aura}。")
    };

    if sensory_line.trim().is_empty() {
        let personality = crate::db::get_personality_for_entity(&target.id)
            .or_else(|| target.soul_seed.map(crate::db::expand_soul_seed_to_personality));
        if let Some(p) = personality {
            let bold_str = if p.boldness < 0.5 {
                "神態內斂"
            } else {
                "氣勢沉猛"
            };
            let sens_str = if p.sensitivity >= 0.5 { "目光敏銳" } else { "神色淡然" };
            let order_str = if p.orderliness >= 0.5 {
                "衣物整飭"
            } else {
                "行止隨性"
            };
            desc.push_str(&format!("其{bold_str}、{sens_str}，{order_str}。"));
        }
    }
    let has_outfit = is_known_resident_desc || has_resident_desc.is_some() || !sensory_line.trim().is_empty();
    let mut held_items: Vec<String> = Vec::new();

    if !target.equipment_slots.is_empty() {
        if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(&target.equipment_slots) {
            let slots_map: std::collections::HashMap<String, String> = map
                .into_iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
                .collect();
            let names = db::get_item_names(&target.equipment_slots);

            let is_starter_gear = |id: &str| {
                id.starts_with("starter_")
                    || matches!(
                        id,
                        "粗布短褂" | "粗布長褲" | "草鞋" | "棉麻日常上衣" | "耐磨休閒長褲" | "輕便布鞋"
                    )
            };

            // 1. 若無專屬服飾，顯示基本服飾穿戴（避免與特定 NPC 訂製外貌描寫衝突）
            if !has_outfit {
                let pieces: Vec<String> = ["body", "legs", "feet"]
                    .iter()
                    .filter_map(|&slot| {
                        let id = slots_map.get(slot)?;
                        if id.is_empty() { return None; }
                        let name = names.get(slot)?;
                        Some(match name.as_str() {
                            "粗布短褂" | "棉麻日常上衣" => "日常上衣".to_string(),
                            "粗布長褲" | "耐磨休閒長褲" => "耐磨長褲".to_string(),
                            "草鞋" => "輕便布鞋".to_string(),
                            other => other.to_string(),
                        })
                    })
                    .collect();
                if !pieces.is_empty() {
                    desc.push_str("只見其身穿");
                    desc.push_str(&pieces.join("、"));
                    desc.push('。');
                }
            }

            // 2. 持握工具 / 武器（無論是否有專屬服飾皆顯示器物）
            let held_slots = ["hold_r", "weapon", "hand", "tool", "hold_l"];
            for &slot in &held_slots {
                if let Some(item_id) = slots_map.get(slot) {
                    if !item_id.is_empty() && !is_starter_gear(item_id) {
                        let ecol_name = crate::game::object_ecology::lookup_ecology_item_def(item_id).map(|d| d.name);
                        let base_name = names
                            .get(slot)
                            .map(|s| s.as_str())
                            .or(ecol_name.as_deref())
                            .unwrap_or(item_id);
                        let clean_public_item = |raw: &str| -> String {
                            let mut s = raw.trim().to_string();
                            while let Some(start) = s.find("【異變") {
                                if let Some(end_rel) = s[start..].find('】') {
                                    let after = start + end_rel + '】'.len_utf8();
                                    let rest = s[after..].trim_start_matches('・').trim();
                                    s = format!("{}{}", &s[..start], rest);
                                } else {
                                    break;
                                }
                            }
                            let s = s.trim_matches(|c| c == '【' || c == '】').trim();
                            for prefix in ["精鍛", "百煉", "特製", "加固", "防身"] {
                                if s.starts_with(prefix) && s.chars().count() > prefix.chars().count() + 1 {
                                    return s[prefix.len()..].to_string();
                                }
                            }
                            s.to_string()
                        };
                        let cleaned = clean_public_item(base_name);
                        let item_label = crate::server::handler::format_entity_item_with_inscription(
                            &target.id,
                            item_id,
                            &cleaned,
                        );
                        if crate::pg::living_world::is_weapon_or_tool_item(item_id, base_name) {
                            let mut vstate = crate::pg::living_world::get_vessel_state(&target.id, item_id);
                            if vstate.use_count == 0 {
                                let init_use = if target.kind == crate::entity::EntityKind::Npc {
                                    4 + ((target.soul_seed.unwrap_or(0).unsigned_abs() % 12) as i32)
                                } else {
                                    2
                                };
                                vstate = crate::pg::living_world::record_vessel_use(
                                    &target.id,
                                    item_id,
                                    0,
                                    init_use,
                                    crate::game::now_unix(),
                                );
                            }
                            if viewer_pid != Some(target.id.as_str()) {
                                match vstate.condition {
                                    crate::pg::living_world::VesselCondition::SlightlyDull
                                    | crate::pg::living_world::VesselCondition::RolledEdge => {
                                        let cond_lbl = vstate.condition.label_for_item(item_id, &cleaned);
                                        held_items.push(format!("【{item_label}】（{cond_lbl}）"));
                                    }
                                    crate::pg::living_world::VesselCondition::Sharp => {
                                        held_items.push(format!("【{item_label}】"));
                                    }
                                }
                            } else {
                                held_items.push(format!(
                                    "【{item_label}】（{}・{}）",
                                    vstate.familiarity().label(),
                                    vstate.condition.label()
                                ));
                            }
                        } else {
                            held_items.push(format!("【{item_label}】"));
                        }
                    }
                }
            }
            if !held_items.is_empty() {
                desc.push_str(&format!("手裡握著{}。", held_items.join("、")));
            }

            // 3. 非初始裝備之隨身佩戴物（如披風、腰帶、飾品等）
            let clean_public_item_helper = |raw: &str| -> String {
                let mut s = raw.trim().to_string();
                while let Some(start) = s.find("【異變") {
                    if let Some(end_rel) = s[start..].find('】') {
                        let after = start + end_rel + '】'.len_utf8();
                        let rest = s[after..].trim_start_matches('・').trim();
                        s = format!("{}{}", &s[..start], rest);
                    } else {
                        break;
                    }
                }
                let s = s.trim_matches(|c| c == '【' || c == '】').trim();
                for prefix in ["精鍛", "百煉", "特製", "加固", "防身"] {
                    if s.starts_with(prefix) && s.chars().count() > prefix.chars().count() + 1 {
                        return s[prefix.len()..].to_string();
                    }
                }
                s.to_string()
            };

            // 3. 外罩斗篷 / 披風 / 蓑衣
            let mut cloak_items: Vec<String> = Vec::new();
            for &slot in &["cloak", "body"] {
                if let Some(item_id) = slots_map.get(slot) {
                    if !item_id.is_empty()
                        && !is_starter_gear(item_id)
                        && !item_id.contains("storage_box")
                        && !item_id.contains("pine_soot_ink")
                    {
                        let ecol_name = crate::game::object_ecology::lookup_ecology_item_def(item_id).map(|d| d.name);
                        let base_name = names
                            .get(slot)
                            .map(|s| s.as_str())
                            .or(ecol_name.as_deref())
                            .unwrap_or(item_id);
                        let is_cloak_slot = slot == "cloak";
                        let is_outer_cloak = crate::game::object_ecology::ItemPhysicalClass::classify(item_id, base_name)
                            == crate::game::object_ecology::ItemPhysicalClass::OuterCloakOrRobe;
                        if is_cloak_slot || is_outer_cloak {
                            let cleaned = clean_public_item_helper(base_name);
                            let item_label = crate::server::handler::format_entity_item_with_inscription(
                                &target.id,
                                item_id,
                                &cleaned,
                            );
                            cloak_items.push(format!("【{item_label}】"));
                        }
                    }
                }
            }
            if !cloak_items.is_empty() {
                desc.push_str(&format!("外罩{}。", cloak_items.join("、")));
            }

            // 4. 非初始裝備之隨身佩戴飾品（腰帶、佩件、項飾、腕飾、頭飾、背負等）
            let accessory_slots: &[&str] = &["waist", "trinket", "neck", "wrist", "head", "back"];
            let mut worn_items: Vec<String> = Vec::new();
            for &slot in accessory_slots {
                if let Some(item_id) = slots_map.get(slot) {
                    if !item_id.is_empty()
                        && !is_starter_gear(item_id)
                        && !item_id.contains("storage_box")
                        && !item_id.contains("pine_soot_ink")
                    {
                        let ecol_name = crate::game::object_ecology::lookup_ecology_item_def(item_id).map(|d| d.name);
                        let base_name = names
                            .get(slot)
                            .map(|s| s.as_str())
                            .or(ecol_name.as_deref())
                            .unwrap_or(item_id);
                        if base_name.contains("隨身匣")
                            || base_name.contains("收納匣")
                            || base_name.contains("木匣")
                            || base_name.contains("墨錠")
                        {
                            continue;
                        }
                        let cleaned = clean_public_item_helper(base_name);
                        let item_label = crate::server::handler::format_entity_item_with_inscription(
                            &target.id,
                            item_id,
                            &cleaned,
                        );
                        worn_items.push(format!("【{item_label}】"));
                    }
                }
            }
            if !worn_items.is_empty() {
                desc.push_str(&format!("隨身佩著{}。", worn_items.join("、")));
            }
        }
    }
    let hour = crate::store::current_game_hour();
    let is_night = hour >= 22 || hour < 5;
    let live_act = crate::store::LIVE_NPC_GRID_STATES
        .read()
        .get(&target.id)
        .map(|s| s.current_activity())
        .filter(|act| !act.trim().is_empty());
    let raw_act = if !target.current_activity.trim().is_empty() {
        Some(target.current_activity.as_str())
    } else {
        live_act.as_deref()
    };

    if !is_known_resident_desc && target.kind == crate::entity::EntityKind::Npc && !sensory_line.trim().is_empty() {
        let mut clean_sensory = sensory_line.clone();
        if !held_items.is_empty() {
            for hold_prefix in ["，手持", "、手持", "，手握", "、手握", "，手執", "、手執"] {
                if let Some(start_idx) = clean_sensory.find(hold_prefix) {
                    let clause_after = &clean_sensory[start_idx + hold_prefix.len()..];
                    if let Some(comma_rel) = clause_after.find('，') {
                        clean_sensory = format!("{}{}", &clean_sensory[..start_idx], &clause_after[comma_rel..]);
                    } else if let Some(comma_rel) = clause_after.find('、') {
                        clean_sensory = format!("{}{}", &clean_sensory[..start_idx], &clause_after[comma_rel..]);
                    } else {
                        clean_sensory = clean_sensory[..start_idx].to_string();
                    }
                }
            }
        }
        for gait_prefix in ["，步態", "、步態", "，步履", "、步履", "，步伐", "、步伐"] {
            if let Some(start_idx) = clean_sensory.find(gait_prefix) {
                let clause_after = &clean_sensory[start_idx + gait_prefix.len()..];
                if let Some(comma_rel) = clause_after.find('，') {
                    clean_sensory = format!("{}{}", &clean_sensory[..start_idx], &clause_after[comma_rel..]);
                } else if let Some(comma_rel) = clause_after.find('、') {
                    clean_sensory = format!("{}{}", &clean_sensory[..start_idx], &clause_after[comma_rel..]);
                } else {
                    clean_sensory = clean_sensory[..start_idx].to_string();
                }
            }
        }
        if raw_act.is_some() {
            for act_prefix in ["，正", "、正"] {
                if let Some(start_idx) = clean_sensory.find(act_prefix) {
                    clean_sensory = clean_sensory[..start_idx].to_string();
                }
            }
        }
        let clean_sensory = clean_sensory.trim().trim_matches(['，', '、', '。']);
        if !clean_sensory.is_empty() {
            desc.push_str(&format!("只見其{clean_sensory}。"));
        }
    }

    if let Some(act_str) = raw_act {
        let mut sanitized_act = sanitize_activity_for_display(act_str);
        // 修復「正在茶肆小憩」場所錯置
        if sanitized_act.contains("茶屋或食堂小憩") || sanitized_act.contains("茶肆小憩") {
            if room_name.contains("大食堂") || room_name.contains("食堂") {
                sanitized_act = "正在食堂用餐小憩並品嚐熱湯糕點".to_string();
            } else if room_name.contains("茶屋") || room_name.contains("茶寮") {
                sanitized_act = "正在茶屋小憩並品嚐熱茶糕點".to_string();
            } else if room_name.contains("公議堂") || room_name.contains("總商行") || room_name.contains("鍛造") || room_name.contains("工坊") || room_name.contains("鐘樓") || room_name.contains("學院") {
                // 在非飲食場所若為預設茶肆小憩，優先採用該場所專屬觀察敘事
                if let Some(line) = scene_observation_line(room_name, pronoun) {
                    sanitized_act = String::new();
                    desc.push_str(&line);
                } else {
                    sanitized_act = "正在此處稍作小憩".to_string();
                }
            }
        }
        if !sanitized_act.is_empty() {
            let is_transit_act = sanitized_act.contains("動身前往")
                || sanitized_act.contains("步行前往")
                || sanitized_act.contains("沿街步行")
                || sanitized_act.contains("前往【")
                || sanitized_act.contains("回返【")
                || sanitized_act.contains("前往")
                || sanitized_act.contains("返回")
                || sanitized_act.contains("回返")
                || sanitized_act.contains("趕往")
                || sanitized_act.contains("漫步")
                || sanitized_act.contains("穿行")
                || sanitized_act.contains("步行")
                || sanitized_act.contains("推進目標");
            let is_phantom_bridge_stall = (room_name.contains("三水匯流大石橋") || room_name.contains("中心樞紐石橋"))
                && (sanitized_act.contains("攤")
                    || sanitized_act.contains("選購")
                    || sanitized_act.contains("採買")
                    || sanitized_act.contains("購買")
                    || sanitized_act.contains("布莊")
                    || sanitized_act.contains("金屬行")
                    || sanitized_act.contains("線行"));
            if is_transit_act {
                if is_stranger && (sanitized_act.contains('【') || sanitized_act.contains("準備")) {
                    desc.push_str(&format_activity_sentence(pronoun, "正沿街快步穿行，似有差事在身"));
                } else if !sanitized_act.contains('(')
                    && !sanitized_act.contains('（')
                    && !sanitized_act.contains("推進目標")
                    && sanitized_act.chars().count() >= 10
                {
                    desc.push_str(&format_activity_sentence(pronoun, &sanitized_act));
                } else {
                    let vis_walk = if is_night {
                        "正沿著街面夜色邁步穿行，往鄰街一帶行去"
                    } else {
                        "正沿著街面邁步穿行，步履不停地往鄰街一帶行去"
                    };
                    desc.push_str(&format_activity_sentence(pronoun, vis_walk));
                }
            } else if is_phantom_bridge_stall || (is_night && (room_name.contains("三水匯流大石橋") || room_name.contains("中心樞紐石橋"))) {
                let vis_bridge = if is_night {
                    if room_name.contains("三水匯流大石橋") {
                        "正駐足青石橋欄旁，迎著河面晚風凝望三水川夜色"
                    } else {
                        "正沿著青石橋面緩步徐行，在橋頭駐足歇腳"
                    }
                } else {
                    "正駐足青石橋欄旁歇腳，望著橋下三水川匯流碧波"
                };
                desc.push_str(&format_activity_sentence(pronoun, vis_bridge));
            } else if is_night {
                let is_residential_room = room_name.contains("居所")
                    || room_name.contains("住宅")
                    || room_name.contains("民宅")
                    || room_name.contains("宅邸")
                    || room_name.contains("民舍")
                    || room_name.contains("客房")
                    || room_name.contains("內室")
                    || room_name.contains("宿所")
                    || room_name.contains("宿舍");
                let is_sleep = sanitized_act.contains("安歇")
                    || sanitized_act.contains("熟睡")
                    || sanitized_act.contains("就寢")
                    || sanitized_act.contains("熄燈")
                    || sanitized_act.contains("歇息");
                if !is_residential_room || is_sleep {
                    desc.push_str(&format_activity_sentence(pronoun, &sanitized_act));
                } else {
                    desc.push_str("此刻已在屋內床鋪上熄燈就寢安歇。");
                }
            } else {
                desc.push_str(&format_activity_sentence(pronoun, &sanitized_act));
            }
        }
    } else if let Some(line) = scene_observation_line(room_name, pronoun) {
        desc.push_str(&line);
    }
    if target.kind == crate::entity::EntityKind::Npc {
        {
            let parties = crate::store::LIVE_HUNTING_PARTIES.read();
            if let Some(party) = parties.values().find(|p| p.member_ids.contains(&target.id)) {
                desc.push_str(&format!(
                    "正隨【{}】率領之{}合社獵團（共 {} 人）追獵【{}】。",
                    party.leader_name, party.zone_name, party.member_ids.len(), party.prey_name
                ));
            }
        }
        if sensory_line.trim().is_empty() {
            let mood_phrase = match target.disposition {
                ..=-30 => Some("眉宇微蹙，神色間透著幾分鬱結與焦躁"),
                -29..=-10 => Some("神色略顯疲態，似有心事縈繞"),
                15..=39 => Some("神色舒展，氣息平和安穩"),
                40.. => Some("眉目舒朗、神采奕奕，心境頗為暢快"),
                -9..=14 => None,
            };
            if let Some(mood) = mood_phrase {
                desc.push_str(&format!("此刻{mood}。"));
            }
        }
    }

    // 觀測目標肉身創傷或暗疾
    let afflictions = crate::pg::living_world::get_entity_afflictions(&target.id);
    if !afflictions.is_empty() {
        let labels: Vec<&str> = afflictions.iter().map(|a| a.label()).collect();
        desc.push_str(&format!("細看之下，似帶有{}之態。", labels.join("、")));
    }

    append_universal_living_entity_details(target, room_name, is_stranger, &mut desc);

    if let Some(pool) = crate::pg::pool() {
        if let Some(pid) = viewer_pid
            && !pid.is_empty()
        {
            let fav = db::get_favorability(&target.id, pid);
            let rel_opt = crate::pg::living_world::get_relationship_summary(&pool, &target.id, pid).ok();
            let (eff_balance, eff_fam) = if let Some(rel) = rel_opt {
                (
                    rel.balance_score + fav / 10,
                    rel.familiarity_score + if fav > 0 { 1 } else { 0 },
                )
            } else {
                (fav / 10, if fav > 0 { 1 } else { 0 })
            };
            let rel_phrase = if eff_balance <= -3 {
                Some("看向你時神色帶著明顯戒備與舊怨")
            } else if !is_stranger && eff_balance >= 3 {
                Some("看向你時神色熟稔溫和")
            } else if !is_stranger && eff_fam >= 2 {
                Some("對你已有幾分印象")
            } else {
                None
            };
            if let Some(phrase) = rel_phrase {
                desc.push_str(&format!("{pronoun}{phrase}。"));
            }
        }
    } else {
        if let Some(pid) = viewer_pid
            && !pid.is_empty()
        {
            let fav = db::get_favorability(&target.id, pid);
            let eff_balance = fav / 10;
            let eff_fam = if fav > 0 { 1 } else { 0 };
            let rel_phrase = if eff_balance <= -3 {
                Some("看向你時神色帶著明顯戒備與舊怨")
            } else if !is_stranger && eff_balance >= 3 {
                Some("看向你時神色熟稔溫和")
            } else if !is_stranger && eff_fam >= 2 {
                Some("對你已有幾分印象")
            } else {
                None
            };
            if let Some(phrase) = rel_phrase {
                desc.push_str(&format!("{pronoun}{phrase}。"));
            }
        }
    }
    desc
}

fn is_starter_gear_id(id: &str) -> bool {
    id.starts_with("starter_")
        || matches!(
            id,
            "粗布短褂" | "粗布長褲" | "草鞋" | "棉麻日常上衣" | "耐磨休閒長褲" | "輕便布鞋"
        )
}

fn append_universal_living_entity_details(
    target: &Character,
    room_name: &str,
    is_stranger: bool,
    desc: &mut String,
) {
    if !is_stranger {
    let breath = crate::pg::living_world::get_or_derive_breath_habit(
        &target.id,
        &target.display_title,
        target.soul_seed.unwrap_or(42) as i64,
    );
    let habit_label = if breath.dominant == crate::pg::living_world::BreathHabitKind::TideContinuous {
        "潮汐綿長息"
    } else {
        breath.dominant.label()
    };
    if !desc.contains("潮汐綿長息") && !desc.contains(habit_label) {
        desc.push_str(&format!("吐納起落悠長（{}）。", habit_label));
    }

    // 2. 師承造詣
    let imprints = crate::pg::living_world::get_entity_lineage_imprints(&target.id);
    for imp in imprints {
        if !imp.imprint_title.is_empty() && !desc.contains(&imp.imprint_title) {
            desc.push_str(&format!("身負「{}」師承造詣。", imp.imprint_title));
            break;
        }
    }

    // 3. 隨身持握器物磨損與歷練狀態
    let slots_map: std::collections::HashMap<String, String> =
        serde_json::from_str(&target.equipment_slots).unwrap_or_default();
    let held_tool_id = slots_map
        .get("hold_r")
        .or_else(|| slots_map.get("hold_l"))
        .filter(|id| !id.is_empty() && !is_starter_gear_id(id));
    if let Some(tool_id) = held_tool_id {
        let vstate = crate::pg::living_world::get_vessel_state(&target.id, tool_id);
        if vstate.use_count > 0 && !desc.contains("手頭器物已歷") {
            let t_name = crate::db::get_item_name(tool_id);
            let cond_lbl = vstate.condition.label_for_item(tool_id, &t_name);
            desc.push_str(&format!(
                "手頭器物已歷 {} 次運使（{}）。",
                vstate.use_count,
                cond_lbl
            ));
        }
    } else if target.kind == crate::entity::EntityKind::Npc {
        let vessel = crate::npc::causal_core::resolve_vocational_vessel(&format!(
            "{}|{}",
            target.display_title, target.id
        ));
        let inscribed = crate::server::handler::format_entity_item_with_inscription(
            &target.id,
            vessel.item_id,
            vessel.display_name,
        );
        let vstate = crate::pg::living_world::get_vessel_state(&target.id, vessel.item_id);
        if !desc.contains(vessel.display_name) {
            if vstate.use_count > 0 {
                let cond_lbl = vstate.condition.label_for_item(vessel.item_id, vessel.display_name);
                desc.push_str(&format!(
                    "手邊常備本職器物【{}】（已歷 {} 次運使・{}）。",
                    inscribed,
                    vstate.use_count,
                    cond_lbl
                ));
            } else {
                desc.push_str(&format!("手邊常備本職器物【{}】。", inscribed));
            }
        }
    }

    // 4. 隨身攜有之可交易成品或原料（自然量詞：桶、袋、×N）
    if !desc.contains("隨身攜有") {
        let equipped_ids: std::collections::HashSet<String> = slots_map
            .into_values()
            .filter(|v| !v.trim().is_empty())
            .collect();
        let inv = crate::db::get_inventory(&target.inventory, target.vit);
        let stock_parts: Vec<String> = inv
            .items
            .iter()
            .filter(|i| {
                i.qty > 0
                    && !equipped_ids.contains(&i.item_id)
                    && !i.item_id.starts_with("crystal_")
                    && (crate::world::author_economy::is_runtime_good_item(&i.item_id)
                        || crate::db::item_is_tradable(&i.item_id)
                        || i.item_id.starts_with("craft_")
                        || i.item_id.starts_with("wt_cat_")
                        || i.item_id.starts_with("wt-good-")
                        || i.item_id.starts_with("mat_")
                        || i.item_id.starts_with("harvest_")
                        || i.item_id.starts_with("ore_")
                        || i.item_id.starts_with("wood_"))
            })
            .take(3)
            .map(|i| {
                let display_name = if i.item_id == "wt-good-009" || i.name == "精碾白米" {
                    "待磨精碾白米"
                } else {
                    &i.name
                };
                crate::game::object_ecology::ItemPhysicalClass::classify(&i.item_id, display_name)
                    .format_qty(display_name, i.qty)
            })
            .collect();
        if !stock_parts.is_empty() {
            desc.push_str(&format!("隨身攜有{}。", stock_parts.join("、")));
        }
    }
    }

    if target.kind == crate::entity::EntityKind::Npc {
        let mut probe = character_to_store_entity_probe(target);
        if probe.current_activity.trim().is_empty() && !room_name.is_empty() {
            probe.current_activity = room_name.to_string();
        }
        let coord = (target.grid_x.unwrap_or(0), target.grid_y.unwrap_or(0));
        let (has_visible_uniform, in_staff_only_counter_zone) =
            crate::pg::living_world::is_recognizable_room_staff_or_counter_holder_entity(&probe, coord, "");

        if !has_visible_uniform
            && in_staff_only_counter_zone
            && !desc.contains("櫃檯與內場禁區")
        {
            desc.push_str("雖著尋常便服，卻能自由出入非外客可入之櫃檯與內場禁區，顯係此處主事之人。");
        } else if has_visible_uniform
            && in_staff_only_counter_zone
            && !desc.contains("櫃檯與內場禁區")
        {
            desc.push_str("身穿當值裝束、正立於櫃檯與內場禁區之內當值主事。");
        } else if has_visible_uniform
            && !in_staff_only_counter_zone
            && !desc.contains("身著當值制服")
        {
            desc.push_str("身著明顯的當值工作制服，正於場內料理店務與百工差事。");
        }

        if (has_visible_uniform || in_staff_only_counter_zone) && !room_name.trim().is_empty() {
            if let Some((ev, _sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
                room_name,
                None,
                crate::game::now_unix() as i64,
            ) {
                if !desc.contains(&ev.event_title) {
                    let primary_obj = ev.key_objects.split('、').next().unwrap_or(ev.key_objects).trim();
                    desc.push_str(&format!(
                        "此刻正當值料理「{}」，手頭備有{}。",
                        ev.event_title,
                        primary_obj
                    ));
                }
            }
        }
    }

    // 5. 離線玩家日常行事傾向（純世界內語境，不暴露第四面牆系統詞）
    if target.kind == crate::entity::EntityKind::Player
        && !crate::server::session::list_online_player_ids().contains(&target.id)
    {
        let directive = crate::npc::causal_core::get_player_offline_directive(&target.id);
        let dir_label = directive.label();
        if !target.current_activity.contains(dir_label) && !desc.contains(dir_label) {
            desc.push_str(&format!("平日多留心於「{}」諸務。", dir_label));
        }
    }
}

include!("look/details.rs");

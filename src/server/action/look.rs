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
    let has_canon_desc = if is_stranger {
        None
    } else {
        canonical_npc_appearance_and_demeanor(target, room_name)
    };
    let mut is_known_canon = false;
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
    let mut desc = if let Some(canon) = has_canon_desc.as_ref() {
        is_known_canon = true;
        let final_canon = if name == "陌生人" && !target.display_title.is_empty() {
            canon.replace(&target.display_title, "眼前之人")
        } else {
            canon.clone()
        };
        format!("你仔細打量了【{name}】。{final_canon}")
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
    let has_outfit = is_known_canon || has_canon_desc.is_some() || !sensory_line.trim().is_empty();

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
            let mut held_items: Vec<String> = Vec::new();
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
                                    crate::pg::living_world::VesselCondition::SlightlyDull => {
                                        held_items.push(format!("【{item_label}】（刃口微鈍）"));
                                    }
                                    crate::pg::living_world::VesselCondition::RolledEdge => {
                                        held_items.push(format!("【{item_label}】（刃口微捲）"));
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
            let worn_slots: &[&str] = if has_outfit {
                &["waist", "cloak", "trinket", "neck", "head", "wrist", "leg_armor", "body", "legs", "feet"]
            } else {
                &["waist", "cloak", "trinket", "neck", "head", "wrist", "leg_armor"]
            };
            let mut worn_items: Vec<String> = Vec::new();
            for &slot in worn_slots {
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
                        let s = base_name.trim_matches(|c| c == '【' || c == '】').trim();
                        let item_label = crate::server::handler::format_entity_item_with_inscription(
                            &target.id,
                            item_id,
                            s,
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
    if !is_known_canon && target.kind == crate::entity::EntityKind::Npc && !sensory_line.trim().is_empty() {
        desc.push_str(&format!("只見其{sensory_line}。"));
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
                if !sanitized_act.contains('(')
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
                    if target.id.bytes().fold(0u8, u8::wrapping_add).is_multiple_of(2) {
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

    append_universal_living_entity_details(target, room_name, &mut desc);

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

fn append_universal_living_entity_details(target: &Character, room_name: &str, desc: &mut String) {
    // 1. 內息底色與吐納習慣
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
            desc.push_str(&format!(
                "手頭器物已歷 {} 次運使（{}）。",
                vstate.use_count,
                vstate.condition.label()
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
                desc.push_str(&format!(
                    "手邊常備本職器物【{}】（已歷 {} 次運使・{}）。",
                    inscribed,
                    vstate.use_count,
                    vstate.condition.label()
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
                let unit = if display_name.contains("米漿")
                    || display_name.contains("湯")
                    || display_name.contains("飲")
                    || display_name.contains("油")
                    || display_name.contains("水")
                {
                    "桶"
                } else if display_name.contains("粉")
                    || display_name.contains("米")
                    || display_name.contains("麵")
                    || display_name.contains("穀")
                    || display_name.contains("鹽")
                {
                    "袋"
                } else {
                    ""
                };
                if !unit.is_empty() {
                    format!("{display_name} {} {unit}", i.qty)
                } else {
                    format!("{display_name} ×{}", i.qty)
                }
            })
            .collect();
        if !stock_parts.is_empty() {
            desc.push_str(&format!("隨身攜有{}。", stock_parts.join("、")));
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
            if let Some((ev, sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
                room_name,
                None,
                crate::game::now_unix() as i64,
            ) {
                if !desc.contains(&ev.event_title) {
                    desc.push_str(&format!(
                        "此刻正當值經手場上事件包【{}】（劇本包第 {}/3 幕・關鍵器物：{}）。",
                        ev.event_title,
                        sp.current_step.max(1),
                        ev.key_objects
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

fn character_to_store_entity_probe(target: &Character) -> crate::store::Entity {
    let mut probe = crate::store::Entity::new_npc(&target.id, &target.display_char);
    probe.display_title = target.display_title.clone();
    probe.current_activity = target.current_activity.clone();
    probe.equipment_slots = target.equipment_slots.clone();
    probe.inventory = target.inventory.clone();
    probe.grid_x = target.grid_x;
    probe.grid_y = target.grid_y;
    probe.magnesium = target.magnesium;
    probe
}


fn followup_actions(player_id: &str, target: &Character) -> Vec<String> {
    if target.id.starts_with("shanhai_")
        || (target.id.starts_with("ambient_")
            && crate::game::ambient_creature_loot(&target.display_title).is_some())
    {
        return vec!["Look".into(), "Subdue".into(), "Slay".into()];
    }
    let room_name = super::resolve_action_room_name(player_id, target);
    let named = db::get_acquaintance(player_id, &target.id)
        .ok()
        .flatten()
        .map(|a| a.named)
        .unwrap_or(false);
    if named {
        vec![
            "Talk".into(),
            "Trade".into(),
            "Borrow".into(),
            "Subdue".into(),
            "Slay".into(),
        ]
    } else if super::trade::target_has_shop_offer(target, &room_name)
        || target_is_available_market_operator(player_id, &target.id)
        || {
            let probe = character_to_store_entity_probe(target);
            let coord = (target.grid_x.unwrap_or(0), target.grid_y.unwrap_or(0));
            let (u, c) = crate::pg::living_world::is_recognizable_room_staff_or_counter_holder_entity(&probe, coord, "");
            (u || c) && crate::db::get_inventory(&target.inventory, target.vit).items.iter().any(|i| i.qty > 0)
        }
    {
        vec!["Talk".into(), "Trade".into()]
    } else {
        vec!["Talk".into()]
    }
}

fn canonical_npc_appearance_and_demeanor(target: &Character, room_name: &str) -> Option<String> {
    let hint = format!("{} {}", target.display_title, target.id);
    if let Some(venue) = crate::grid::resolve_room_venue_object(room_name, &hint)
        .or_else(|| crate::grid::resolve_market_or_specialty_room_object(room_name, &hint))
    {
        if !venue.on_duty_demeanor.is_empty() {
            return Some(venue.on_duty_demeanor.to_string());
        }
    }
    None
}

/// 市場交易是場所角色，不應要求玩家先知道經手人的私名才看得到。
/// 只有同地、受僱且此刻能從真實市場庫存提出報價的人才通過。
fn target_is_available_market_operator(player_id: &str, target_id: &str) -> bool {
    let Some(pool) = crate::pg::pool() else {
        return false;
    };
    crate::pg::world_market::offer_for_operator(
        &pool,
        target_id,
        player_id,
        crate::game::now_unix(),
    )
    .ok()
    .flatten()
    .is_some()
}

#[cfg(test)]
fn observed_label_for_room(_raw_name: &str, _room_name: &str) -> String {
    "陌生人".to_string()
}


fn scene_observation_line(room_name: &str, pronoun: &str) -> Option<String> {
    let hour = crate::store::current_game_hour();
    let is_night = !cfg!(test) && (hour >= 22 || hour < 5);
    if is_night {
        let is_watch = room_name.contains("巡防")
            || room_name.contains("哨")
            || room_name.contains("關口")
            || room_name.contains("道口")
            || room_name.contains("水口")
            || room_name.contains("渡口")
            || room_name.contains("門坊")
            || room_name.contains("旅社")
            || room_name.contains("歇舍");
        if is_watch {
            return Some(format!("{pronoun}正提著防風燈籠，在夜色中值守巡看周圍門戶與動靜。"));
        } else {
            return Some(format!("{pronoun}已在內室床鋪上熄燈安歇，四周萬籟俱寂。"));
        }
    }
    if let Some(template) = final_map_observation_template_for_room(room_name) {
        Some(template.replace("{pronoun}", pronoun))
    } else if room_name.contains("半嶺水岸渡口") || room_name.contains("水岸渡口") || room_name.contains("北岸") || room_name.contains("等候") {
        Some(format!(
            "{pronoun}沒有主動搭話，只不時望向渡船踏板，又把身邊的包袱往腳邊收了收。"
        ))
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") || room_name.contains("市集") {
        Some(format!(
            "{pronoun}在價牌前停了停，指尖掂著一只小包貨物，像是在估量要不要趁船來前換些東西。"
        ))
    } else if room_name.contains("歇腳") {
        Some(format!(
            "{pronoun}靠近長凳坐著，茶碗握在手裡，包袱收在腳邊，聽見船繩聲便抬頭看一眼。"
        ))
    } else if room_name.contains("踏板") {
        Some(format!(
            "{pronoun}一手扶著濕木扶手，等船身在水流裡穩下來，腳尖一直避著踏板邊緣的積水。"
        ))
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some(format!(
            "{pronoun}把包袱提高些，鞋底還沾著濕泥，走兩步便回頭看一眼渡船是否還靠在岸邊。"
        ))
    } else if room_name.contains("東南船塢口") {
        Some(format!(
            "{pronoun}把半修的船板靠回木架旁，手上還留著瀝青和木屑的氣味。"
        ))
    } else if room_name.contains("修船架") {
        Some(format!(
            "{pronoun}沿著船肋敲了幾下木楔，又側耳聽船身裡回出的空響。"
        ))
    } else if room_name.contains("補給倉") {
        Some(format!(
            "{pronoun}把淡水桶和乾糧箱重新分成幾堆，低聲核對哪艘船先裝。"
        ))
    } else if room_name.contains("碼頭木階") {
        Some(format!(
            "{pronoun}扶著濕木階等船身晃穩，腳邊的水痕被來往腳步踩亮。"
        ))
    } else if room_name.contains("帆布曬場") {
        Some(format!(
            "{pronoun}用膝蓋壓住被風吹起的舊帆，沿著裂口把針線慢慢收緊。"
        ))
    } else if room_name.contains("船戶街口") {
        Some(format!(
            "{pronoun}在招工牌下比了比工錢，又朝飯攤旁等活的人低聲交代船期。"
        ))
    } else if room_name.contains("外貿碼頭") {
        Some(format!(
            "{pronoun}扶住新卸下的外地貨箱，抬頭看港旗和舶司前棚那邊的手勢。"
        ))
    } else if room_name.contains("舶司前棚") {
        Some(format!(
            "{pronoun}按著濕貨單和封條，等翻譯桌那邊把貨名重新說清。"
        ))
    } else if room_name.contains("商館街口") {
        Some(format!(
            "{pronoun}把貨樣匣打開一角，低聲把新問到的價碼送往匯兌小屋。"
        ))
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        Some(format!(
            "{pronoun}把異地木箱推到雨布內側，分出已查和待查的箱包。"
        ))
    } else if room_name.contains("翻譯桌") {
        Some(format!(
            "{pronoun}指著合約上並排的兩種文字，把陌生貨名轉成在地話。"
        ))
    } else if room_name.contains("匯兌小屋") {
        Some(format!(
            "{pronoun}撥了撥銅秤，將錢票和商館街口報來的價碼寫入帳冊。"
        ))
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        Some(format!(
            "{pronoun}把結霜纜繩重新繞上木樁，又抬頭看避風船是否貼穩碼頭。"
        ))
    } else if room_name.contains("港灣貨棚") {
        Some(format!(
            "{pronoun}把濕魚籃推到鹽箱旁，鞋底在棚門下拖出一道水痕。"
        ))
    } else if room_name.contains("漁網曬場") {
        Some(format!(
            "{pronoun}沿著濕網破口慢慢穿線，膝邊的木架被海風吹得輕響。"
        ))
    } else if room_name.contains("冷霧魚市") {
        Some(format!(
            "{pronoun}掀開濕布讓買貨人看魚色，又把木秤往身邊拉近。"
        ))
    } else if room_name.contains("船戶屋排") {
        Some(format!(
            "{pronoun}在輪值板前換下一枚濕木牌，火盆煙氣貼著袖口散開。"
        ))
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
        Some(format!(
            "{pronoun}扶著濕石欄看向港外海面，指尖停在警鈴繩旁。"
        ))
    } else if room_name.contains("礦坑外棚") {
        Some(format!(
            "{pronoun}把沾泥的鎬靠上木架，抬頭看坑口吊燈是否還亮著。"
        ))
    } else if room_name.contains("洗礦溝") {
        Some(format!(
            "{pronoun}把石籃壓進冷水槽，從混濁水裡分出還能送往粗料倉的礦料。"
        ))
    } else if room_name.contains("粗料倉") {
        Some(format!(
            "{pronoun}在濕礦袋旁補上一道炭線，讓坑口、重量和去向都留在木牌上。"
        ))
    } else if room_name.contains("工具修棚") {
        Some(format!(
            "{pronoun}把鬆掉的鐵圈敲回車輪邊，火星在裂柄鎬旁短短一亮。"
        ))
    } else if room_name.contains("礦渣坡") {
        Some(format!(
            "{pronoun}壓住推車把手，把廢料倒在安全線內，避開新滑坡痕。"
        ))
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
        Some(format!(
            "{pronoun}把繩索掛上礦車車架，又伸手抹開稱重木牌上的灰粉。"
        ))
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some(format!(
            "{pronoun}低頭撥著纜繩與木楔，偶爾抬眼看水勢，像是在等下一個靠船的手勢。"
        ))
    } else if room_name.contains("倉棚") {
        Some(format!(
            "{pronoun}在麻袋與木箱旁停下，手掌拍去袖口的泥點，視線一直留意著棚門和河邊的動靜。"
        ))
    } else if let Some(venue) = crate::grid::resolve_any_room_venue_object(room_name, "") {
        Some(format!("{pronoun}{}。", venue.on_duty_fixture_activity))
    } else {
        None
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    fn test_character() -> Character {
        Character {
            id: "npc_test".to_string(),
            kind: crate::entity::EntityKind::Npc,
            display_char: String::new(),
            x: 0,
            y: 0,
            move_state: crate::entity::MoveState::Idle,
            target_x: None,
            target_y: None,
            walk_or_run: crate::entity::WalkOrRun::Walk,
            move_started_at: None,
            vit: 10,
            qi: 10,
            dex: 10,
            magnesium: 0,
            last_observed_at: None,
            created_at: 0,
            gender: Some(crate::entity::Gender::M),
            soul_seed: None,
            display_title: String::new(),
            activated_nodes: "[]".to_string(),
            equipment_slots: String::new(),
            inventory: "[]".to_string(),
            disposition: 0,
            thoughts: vec![],
            current_activity: String::new(),
            grid_x: None,
            grid_y: None,
        }
    }

    #[test]
    fn look_narrative_has_no_ascii_space_after_sentence_break() {
        let mut ch = test_character();
        ch.current_activity = "整理包袱".to_string();
        let line = build_look_narrative(&ch, "候渡者", "半嶺水岸渡口", None);
        assert!(!line.contains("。 "));
        assert!(line.contains("。他目前整理包袱。"));
    }
    #[test]
    fn generated_private_name_falls_back_to_stranger_label() {
        assert_eq!(observed_label_for_room("王小明", "半嶺水岸渡口"), "陌生人");
    }

    #[test]
    fn public_role_label_is_kept() {
        assert_eq!(observed_label_for_room("船戶", "半嶺水岸渡口"), "陌生人");
    }

    #[test]
    fn market_private_name_falls_back_to_scene_label() {
        assert_eq!(observed_label_for_room("王小明", "水尾寮渡頭市集"), "陌生人");
    }

    #[test]
    fn south_bank_private_name_falls_back_to_scene_label() {
        assert_eq!(observed_label_for_room("王小明", "南岸泥坡"), "陌生人");
    }

    #[test]
    fn warehouse_private_name_falls_back_to_scene_label() {
        assert_eq!(observed_label_for_room("王小明", "倉棚前場"), "陌生人");
    }

    #[test]
    fn m3_050_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("船匠", "東南船塢口"), "陌生人");
        assert_eq!(observed_label_for_room("補帆人", "帆布曬場"), "陌生人");
        assert_eq!(observed_label_for_room("王小明", "船戶街口"), "陌生人");
    }
    #[test]
    fn boathouse_street_look_uses_street_work_not_ferry_work_point() {
        let line = scene_observation_line("船戶街口", "他").expect("船戶街口應有觀察句");

        assert!(line.contains("招工牌"));
        assert!(line.contains("船期"));
        assert!(!line.contains("木樁"));
        assert!(!line.contains("纜繩"));
    }

    #[test]
    fn m3_053_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("舶司吏", "舶司前棚"), "陌生人");
        assert_eq!(observed_label_for_room("商館人", "商館街口"), "陌生人");
        assert_eq!(observed_label_for_room("王小明", "匯兌小屋"), "陌生人");
    }
    #[test]
    fn foreign_trade_look_uses_trade_work_not_generic_passerby() {
        let customs = scene_observation_line("舶司前棚", "他").expect("舶司前棚應有觀察句");
        let exchange = scene_observation_line("匯兌小屋", "她").expect("匯兌小屋應有觀察句");

        assert!(customs.contains("貨單"));
        assert!(customs.contains("封條"));
        assert!(exchange.contains("銅秤"));
        assert!(exchange.contains("帳冊"));
        assert!(!customs.contains("包袱"));
        assert!(!exchange.contains("包袱"));
    }

    #[test]
    fn m3_006_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("搬魚人", "港灣貨棚"), "陌生人");
        assert_eq!(observed_label_for_room("補網人", "漁網曬場"), "陌生人");
        assert_eq!(observed_label_for_room("王小明", "烏石鼻守望石階"), "陌生人");
    }
    #[test]
    fn north_harbor_look_uses_harbor_work_not_generic_passerby() {
        let net = scene_observation_line("漁網曬場", "她").expect("漁網曬場應有觀察句");
        let watch = scene_observation_line("烏石鼻守望石階", "他").expect("烏石鼻守望石階應有觀察句");

        assert!(net.contains("濕網"));
        assert!(net.contains("穿線"));
        assert!(watch.contains("海面"));
        assert!(watch.contains("警鈴"));
        assert!(!net.contains("包袱"));
        assert!(!watch.contains("包袱"));
    }

    #[test]
    fn m3_009_public_roles_are_kept_when_looking() {
        assert_eq!(observed_label_for_room("礦工", "礦坑外棚"), "陌生人");
        assert_eq!(observed_label_for_room("洗礦工", "洗礦溝"), "陌生人");
        assert_eq!(observed_label_for_room("王小明", "工具修棚"), "陌生人");
        assert_eq!(observed_label_for_room("李小明", "礦渣坡"), "陌生人");
    }
    #[test]
    fn northwest_mine_look_uses_mine_work_not_generic_passerby() {
        let wash = scene_observation_line("洗礦溝", "她").expect("洗礦溝應有觀察句");
        let load = scene_observation_line("紅土崁選礦車場", "他").expect("紅土崁選礦車場應有觀察句");

        assert!(wash.contains("冷水槽"));
        assert!(wash.contains("粗料倉"));
        assert!(load.contains("礦車"));
        assert!(load.contains("稱重木牌"));
        assert!(!wash.contains("包袱"));
        assert!(!load.contains("包袱"));
    }

    #[test]
    fn final_map_public_roles_are_kept_when_looking() {
        for (raw, room, _label) in [
            ("守門人", "邊城外門", "守門人"),
            ("王小明", "糧倉前場", "倉管"),
            ("鐵匠", "鍛造棚", "鐵匠"),
            ("李小明", "牧地柵門", "牧人"),
            ("獸醫", "獸醫棚", "獸醫"),
            ("趙小明", "商隊泊位", "商隊人"),
            ("店伙", "客棧前廊", "店伙"),
            ("巡哨", "巡哨棚", "巡哨"),
        ] {
            assert_eq!(observed_label_for_room(raw, room), "陌生人", "{room}");
        }
    }
    #[test]
    fn final_map_look_uses_scene_work_not_generic_passerby() {
        let border = scene_observation_line("邊城外門", "他").expect("邊城外門應有觀察句");
        let forge = scene_observation_line("鍛造棚", "她").expect("鍛造棚應有觀察句");
        let pasture = scene_observation_line("牧地柵門", "他").expect("牧地柵門應有觀察句");
        let town = scene_observation_line("邊鎮井口", "她").expect("邊鎮井口應有觀察句");

        assert!(border.contains("通行牌"));
        assert!(forge.contains("鐵胚"));
        assert!(pasture.contains("畜牌"));
        assert!(town.contains("井繩"));
        assert!(!border.contains("包袱"));
        assert!(!forge.contains("包袱"));
        assert!(!pasture.contains("包袱"));
        assert!(!town.contains("包袱"));
    }

    #[test]
    fn no_double_outfit_conflict() {
        let mut ch = test_character();
        ch.id = "wt_npc_chen_wanfu".to_string();
        ch.display_title = "三水食堂當家大廚|陳萬福".to_string();
        ch.equipment_slots = r#"{"body":"starter_body_m","legs":"starter_legs_m","feet":"starter_feet_m"}"#.to_string();
        let line = build_look_narrative(&ch, "陳萬福", "文曜・三水口・三水食堂", None);
        assert!(line.contains("圍裙") || line.contains("大廚") || line.contains("主廚"));
        assert!(!line.contains("身上穿戴著"));
        assert!(!line.contains("粗布短褂"));
    }

    #[test]
    fn look_shows_equipped_tools_even_with_custom_outfit() {
        let mut ch = test_character();
        ch.id = "wt_npc_chen_wanfu".to_string();
        ch.display_title = "三水食堂當家大廚|陳萬福".to_string();
        ch.equipment_slots = r#"{"body":"starter_body_m","hold_r":"craft_steel_hunting_knife"}"#.to_string();
        let line = build_look_narrative(&ch, "陳萬福", "文曜・三水口・三水食堂", None);
        assert!(line.contains("手裡握著【防身獵刀"));
    }

    #[test]
    fn test_look_favorability_rel_phrase() {
        let ch = test_character();
        let line = build_look_narrative(&ch, "某甲", "客棧前廊", Some("player_test_viewer"));
        // Without store/pg memory, favorability defaults to 0 -> no special rel phrase
        assert!(!line.contains("看向你時神色帶著明顯戒備與舊怨"));
    }

    #[test]
    fn look_narrative_includes_breath_habit_vessel_and_affliction_cleanly() {
        let mut ch = test_character();
        ch.id = "npc_look_test_living".to_string();
        ch.display_title = "西市打鐵鋪匠師|鐵掌老張".to_string();
        ch.soul_seed = Some(888);
        ch.equipment_slots = r#"{"hold_r":"craft_steel_hunting_knife"}"#.to_string();

        // 植入創傷暗疾
        crate::pg::living_world::record_entity_affliction(
            &ch.id,
            crate::pg::living_world::AfflictionKind::ArmSprain,
            "forge_work",
            crate::game::now_unix(),
        );

        let narrative = build_look_narrative(&ch, "鐵掌老張", "文曜・白塔西市", None);

        // 1. 內息底色不應出現在外部打量中
        assert!(!narrative.contains("內息底色"));
        assert!(!narrative.contains("思忖著"));

        // 2. 外部觀察器物（非自身觀測時，Sharp 狀態保持乾淨器名，無內部熟練度標籤）
        assert!(narrative.contains("手裡握著【防身獵刀】"));
        assert!(!narrative.contains("如臂使指"));
        assert!(!narrative.contains("漸趨順手"));
        assert!(!narrative.contains("沉穩趁手"));

        // 3. 創傷觀測
        assert!(narrative.contains("似帶有腕肘挫傷之態。"), "narrative was: {narrative}");
        // 4. 嚴格檢查無內部 ID 洩漏與違禁詞
        assert!(!narrative.contains("npc_"));
        assert!(!narrative.contains("wt_"));
        assert!(!narrative.contains("mat_"));
        assert!(!narrative.contains("craft_"));
        assert!(!narrative.contains("database"));
        assert!(!narrative.contains("資料庫"));
        assert!(!narrative.contains("還有"));
        assert!(!narrative.contains(concat!("\u{525b}", "\u{624d}")));
        assert!(!narrative.contains(concat!("\u{525b}", "\u{525b}")));
        assert!(!narrative.contains(concat!("\u{65b9}", "\u{624d}")));
        assert!(!narrative.contains("頃刻"));
        assert!(!narrative.contains("110V"));
        assert!(!narrative.contains("念場"));
        assert!(!narrative.contains("聖塔"));
        assert!(!narrative.contains("小市"));
        assert!(!narrative.contains("短市"));
        assert!(!narrative.contains("青籠關"));

        // 清理創傷
        crate::pg::living_world::clear_entity_afflictions(&ch.id, true, crate::game::now_unix());
        crate::store::clear_active_npc_obligation(&ch.id, "treat_injury");
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut conn) = pool.get()
        {
            let _ = conn.execute(
                "DELETE FROM resident_obligations WHERE entity_id = 'npc_look_test_living'",
                &[],
            );
        }
    }

    #[test]
    fn test_look_qin_shuisheng_real_time_inscriptions_and_state() {
        let now = 1_800_000_000i64;
        crate::pg::living_world::seed_flagship_specimen_qin_shuisheng_once(now);
        let mut qin = test_character();
        qin.id = "秦水生".to_string();
        qin.display_title = "老堰頭水輪石磨首座|秦水生".to_string();
        qin.soul_seed = Some(4_088_606_620_229_995_941);
        qin.equipment_slots = r#"{"head":"wt_cat_0875","body":"wt_cat_0869","legs":"starter_legs_m","feet":"starter_feet_m","wrist":"wt_cat_0852","waist":"wt_cat_0897","hold_r":"wt_cat_0984","hold_l":"wt_cat_0973"}"#.to_string();
        qin.inventory = r#"[{"item_id":"wt_cat_0388","qty":5},{"item_id":"wt_cat_0279","qty":7},{"item_id":"wt-good-009","qty":9}]"#.to_string();
        qin.current_activity = "背負薄層重礦烏木膽溫存桶沿山道前往文曜三水大食堂送交早市米漿".to_string();

        let narrative = build_look_narrative(&qin, "秦水生", "老堰頭・山道", Some("葉卅"));
        assert!(
            narrative.contains("羊角鐵錘打石手鏨 [巧][堅]"),
            "expected inscribed chisel in look narrative: {narrative}"
        );
        assert!(
            narrative.contains("平底石磨分粉篩架 [純][流]"),
            "expected inscribed sieve in look narrative: {narrative}"
        );
        assert!(
            narrative.contains("潮汐綿長息"),
            "expected breath habit in look narrative: {narrative}"
        );
        assert!(
            narrative.contains("老堰頭水輪分動與雙疊石磨聽音辨距法"),
            "expected lineage mastery in look narrative: {narrative}"
        );
        assert!(
            narrative.contains("現磨溫米漿 5 桶") && narrative.contains("特級在來米粉 7 袋") && narrative.contains("待磨精碾白米 9 袋"),
            "expected real-time inventory counts in look narrative: {narrative}"
        );
        assert!(
            narrative.contains("背負薄層重礦烏木膽溫存桶沿山道前往文曜三水大食堂送交早市米漿"),
            "expected specific transit activity preserved in look narrative: {narrative}"
        );
    }

    #[test]
    fn test_look_universal_living_entity_details_on_offline_player_and_npc() {
        let mut player = test_character();
        player.id = "player_offline_test_look".to_string();
        player.kind = crate::entity::EntityKind::Player;
        player.inventory = r#"[{"item_id":"craft_wooden_tool_handle","qty":2},{"item_id":"wt-good-012","qty":3}]"#.to_string();
        player.current_activity = "沿著石板步道漫步穿行前往工坊".to_string();

        let (_rec, _) = crate::pg::living_world::record_mentor_imprint_progress(
            &player.id,
            "溫掌事",
            "百工調息衡準訣",
            "神思澄明",
            crate::game::now_unix(),
        );

        let narrative = build_look_narrative(&player, "某玩家", "工坊前廊", Some("viewer_p"));
        assert!(narrative.contains("身負「百工調息衡準訣」師承造詣。"), "expected lineage: {narrative}");
        assert!(narrative.contains("隨身攜有"), "expected carried items: {narrative}");
        assert!(narrative.contains("平日多留心於「"), "expected in-world offline directive posture: {narrative}");
        assert!(narrative.contains("沿著石板步道漫步穿行前往工坊"), "expected generalized transit preserved: {narrative}");
    }
    #[test]
    fn test_look_and_followup_actions_recognize_uniform_and_plain_clothes_counter_manager() {
        let viewer_id = "player_viewer_test";

        // 1. 穿便服但站在非夥計禁止入內櫃檯區的掌事（有貨物，但無制服、未結識為陌生人）
        let mut manager = test_character();
        manager.id = "npc_counter_manager_plain".to_string();
        manager.display_title = "穿便服但站在非夥計禁止入內櫃檯區的掌事".to_string();
        manager.grid_x = Some(2);
        manager.grid_y = Some(0);
        manager.inventory = r#"[{"item_id":"wt-good-009","qty":5}]"#.to_string();

        let manager_narrative = build_look_narrative(&manager, "陌生人", "沙鹿商鋪", Some(viewer_id));
        assert!(
            manager_narrative.contains("雖著尋常便服，卻能自由出入非外客可入之櫃檯與內場禁區，顯係此處主事之人。"),
            "expected plain-clothes counter manager identity in narrative: {manager_narrative}"
        );
        let manager_actions = followup_actions(viewer_id, &manager);
        assert!(
            manager_actions.contains(&"Trade".to_string()),
            "unacquainted counter manager with stock MUST expose Trade socket in followup_actions: {manager_actions:?}"
        );

        // 2. 同一房間內的路人顧客（無制服、非櫃檯內場、未結識為陌生人）
        let mut passerby = test_character();
        passerby.id = "npc_passerby_customer".to_string();
        passerby.display_title = "店內喝茶閒聊的茶客顧客".to_string();
        passerby.grid_x = Some(2);
        passerby.grid_y = Some(0);
        passerby.inventory = r#"[{"item_id":"wt-good-009","qty":5}]"#.to_string();

        let passerby_narrative = build_look_narrative(&passerby, "陌生人", "沙鹿商鋪", Some(viewer_id));
        assert!(
            !passerby_narrative.contains("櫃檯與內場禁區"),
            "passerby customer narrative must NOT contain counter zone text: {passerby_narrative}"
        );
        let passerby_actions = followup_actions(viewer_id, &passerby);
        assert_eq!(
            passerby_actions,
            vec!["Talk".to_string()],
            "passerby customer followup_actions must only be Talk: {passerby_actions:?}"
        );
    }

    #[test]
    fn test_look_any_resident_shows_inscribed_vocational_vessel_and_event_pack_duty() {
        let now = 1_800_000_000i64;
        crate::pg::living_world::seed_flagship_specimen_qin_shuisheng_once(now);

        let mut resident = test_character();
        resident.id = "秦水生".to_string();
        resident.kind = crate::entity::EntityKind::Npc;
        resident.display_title = "老堰頭水輪石磨首座|秦水生".to_string();
        resident.soul_seed = Some(4_088_606_620_229_995_941);
        resident.equipment_slots = r#"{"head":"wt_cat_0875","body":"wt_cat_0840","legs":"starter_legs_m","feet":"starter_feet_m","waist":"wt_cat_0897"}"#.to_string();
        resident.current_activity = "於水輪石磨工坊料理店務".to_string();

        let room_name = "老堰頭・水輪石磨工坊";
        let narrative = build_look_narrative(&resident, "秦水生", room_name, Some("viewer_p"));

        assert!(
            narrative.contains("平底石磨分粉篩架 [純][流]"),
            "expected inscribed vocational vessel for resident without held tools: {narrative}"
        );
        assert!(
            narrative.contains("手邊常備本職器物【平底石磨分粉篩架 [純][流]】"),
            "expected vocational vessel format in narrative: {narrative}"
        );
        assert!(
            narrative.contains("此刻正當值經手場上事件包【") && narrative.contains("/3 幕・關鍵器物："),
            "expected active EventPack duty title and key objects in narrative: {narrative}"
        );

        let mut staff = test_character();
        staff.id = "npc_staff_floor_worker".to_string();
        staff.kind = crate::entity::EntityKind::Npc;
        staff.display_title = "石磨研磨學徒".to_string();
        staff.equipment_slots = r#"{"body":"wt_cat_0840"}"#.to_string();
        staff.current_activity = "在場中搬運麻袋".to_string();
        let floor_narrative = build_look_narrative(&staff, "學徒", room_name, Some("viewer_p"));
        assert!(
            floor_narrative.contains("身著明顯的當值工作制服，正於場內料理店務與百工差事。"),
            "expected floor uniform line: {floor_narrative}"
        );
        assert!(
            floor_narrative.contains("此刻正當值經手場上事件包【"),
            "floor staff with uniform must also show event pack duty: {floor_narrative}"
        );
        assert!(
            floor_narrative.contains("手邊常備本職器物【百工學徒手用小木尺】"),
            "apprentice should show vocational vessel: {floor_narrative}"
        );
    }
}

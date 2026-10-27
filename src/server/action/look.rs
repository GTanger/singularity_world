//! `do_action` — Look 分支：打量目標，輸出外觀敘述。

use crate::db;
use crate::entity::Character;
use crate::event::{self, types};
use crate::game::{
    final_map_observation_template_for_room, is_final_map_public_role, public_person_label_for_room,
};

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
    let _ = event::append(now, pid, types::OBSERVED, target_id);
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
        .replace("茶肆", "茶屋或食堂")
        .replace("客棧", "旅社")
        .replace("熟絡", "熱絡")
        .replace("床榻", "床鋪")
        .replace("榻上", "床鋪上")
}
fn format_activity_sentence(pronoun: &str, act: &str) -> String {
    let trimmed = act.trim();
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
            let mut desc = format!(
                "你屏息凝神，仔細觀察眼前的山海生靈【{}】。{}\n體魄 {}・靈敏 {}・氣勁 {}。",
                target.display_title, shanhai_note, target.vit, target.dex, target.qi
            );
            if !target.current_activity.trim().is_empty() {
                desc.push_str(&format!(" 牠{}。", target.current_activity.trim()));
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
    let has_canon_desc = canonical_npc_appearance_and_demeanor(target, room_name);
    let mut is_known_canon = false;
    let sensory_line = if target.kind == crate::entity::EntityKind::Npc {
        crate::store::npc_sensory_appearance_line(&target.id, &target.display_title)
    } else {
        String::new()
    };
    let is_female_sensory = sensory_line.contains("裙")
        || sensory_line.contains("簪")
        || sensory_line.contains("髮髻")
        || sensory_line.contains("娘")
        || sensory_line.contains("小姐")
        || sensory_line.contains("婦")
        || sensory_line.contains("溫婉")
        || sensory_line.contains("靈秀")
        || sensory_line.contains("紅絲帶");
    let pronoun = if is_female_sensory || target.gender == Some(crate::entity::Gender::F) {
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

    let personality = crate::db::get_personality_for_entity(&target.id)
        .or_else(|| target.soul_seed.map(crate::db::expand_soul_seed_to_personality));
    if let Some(p) = personality {
        let is_gentle_demeanor = sensory_line.contains("端方")
            || sensory_line.contains("溫婉")
            || sensory_line.contains("輕柔")
            || sensory_line.contains("細緻")
            || sensory_line.contains("平和")
            || sensory_line.contains("斯文")
            || sensory_line.contains("合度");
        let bold_str = if is_gentle_demeanor || p.boldness < 0.5 {
            "神態內斂"
        } else {
            "氣勢沉猛"
        };
        let sens_str = if p.sensitivity >= 0.5 { "目光敏銳" } else { "神色淡然" };
        let order_str = if is_gentle_demeanor || p.orderliness >= 0.5 {
            "衣物整飭"
        } else {
            "行止隨性"
        };
        desc.push_str(&format!("{pronoun}{bold_str}、{sens_str}，{order_str}。"));
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

            // 1. 若無專屬服飾，顯示基本服飾穿戴（避免與特定 NPC 定制外貌描寫衝突）
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
                    desc.push_str("身上穿戴著");
                    desc.push_str(&pieces.join("、"));
                    desc.push('。');
                }
            }

            // 2. 持握工具 / 武器（無論是否有專屬服飾皆顯示器物與銘文字）
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
                        let formatted = crate::server::handler::format_entity_item_with_inscription(
                            &target.id,
                            item_id,
                            base_name,
                        );
                        if crate::pg::living_world::is_weapon_or_tool_item(item_id, base_name) {
                            let mut vstate = crate::pg::living_world::get_vessel_state(&target.id, item_id);
                            if vstate.use_count == 0 && target.kind == crate::entity::EntityKind::Npc {
                                vstate = crate::pg::living_world::record_vessel_use(
                                    &target.id,
                                    item_id,
                                    0,
                                    4 + ((target.soul_seed.unwrap_or(0).unsigned_abs() % 12) as i32),
                                    crate::game::now_unix(),
                                );
                            }
                            held_items.push(format!(
                                "【{formatted}】（{}・{}）",
                                vstate.familiarity().label(),
                                vstate.condition.label()
                            ));
                        } else {
                            held_items.push(format!("【{formatted}】"));
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
                    if !item_id.is_empty() && !is_starter_gear(item_id) {
                        let ecol_name = crate::game::object_ecology::lookup_ecology_item_def(item_id).map(|d| d.name);
                        let base_name = names
                            .get(slot)
                            .map(|s| s.as_str())
                            .or(ecol_name.as_deref())
                            .unwrap_or(item_id);
                        let formatted = crate::server::handler::format_entity_item_with_inscription(
                            &target.id,
                            item_id,
                            base_name,
                        );
                        worn_items.push(format!("【{formatted}】"));
                    }
                }
            }
            if !worn_items.is_empty() {
                desc.push_str(&format!("隨身佩著{}。", worn_items.join("、")));
            }
        }
    }
    if !is_known_canon && target.kind == crate::entity::EntityKind::Npc && !sensory_line.trim().is_empty() {
        desc.push_str(&format!("{pronoun}{sensory_line}。"));
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
            if is_night {
                let is_watch = sanitized_act.contains("值守")
                    || sanitized_act.contains("巡看")
                    || sanitized_act.contains("巡防")
                    || sanitized_act.contains("守夜")
                    || sanitized_act.contains("攤")
                    || sanitized_act.contains("叫賣")
                    || sanitized_act.contains("吆喝")
                    || sanitized_act.contains("招呼")
                    || target.id.starts_with("wt_vendor_")
                    || room_name.contains("哨")
                    || room_name.contains("關口")
                    || room_name.contains("道口")
                    || room_name.contains("水口")
                    || room_name.contains("渡口")
                    || room_name.contains("門坊");
                let is_sleep = sanitized_act.contains("安歇")
                    || sanitized_act.contains("熟睡")
                    || sanitized_act.contains("就寢")
                    || sanitized_act.contains("熄燈")
                    || sanitized_act.contains("歇息");
                if is_watch || is_sleep {
                    desc.push_str(&format_activity_sentence(pronoun, &sanitized_act));
                } else {
                    desc.push_str(&format!("{pronoun}已在屋內床鋪上熄燈就寢安歇。"));
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
                    "\n【合社圍獵編組】目前隸屬領隊【{}】率領之{}合社獵團（共 {} 人），目標獵物為【{}】。",
                    party.leader_name, party.zone_name, party.member_ids.len(), party.prey_name
                ));
            }
        }
        let now_ts = crate::game::now_unix();
        // 優先尋找非通用、非「今日花用又見底」的具體心緒
        let mut chosen_flavor = None;
        for t in target.thoughts.iter().rev() {
            let not_expired = t.decay_minutes <= 0 || (now_ts - t.created_at) < t.decay_minutes * 60;
            if not_expired {
                if let Some(f) = &t.flavor {
                    let s = f.trim();
                    if !s.is_empty()
                        && !s.contains("花用又見底")
                        && !s.contains("又見底，發愁")
                        && !s.is_ascii()
                    {
                        chosen_flavor = Some(s.to_string());
                        break;
                    }
                }
            }
        }
        // 若無特定具體心緒，則根據 NPC 當前待辦義務或當前活動合成真實具體的心緒
        if chosen_flavor.is_none() {
            if let Some((kind, subj)) = crate::store::get_active_npc_obligation(&target.id) {
                let synth = match kind.as_str() {
                    "restock_goods" => "店裡現貨吃緊，得趁早補齊庫存".to_string(),
                    "repair_tool" => "手頭工具耗損得緊，得抽空修整一番".to_string(),
                    "stranded_cargo" => "卡在外頭的那批貨件，得想方設法接應回來".to_string(),
                    "repay_debt" => "近來的幾筆人情帳目，得早些算個分明".to_string(),
                    "treat_injury" => "這身傷損得妥善料理，切莫傷了筋骨根本".to_string(),
                    _ => {
                        let clean_subj = if subj.is_ascii() {
                            crate::world::materials::material_by_item_id(&subj)
                                .map(|m| m.name.clone())
                                .or_else(|| db::get_item_def(&subj).map(|d| d.name))
                                .unwrap_or_else(|| "手頭差事".to_string())
                        } else {
                            subj.clone()
                        };
                        format!("得把【{clean_subj}】這檔差事處置妥貼")
                    }
                };
                chosen_flavor = Some(synth);
            } else {
                let act = &target.current_activity;
                let synth = if act.contains("鍛") || act.contains("鐵") || act.contains("爐") {
                    "爐火正旺，這批鐵胚得趁熱打出成色來".to_string()
                } else if act.contains("食") || act.contains("麵") || act.contains("煮") || act.contains("煎") || act.contains("烤") || act.contains("廚") {
                    "湯頭與火候得分毫不差，讓過路食客吃口暖胃順心的".to_string()
                } else if act.contains("帳") || act.contains("盤點") || act.contains("貨") || act.contains("算") || act.contains("商") || act.contains("賣") {
                    "市面行情起落無常，每筆進出都得算得明明白白".to_string()
                } else if act.contains("書") || act.contains("讀") || act.contains("卷") || act.contains("學") || act.contains("閣") {
                    "卷冊所錄的脈絡章法，尚有幾處關竅待細細推敲".to_string()
                } else if act.contains("藥") || act.contains("草") || act.contains("診") || act.contains("醫") {
                    "藥石性味不同，劑量配比更得嚴絲合縫，輕忽不得".to_string()
                } else if act.contains("巡") || act.contains("守") || act.contains("衛") || act.contains("哨") {
                    "四下動靜不可大意，各處關口都得盯得踏實些".to_string()
                } else if act.contains("走") || act.contains("趕") || act.contains("行") || act.contains("路") {
                    "腳下得快些走動，免得耽擱了前頭約定的時辰".to_string()
                } else if act.contains("歇") || act.contains("茶") || act.contains("坐") || act.contains("飲") {
                    "忙裡偷得片刻清閒，定定心神也是保養".to_string()
                } else if act.contains("耕") || act.contains("農") || act.contains("田") || act.contains("苗") {
                    "農時節令緊要，地裡的苗情得細心料理照拂".to_string()
                } else if target.id.starts_with("wt_vendor_") || target.display_title.contains("攤") {
                    "手頭這口營生不可怠慢，得把每樣貨色都拾掇得當".to_string()
                } else if !target.display_title.is_empty() {
                    "把手頭這份營生踏實顧好，求個平順安穩".to_string()
                } else {
                    "看顧好眼前手頭這點活計，步步踏實便好".to_string()
                };
                chosen_flavor = Some(synth);
            }
        }

        let clean_flavor = chosen_flavor.and_then(|mut s| {
            if s.is_ascii() {
                return None;
            }
            for forbidden in [
                "小市", "短市", "剛才", "剛剛", "還有", "資料庫", "database", "grid", "debug", "scene_stub", "聖塔", "110V", "110v", "念場", "富態",
            ] {
                s = s.replace(forbidden, "");
            }
            let s = s.trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        });

        let mood_phrase = match target.disposition {
            ..=-30 => Some("眉宇微蹙，神色間透著幾分鬱結與焦躁"),
            -29..=-10 => Some("神色略顯疲態，似有心事縈繞"),
            15..=39 => Some("神色舒展，氣息平和安穩"),
            40.. => Some("眉目舒朗、神采奕奕，心境頗為暢快"),
            -9..=14 => None,
        };

        match (mood_phrase, clean_flavor) {
            (Some(mood), Some(flavor)) => {
                desc.push_str(&format!("{pronoun}此刻{mood}，眉眼間似正思忖著「{flavor}」。"));
            }
            (None, Some(flavor)) => {
                desc.push_str(&format!("{pronoun}眉眼間似正思忖著「{flavor}」。"));
            }
            (Some(mood), None) => {
                desc.push_str(&format!("{pronoun}此刻{mood}。"));
            }
            (None, None) => {}
        }
    }

    // 觀測目標內息底色與吐納氣機
    let vocation_for_habit = if target.display_title == "山海行旅者" {
        ""
    } else {
        target.display_title.as_str()
    };
    let breath_rec = crate::pg::living_world::get_or_derive_breath_habit(
        &target.id,
        vocation_for_habit,
        target.soul_seed.unwrap_or(0),
    );
    desc.push_str(&format!(
        "{pronoun}{}（內息底色：【{}】）。",
        breath_rec.dominant.combat_rhythm_phrase(),
        breath_rec.dominant.label()
    ));

    // 觀測目標肉身創傷或暗疾
    let afflictions = crate::pg::living_world::get_entity_afflictions(&target.id);
    if !afflictions.is_empty() {
        let labels: Vec<&str> = afflictions.iter().map(|a| a.label()).collect();
        desc.push_str(&format!("細看之下，{pronoun}似帶有{}之態。", labels.join("、")));
    }

    let mut ob_matter_opt = None;
    if let Some((kind, subj)) = crate::store::get_active_npc_obligation(&target.id) {
        ob_matter_opt = match kind.as_str() {
            "restock_goods" => Some("正為店內現貨見底、急需提貨補倉而掛心".to_string()),
            "repair_tool" => Some("手頭器物似有耗損待修".to_string()),
            "stranded_cargo" => Some("正為一批滯留貨物發愁".to_string()),
            "repay_debt" => Some("眉宇間似有帳務掛心".to_string()),
            "treat_injury" => Some("氣息略顯不穩，似有傷在身".to_string()),
            _ => if !subj.is_empty() {
                let clean_subj = if subj.is_ascii() {
                    crate::world::materials::material_by_item_id(&subj)
                        .map(|m| m.name.clone())
                        .or_else(|| db::get_item_def(&subj).map(|d| d.name))
                        .unwrap_or_else(|| "手頭差事".to_string())
                } else {
                    subj.clone()
                };
                Some(format!("正為【{clean_subj}】掛心"))
            } else {
                None
            },
        };
    }
    if let Some(pool) = crate::pg::pool() {
        if ob_matter_opt.is_none() {
            if let Ok(obligations) =
                crate::pg::living_world::list_open_obligations_for_entity(&pool, &target.id)
            {
                for ob in &obligations {
                    let matter = match ob.obligation_kind.as_str() {
                        "restock_goods" => Some("正為店內現貨見底、急需提貨補倉而掛心".to_string()),
                        "repair_tool" => Some("手頭器物似有耗損待修".to_string()),
                        "stranded_cargo" => Some("正為一批滯留貨物發愁".to_string()),
                        "repay_debt" => Some("眉宇間似有帳務掛心".to_string()),
                        "treat_injury" => Some("氣息略顯不穩，似有傷在身".to_string()),
                        _ => None,
                    };
                    if let Some(matter) = matter {
                        ob_matter_opt = Some(matter);
                        break;
                    }
                }
            }
        }
        if let Some(matter) = ob_matter_opt.take() {
            desc.push_str(&format!("{pronoun}{matter}。"));
        }

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
            } else if eff_balance >= 3 {
                Some("看向你時神色熟稔溫和")
            } else if eff_fam >= 2 {
                Some("對你已有幾分印象")
            } else {
                None
            };
            if let Some(phrase) = rel_phrase {
                desc.push_str(&format!("{pronoun}{phrase}。"));
            }
        }
    } else {
        if let Some(matter) = ob_matter_opt {
            desc.push_str(&format!("{pronoun}{matter}。"));
        }
        if let Some(pid) = viewer_pid
            && !pid.is_empty()
        {
            let fav = db::get_favorability(&target.id, pid);
            let eff_balance = fav / 10;
            let eff_fam = if fav > 0 { 1 } else { 0 };
            let rel_phrase = if eff_balance <= -3 {
                Some("看向你時神色帶著明顯戒備與舊怨")
            } else if eff_balance >= 3 {
                Some("看向你時神色熟稔溫和")
            } else if eff_fam >= 2 {
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

fn player_visible_target_name(player_id: &str, target: &Character, _room_name: &str) -> String {
    super::player_visible_target_name(player_id, target)
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
    if super::is_canonical_named_npc(target) || named {
        vec![
            "Talk".into(),
            "Trade".into(),
            "Borrow".into(),
            "Subdue".into(),
            "Slay".into(),
        ]
    } else if super::trade::target_has_shop_offer(target, &room_name)
        || target_is_available_market_operator(player_id, &target.id)
    {
        vec!["Talk".into(), "Trade".into()]
    } else {
        vec!["Talk".into()]
    }
}

fn canonical_npc_appearance_and_demeanor(target: &Character, room_name: &str) -> Option<String> {
    let t_id = target.id.as_str();
    let title = target.display_title.as_str();
    let title_short = title.rsplit(['|', '｜']).next().unwrap_or(title).trim();
    let has = |key: &str| -> bool {
        t_id == key
            || t_id.contains(key)
            || title.contains(key)
            || title_short == key
            || title_short.contains(key)
    };

    // 1. 陸秉衡（文曜商會總會長）
    if has("陸秉衡") || t_id == "wt_npc_lu_bingheng" {
        return Some("老會長年事已高卻精神矍鑠，穿著整潔挺括的藏青色正裝外套，搭配玄武岩雙扣，神色和藹沉穩，眉宇間透著數十年商界歷練的從容大氣，手裡握著一枚精巧的黃銅印章。".into());
    }
    // 2. 陸知微（藏書閣主編）
    if has("陸知微") || t_id == "wt_npc_lu_zhiwei" {
        return Some("清麗素雅的年輕主編，身穿竹青色立領長版襯衫，長髮以素木髮夾挽起，神情專注沉靜，眉眼溫婉而明慧，袖口微挽，指尖常帶著校對書稿與排印圖冊留下的淡淡油墨香。".into());
    }
    // 3. 岑守岳（第一鍛造工坊大匠師）
    if has("岑守岳") || t_id == "wt_npc_cen_shouyue" {
        return Some("滿面絡腮鬍、粗獷豪邁的大匠師，露出結實粗壯的手臂，繫著一條厚牛皮防燙圍裙，眼神如爐火般熾烈堅定，掌心滿是厚繭，渾身散發著炭火與金屬的剛健氣息。".into());
    }
    // 4. 岑拓（第一鍛造工坊青年匠師）
    if has("岑拓") || t_id == "wt_npc_cen_tuo" {
        return Some("年輕幹練的鍛造技師，穿著耐磨工作短衫，體態勻稱結實，神情專注認真，眉宇間帶著鑽研新器物的朝氣與執著，指節滿是長年敲打金屬的厚繭，腰間別著一把黃銅游標測尺。".into());
    }
    // 5. 溫懷瑾（文曜學院教務長）
    if has("溫懷瑾") || t_id == "wt_npc_wen_huaijin" {
        return Some("面容清瘦斯文的長者，鬢髮微霜，穿著水墨月白色剪裁外套，步履沉穩優雅，目光溫和睿智，隨身帶著一組精密銀針與教學筆記，展現出主持學院教務的學者氣度。".into());
    }
    // 6. 溫晴（溫氏脈絡診所主治醫師）
    if has("溫晴") || t_id == "wt_npc_wen_qing" {
        return Some("身形纖細卻步履輕快的年輕醫師，穿著俐落的杏黃色工作長裙，腰間配掛分格醫療包與百草溫灸盒，神情溫柔專注，身上常年帶著淡淡的山風草與清涼薄荷香氣。".into());
    }
    // 7. 韓鐵山（西門車馬聯運總站長）
    if has("韓鐵山") || t_id == "wt_npc_han_tieshan" {
        return Some("膚色黝黑、嗓音洪亮的聯運總站長，身材魁梧結實，身穿厚實毛氈翻領背心，臂膀結實有力，眼神沉穩敏銳，手腕戴著防磨牛皮護腕，帶著長年奔走半島跨城幹線的幹練氣息。".into());
    }
    // 8. 陳萬福（三水食堂當家大廚）
    if has("陳萬福") || t_id == "wt_npc_chen_wanfu" {
        return Some("體態圓潤的三水食堂主廚，臉上總是掛著熱情的笑容，繫著寬大的白色廚師圍裙，捲起袖子，手持大湯勺，周身帶著原牛高湯與蔥香熱油的濃郁香氣。".into());
    }
    // 9. 沈清河（東橋旅社與茶屋東家）
    if has("沈清河") || t_id == "wt_npc_shen_qinghe" {
        return Some("眉目清俊、氣質溫和的旅社負責人，穿著雪青色休閒外套，神態安詳從容，說話不疾不徐，泡茶手法俐落熟練，櫃檯旁擺著一盞旋蓋調光夜光貝母檯燈，讓人感到格外放鬆。".into());
    }
    // 10. 白泉生（泉香溫水浴場場主）
    if has("白泉生") || t_id == "wt_npc_bai_quansheng" {
        return Some("氣色紅潤、神情悠閒的浴場場主，穿著寬鬆透氣的灰白亞麻休閒衫，腳穿防滑木底涼拖，留著整齊的小鬍子，腰間掛著號碼牌，周身帶著淡淡的松脂香皂與溫水蒸氣清香。".into());
    }
    // 11. 柯成器（木作傢俱工坊大木匠）
    if has("柯成器") || t_id == "wt_npc_ke_chengqi" {
        return Some("身型精瘦的老木工師傅，穿著沾有木屑的深藍工作服，臉上帶著親切的笑紋，眼神專注有神，隨身工具腰帶上插著各式精巧的直角尺與導能木刨。".into());
    }
    // 12. 蘇錦心（織造服飾坊當家）
    if has("蘇錦心") || t_id == "wt_npc_su_jinxin" {
        return Some("儀態大方溫婉的服飾坊負責人，身穿親手剪裁的煙青色棉麻套裝長裙，腰間配著軟皮布尺，長髮整齊挽起，目光敏銳，舉手投足展現出專業裁縫設計的俐落感。".into());
    }
    // 13. 沈硯舟（講堂街書局負責人）
    if has("沈硯舟") || t_id == "wt_npc_shen_yanzhou" {
        return Some("斯文清俊的書局負責人，穿著洗得乾淨挺括的淺墨灰襯衫外套，神態溫和，手指留有長年翻閱書頁與活字印刷油墨的細微痕跡，手裡拿著一冊校對完成的《白塔風物誌》。".into());
    }
    // 14. 陸承禮（公議堂地政登記員）
    if has("陸承禮") || t_id == "wt_npc_lu_chengli" {
        return Some("端正嚴謹的青年地政登記員，身穿公議堂深藍色公務外套，腰板挺直，行事一絲不苟，袖口整齊扣好，桌邊整齊擺放著紅印屋契與弧形吸墨器，一舉一動皆依循契約規範。".into());
    }
    // 15. 陸長貴（公議堂民事調解員）
    if has("陸長貴") || t_id == "wt_npc_lu_changgui" {
        return Some("辦事幹練圓融的公議堂民事調解員，年約五十，穿著體面的深赭色排扣外套，留著一撮山羊鬍，笑容可掬，口袋裡常備著幾份街區公告與隨身記事本，是城裡公認的活地圖與調解好手。".into());
    }
    // 16. 陸敬軒（國小老師）
    if has("陸敬軒") || t_id == "wt_npc_lu_jingxuan" {
        return Some("面相斯文溫和的國小老師，穿著深青色剪裁外套，短髮梳理整齊，身形清瘦挺拔，語調溫厚平和，手裡拿著一本《白塔基礎讀本》，看著教室裡的學童時目光充滿期許。".into());
    }
    // 17. 秦明誠（百工核算師）
    if has("秦明誠") || t_id == "wt_npc_qin_mingcheng" {
        return Some("精明幹練的百工核算師，穿著剪裁合身的深灰棉布外套，面容清瘦，眼神銳利，腰間掛著硬革資料袋，隨身攜帶一具精準計數板，隨時都在核算各工坊的成本與產出。".into());
    }
    // 18. 林時中（傳訊鐘樓報時主任）
    if has("林時中") || t_id == "wt_npc_lin_shizhong" {
        return Some("年過半百的鐘樓報時主任，穿著深褐色耐磨工作衫，身形挺拔，斑白短髮梳理得一絲不苟，神情嚴謹，口袋裡放著一枚校時極準的黃銅發條懷錶，行事極為守時。".into());
    }
    // 19. 趙觀星（學院觀星台觀測主任）
    if has("趙觀星") || t_id == "wt_npc_zhao_guanxing" {
        return Some("身形修長的觀星台觀測主任，身穿星青色觀測大衣，神色沉靜專注，雙眼明亮有神，手裡拿著一具摺疊式黃銅星盤量測儀，專注於記錄天象與氣象變化。".into());
    }
    // 20. 周一刀（講堂街理髮老師傅）
    if has("周一刀") || t_id == "wt_npc_zhou_yidao" {
        return Some("年逾六旬的理髮鋪老師傅，穿著一塵不染的白色工作衫，精瘦結實，笑容爽朗，手腕極穩，工具袋裡插著磨刀布與牛角細齒梳，給人一種親切俐落的街坊熟悉感。".into());
    }
    // 21. 岑雲野（分家巷武術館教練）
    if has("岑雲野") || t_id == "wt_npc_cen_yunye" {
        return Some("體格健碩沉著的武術館教練，身穿黑色訓練服與厚牛皮護腰，濃眉有神，動作俐落流暢，雙腕戴著防扭傷護腕，神態沉穩自持。".into());
    }
    // 22. 許阿甘（分家巷雜貨柑仔店阿婆）
    if has("許阿甘") || t_id == "wt_npc_xu_agan" {
        return Some("慈眉善目的白髮阿婆，穿著寬鬆舒適的碎花襯衫，坐在藤椅上微笑，眼神溫和，手裡拿著蒲扇，身邊散發著陳皮、麥芽糖與曬乾艾草的熟悉香氣。".into());
    }
    // 23. 馬補靴（西門修傘補靴老匠）
    if has("馬補靴") || t_id == "wt_npc_ma_buxue" {
        return Some("寡言沉穩的修配老師傅，圍著浸過蜂蠟與桐油的厚帆布工作圍裙，雙手厚實有力，腿上墊著修鞋木楦頭，專注地修補每一雙皮靴與晴雨摺疊傘。".into());
    }
    // 其他聚落正典人物
    if t_id.contains("許萬川") || title.contains("許萬川") {
        return Some("面容堅毅、鬢角斑白的海事商盟會長，穿著厚實防風毛呢大衣，目光沉穩如礁石，歷經數十年海上風浪依然從容不迫。".into());
    }
    if t_id.contains("許海音") || title.contains("許海音") {
        return Some("英氣十足的遠洋大船女船長，身穿合身防水航海夾克，長髮高束，隨身帶著海圖筒，眼神銳利自信。".into());
    }
    if t_id.contains("林正雄") || title.contains("林正雄") {
        return Some("體格魁梧的總造船師，穿著耐磨牛皮工作服，手持木工曲尺，身上帶著百年雲杉與防水桐油的氣息。".into());
    }
    if t_id.contains("周萬生") || title.contains("周萬生") {
        return Some("面容溫和、膚色古銅的農牧總社理事長，捲起麻布襯衫袖口，雙手布滿厚繭，神情踏實從容。".into());
    }
    if t_id.contains("方石") || title.contains("方石") {
        return Some("體格結實的礦務主任，穿著耐火皮革背心，眼神專注銳利，指間常帶著赤鐵礦粉的痕跡。".into());
    }
    if t_id.contains("岑守淵") || title.contains("岑守淵") {
        return Some("身穿防寒長版大衣的實戰名宿，頭髮銀白，目光清冷深邃，站在石台邊神情沉穩如山。".into());
    }
    if t_id.contains("羅烈") || title.contains("羅烈") {
        return Some("身材修長矯健的防務總隊長，身穿深綠色防刮獵裝，眼神如鷹隼般敏銳，時刻留意著林間與水澤的動靜。".into());
    }
    if t_id.contains("顧海平") || title.contains("顧海平") {
        return Some("經驗老到的領航公會首座，穿著防潮航海風衣，手裡拿著夜光貝母領航羅盤，對港灣暗礁與水勢瞭若指掌。".into());
    }
    if t_id.contains("洪守明") || title.contains("洪守明") {
        return Some("守望燈塔的資深匠師，穿著防風厚帆布夾克，目光望向海面，身上帶著冷燻松木屑與海風的乾爽氣息。".into());
    }
    if t_id.contains("鄭烏石") || title.contains("鄭烏石") {
        return Some("精壯黝黑的鑿石匠師，穿著耐磨工作衫，雙手結實有力，神情專注沉穩。".into());
    }
    if t_id.contains("秦水生") || title.contains("秦水生") {
        return Some("樸實沉穩的水輪石磨匠師，穿著淺灰工作衫，袖口沾著些許米粉，專注傾聽著水輪運轉的節奏。".into());
    }
    if t_id.contains("馬山行") || title.contains("馬山行") {
        return Some("健談熱心的山徑補給行負責人，穿著防風厚背心，腰間工具帶插著修鞋工具，笑著招呼往來山路的採集員與旅人。".into());
    }
    if t_id.contains("梁千木") || title.contains("梁千木") {
        return Some("水車鋸木匠師，身材挺拔精悍，工作服上沾著細碎木屑，檢視木材紋理時眼神極為專注。".into());
    }
    if t_id.contains("石萬鈞") || title.contains("石萬鈞") {
        return Some("寒水玉切石匠師，神情沉靜專注，繫著防濺工作圍裙，仔細檢查切石水槽中的切面平整度。".into());
    }
    if t_id.contains("袁山茶") || title.contains("袁山茶") {
        return Some("爽朗幹練的山泉旅社負責人，繫著暖色棉麻圍裙，氣色紅潤，隨時為旅客準備熱騰騰的雪嶺暖身湯。".into());
    }
    if t_id.contains("高望雲") || title.contains("高望雲") {
        return Some("清瘦專注的氣象觀測師，身穿防風長大衣，手持氣象觀測紀錄本，隨時留意山間雲層與風壓變化。".into());
    }
    if t_id.contains("田百稼") || title.contains("田百稼") {
        return Some("笑容滿面的農具工坊大匠，穿著深藍耐磨工作服，手拿鐵扳手，對翻土犁與水閘齒輪的每處細節瞭若指掌。".into());
    }
    if t_id.contains("牧長風") || title.contains("牧長風") {
        return Some("牛埔的乳酪與毛氈匠師，穿著厚毛氈背心，神態溫和開朗，雙手熟練地處理羊毛與乳製品。".into());
    }
    if t_id.contains("方赤土") || title.contains("方赤土") {
        return Some("耐火磚窯資深匠師，膚色帶著窯火烘烤的紅潤，戴著厚牛皮隔熱袖套，對高溫燒製的耐火磚品質把關嚴格。".into());
    }
    if t_id.contains("雷千鈞") || title.contains("雷千鈞") {
        return Some("重型獵具大匠，身材魁梧結實，穿著防刮厚皮護具，雙手沉穩有力，組裝大型獵網與複合獵具十分俐落。".into());
    }
    if t_id.contains("葉青囊") || title.contains("葉青囊") {
        return Some("神情專注的南哨解毒診所醫師，穿著淺青色防護工作服，隨身帶著分類清楚的解毒藥包與急救器材。".into());
    }
    if t_id.contains("阮水生") || title.contains("阮水生") {
        return Some("水尾寮的高腳屋與竹筏匠師，身形精瘦靈活，腰間繫著防滑繩索，在淺澤木棧道上行走自如。".into());
    }

    let _ = room_name;
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

fn observed_label_for_room(raw_name: &str, room_name: &str) -> String {
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
    } else if room_name.contains("總商行") || room_name.contains("票號") {
        Some(format!(
            "{pronoun}正立於樟木高櫃後，一手核對計數板，一手翻閱跨鎮匯兌帳冊，指尖沾著淡藍墨跡。"
        ))
    } else if room_name.contains("契書") {
        Some(format!(
            "{pronoun}手中捏著一枚硃砂小印，細心撫平案几上攤開的紅印屋契與各鎮戶籍抄本。"
        ))
    } else if room_name.contains("公議堂") {
        Some(format!(
            "{pronoun}坐在條形長案旁，專注核對各坊民務登記底本，偶爾抬頭向進廳詢事的街坊點頭示意。"
        ))
    } else if room_name.contains("鐘樓") {
        Some(format!(
            "{pronoun}站在瞭望台銅鐘旁，雙眼注視著日晷刻度與報時鐘擺，手邊放著整裝待發的防雨郵袋。"
        ))
    } else if room_name.contains("大食堂") {
        Some(format!(
            "{pronoun}繫著厚白圍裙在紅鐵厚底大釜旁俐落忙碌，長柄鐵勺盛起熱氣蒸騰的原牛濃湯，香氣四溢。"
        ))
    } else if room_name.contains("浴場") || room_name.contains("浴堂") {
        Some(format!(
            "{pronoun}抱著一疊新洗滌烘乾的蜂格棉巾，俐落地將北山茶籽香皂添入石池旁的木格置物槽。"
        ))
    } else if room_name.contains("藏書閣") {
        Some(format!(
            "{pronoun}踩著木梯輕輕抽出一卷竹綿紙古卷，拂去書脊微塵，神情沈靜地核對目錄標籤。"
        ))
    } else if room_name.contains("觀星台") {
        Some(format!(
            "{pronoun}俯身於青石度盤前，緩步微調三百六十星軌銅儀之子午環，目光專注凝視度數分劃。"
        ))
    } else if room_name.contains("書局") {
        Some(format!(
            "{pronoun}在排架間整理新印的《白塔風物誌》與旬報，手掌將折邊紙頁輕輕壓平。"
        ))
    } else if room_name.contains("商算") {
        Some(format!(
            "{pronoun}就著案前算籌與精準核算板快速敲打，口中低聲念誦進退除法口訣。"
        ))
    } else if room_name.contains("學院") || room_name.contains("講堂") {
        Some(format!(
            "{pronoun}端坐於綠蔭講堂木案前，輕聲吟誦著先賢格物修身之篇，手中握著一支黃銅沾水鋼筆。"
        ))
    } else if room_name.contains("第一鍛造") || room_name.contains("鍛造工坊") {
        Some(format!(
            "{pronoun}手握重鐵火鉗將通紅鐵胚移向鐵砧，鐵錘起落如雷，灼亮火星在燻黑牛皮裙前迸濺。"
        ))
    } else if room_name.contains("木作") || room_name.contains("傢俱") || room_name.contains("巧木") {
        Some(format!(
            "{pronoun}推動手中導能木刨，順著北山雲杉木紋平整推過，捲起層層散發松香氣味的薄木花。"
        ))
    } else if room_name.contains("織造") || room_name.contains("服飾") || room_name.contains("雲錦") {
        Some(format!(
            "{pronoun}坐在織機旁引梭穿線，腳踩踏板節奏均勻，細密平整的大圳棉麻匹料徐徐延展。"
        ))
    } else if room_name.contains("診所") {
        Some(format!(
            "{pronoun}在百草藥匣前拉開抽屜，取出一包乾製山風草置於研缽，手中石杵均勻研磨。"
        ))
    } else if room_name.contains("東橋旅社") || room_name.contains("茶屋") {
        Some(format!(
            "{pronoun}提起紫銅長嘴水壺，滾水緩緩注入瓷壺，清冽烘青茶香隨白霧在大堂裊裊散開。"
        ))
    } else if room_name.contains("剪剃鋪") {
        Some(format!(
            "{pronoun}手腕輕抖在蕩刀布上往復磨拭精鋼剃刀，銅盆溫水騰起陣陣溫潤白氣。"
        ))
    } else if room_name.contains("習武堂") {
        Some(format!(
            "{pronoun}手持包革練習重木刀在沙地上沉腰坐馬，步法穩健，反覆練習轉腕卸力防守。"
        ))
    } else if room_name.contains("柑仔店") || room_name.contains("雜貨") {
        Some(format!(
            "{pronoun}拿著長柄竹夾自廣口玻璃罐中取出一塊金桔酸梅糖，用紙袋細心包妥交給買客。"
        ))
    } else if room_name.contains("修傘") || room_name.contains("補靴") {
        Some(format!(
            "{pronoun}手捏牛皮扁錐穿過行靴厚底，拉緊浸透蜂蠟的麻線，腳邊堆著幾把修好的桐油晴雨傘。"
        ))
    } else if room_name.contains("聯運") || room_name.contains("貨運") {
        Some(format!(
            "{pronoun}正把沉重的帆布貨包扎上馱架，雙手拽緊粗麻繩扣，反覆確認貨箱穩固。"
        ))
    } else if room_name.contains("石橋") || room_name.contains("樞紐") {
        Some(format!(
            "{pronoun}扶著雕花青石欄杆憑欄遠眺，目光跟隨橋下潺潺流水與穿梭兩岸的商旅背影。"
        ))
    } else if room_name.contains("分家巷") || room_name.contains("西門街") || room_name.contains("講堂街") {
        Some(format!(
            "{pronoun}邁步穿行於平整石板街巷間，神態悠然自得，不時與迎面而來的熱絡街坊點頭致意。"
        ))
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
            "{pronoun}扶住剛落下的外地貨箱，抬頭看港旗和海關前棚那邊的手勢。"
        ))
    } else if room_name.contains("海關前棚") {
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
            "{pronoun}撥了撥銅秤，將錢票和商館街口報來的價碼寫入賬冊。"
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
            "{pronoun}在排班板前換下一枚濕木牌，火盆煙氣貼著袖口散開。"
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
    } else {
        None
    }
}


fn looks_like_private_name(s: &str) -> bool {
    let mut count = 0usize;
    for ch in s.chars() {
        if !('\u{4e00}'..='\u{9fff}').contains(&ch) {
            return false;
        }
        count += 1;
    }
    (2..=4).contains(&count) && !looks_like_public_role(s)
}

fn looks_like_public_role(s: &str) -> bool {
    if is_final_map_public_role(s) {
        return true;
    }
    let roles = [
        "船戶",
        "渡工",
        "挑夫",
        "旅人",
        "商人",
        "攤販",
        "茶客",
        "行人",
        "獵戶",
        "道士",
        "守衛",
        "工匠",
        "學徒",
        "修船者",
        "船匠",
        "修船工",
        "倉管",
        "船客",
        "補帆人",
        "船行人",
        "船員",
        "稅吏",
        "海關吏",
        "商館人",
        "翻譯",
        "兌票人",
        "搬魚人",
        "補網人",
        "魚販",
        "船戶家屬",
        "守望人",
        "礦工",
        "洗礦工",
        "修具工",
        "推車人",
        "候渡者",
        "看貨人",
        "歇腳旅人",
        "候船者",
        "上岸客",
        "搬貨人",
    ];
    roles.iter().any(|role| s.contains(role))
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
    fn look_narrative_with_personality() {
        let mut ch = test_character();
        ch.soul_seed = Some(12345);
        let line = build_look_narrative(&ch, "某甲", "客棧前廊", None);
        assert!(!line.contains("。 "));
        assert!(line.contains("氣勢沉猛") || line.contains("神態內斂"));
        assert!(line.contains("目光敏銳") || line.contains("神色淡然"));
        assert!(line.contains("衣物整飭") || line.contains("行止隨性"));
    }

    #[test]
    fn generated_private_name_falls_back_to_stranger_label() {
        assert_eq!(observed_label_for_room("王小明", "半嶺水岸渡口"), "陌生人");
    }

    #[test]
    fn look_narrative_projects_active_npc_obligation() {
        let mut ch = test_character();
        ch.id = "test_ob_npc_1".to_string();
        ch.soul_seed = Some(42);
        crate::store::open_active_npc_obligation(&ch.id, "restock_goods", "店內現貨見底急需補倉");
        let line = build_look_narrative(&ch, "陳萬福", "文曜・三水口・三水食堂", None);
        assert!(line.contains("急需提貨補倉"));
        crate::store::clear_active_npc_obligation(&ch.id, "restock_goods");
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
        assert_eq!(observed_label_for_room("海關吏", "海關前棚"), "陌生人");
        assert_eq!(observed_label_for_room("商館人", "商館街口"), "陌生人");
        assert_eq!(observed_label_for_room("王小明", "匯兌小屋"), "陌生人");
    }
    #[test]
    fn foreign_trade_look_uses_trade_work_not_generic_passerby() {
        let customs = scene_observation_line("海關前棚", "他").expect("海關前棚應有觀察句");
        let exchange = scene_observation_line("匯兌小屋", "她").expect("匯兌小屋應有觀察句");

        assert!(customs.contains("貨單"));
        assert!(customs.contains("封條"));
        assert!(exchange.contains("銅秤"));
        assert!(exchange.contains("賬冊"));
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
        assert!(line.contains("手裡握著【精鍛防身獵刀"));
    }

    #[test]
    fn test_look_disposition_and_thoughts() {
        let mut ch = test_character();
        ch.disposition = 45;
        ch.thoughts.push(crate::entity::Thought {
            source: "Talked".to_string(),
            delta: 5,
            created_at: crate::game::now_unix(),
            decay_minutes: 180,
            flavor: Some("跟人說了會兒話，心裡踏實".to_string()),
        });
        let line = build_look_narrative(&ch, "某甲", "客棧前廊", None);
        assert!(line.contains("眉目舒朗、神采奕奕，心境頗為暢快"));
        assert!(line.contains("眉眼間似正思忖著「跟人說了會兒話，心裡踏實」。"));
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

        // 1. 內息底色
        assert!(narrative.contains("內息底色：【鍛爐沉息】"));
        assert!(narrative.contains("吐納沉如風箱鼓火"));

        // 2. 人器磨合與鋒刃器況
        assert!(narrative.contains("手裡握著【精鍛防身獵刀】（"));
        assert!(
            narrative.contains("漸趨順手・鋒芒畢露") || narrative.contains("沉穩趁手・鋒芒畢露")
        );

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
        assert!(!narrative.contains("剛才"));
        assert!(!narrative.contains("剛剛"));
        assert!(!narrative.contains("方才"));
        assert!(!narrative.contains("頃刻"));
        assert!(!narrative.contains("110V"));
        assert!(!narrative.contains("念場"));
        assert!(!narrative.contains("聖塔"));
        assert!(!narrative.contains("小市"));
        assert!(!narrative.contains("短市"));
        assert!(!narrative.contains("青籠關"));

        // 清理創傷
        crate::pg::living_world::clear_entity_afflictions(&ch.id, true, crate::game::now_unix());
    }
}

//! 閒置節拍：NPC↔NPC AI、微互動、在職巡邏與閒置動作。

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, RwLock};

static ROOM_IDLE_CHAT_LAST_TS: LazyLock<RwLock<HashMap<String, i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

use rand::Rng;
use crate::config::{Server, sim};
use crate::db::{
    canonical_location_key, get_all_schedules, get_disposition, get_entity_room,
    get_npc_person_display_name, get_npc_title_from_assignments, get_room_name, set_entity_room,
};
use crate::gametext;
use crate::npc::{
    get_time_period, get_wander_flavor, get_wander_rooms, pick_idle_emote, pick_micro_interaction,
    pick_random_npc_npc_topic_by_mask,
};
use crate::npcnpc::{push_room_event, topic_mask_for_room, try_trigger_npc_npc_in_room};

use super::super::SessionStore;
use super::super::broadcast::{refresh_room_views_for_room, send_narrate_to_room};
use super::MainLoopTickState;
use super::helpers::{roll_next_idle_interval, sprintf_s};

fn resolve_caller_name(player_id: &str, npc_id: &str, ent: &crate::store::Entity) -> String {
    let named = crate::db::get_acquaintance(player_id, npc_id)
        .ok()
        .flatten()
        .map(|a| a.named)
        .unwrap_or(false);
    if named {
        let t = ent.display_title.trim();
        if let Some((_, n)) = t.split_once('｜').or_else(|| t.split_once('|')) {
            n.trim().to_string()
        } else if !t.is_empty() {
            t.to_string()
        } else {
            npc_id.to_string()
        }
    } else {
        let room_name = if let (Some(gx), Some(gy)) = (ent.grid_x, ent.grid_y) {
            crate::server::grid_manager::get_square_grid()
                .and_then(|g| g.get(crate::grid::SquareCoord::new(gx, gy)).map(|c| c.name.clone()))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let (is_staff, has_uniform, in_counter_zone) = crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            &room_name,
            npc_id,
            &ent.display_title,
            &ent.current_activity,
            &ent.equipment_slots,
        );
        if is_staff {
            let host_title = crate::grid::resolve_any_room_venue_object(&room_name, &ent.display_title)
                .map(|v| v.default_host_title)
                .unwrap_or("當值人員");
            if in_counter_zone && has_uniform {
                return format!("身著工作制服、站在櫃檯內側的{host_title}");
            } else if in_counter_zone {
                return format!("站在櫃檯內側的{host_title}");
            } else if has_uniform {
                return format!("身著工作制服的{host_title}");
            }
        }
        let tag = crate::store::npc_brief_appearance_tag(npc_id, &ent.display_title);
        let role = if ent.display_title.contains("鐵") || ent.display_title.contains("鍛造") || ent.display_title.contains("木作") || ent.display_title.contains("工坊") {
            "匠人"
        } else if ent.display_title.contains("商") || ent.display_title.contains("鋪") || ent.display_title.contains("市集") || ent.display_title.contains("行") {
            "行商"
        } else if ent.display_title.contains("醫") || ent.display_title.contains("藥") || ent.display_title.contains("診") {
            "醫工"
        } else if ent.display_title.contains("書") || ent.display_title.contains("學") || ent.display_title.contains("講堂") || ent.display_title.contains("觀星") {
            "學者"
        } else {
            "街坊"
        };
        if !tag.is_empty() {
            format!("{tag}的{role}")
        } else {
            format!("一名{role}")
        }
    }
}

fn generate_vendor_banter(caller_a: &str, caller_b: &str, id_a: &str, id_b: &str, hour: i32) -> String {
    if (id_a.contains("roast_beef") && id_b.contains("noodle_soup")) || (id_b.contains("roast_beef") && id_a.contains("noodle_soup")) {
        format!("{caller_a}抽空朝隔壁攤招呼道：「掌麵大姊，待會幫我留碗大骨湯切仔麵，加顆滷蛋！」 {caller_b}笑著應道：「好咧掌爐師傅，等這陣烤肉香氣散了，我親自端過去給你！」")
    } else if (id_a.contains("pepper_bun") && id_b.contains("tofu_pudding")) || (id_b.contains("pepper_bun") && id_a.contains("tofu_pudding")) {
        format!("{caller_a}抽空朝隔壁攤笑道：「老伯，待會留碗黑糖薑汁豆花，吃完現烤出爐的胡椒餅配碗熱甜湯最舒坦！」 {caller_b}樂呵呵應道：「好咧掌攤大哥！我給你多舀一勺黑糖粉圓，趁熱吃啊！」")
    } else if (id_a.contains("herbal_tea") && id_b.contains("roast_beef")) || (id_b.contains("herbal_tea") && id_a.contains("roast_beef")) {
        format!("{caller_a}朝隔壁笑道：「掌爐師傅，烤肉串火氣大，待會過來喝盞寒水玉仙草凍降降火！」 {caller_b}爽朗應道：「鄰攤掌事想得周到！等我烤完這爐牛肉番薯，馬上過去盛一盞！」")
    } else if (id_a.contains("herbal_tea") && id_b.contains("pepper_bun")) || (id_b.contains("herbal_tea") && id_a.contains("pepper_bun")) {
        format!("{caller_a}笑著朝隔壁打趣道：「掌攤大哥，胡椒餅現烤出爐辛香撲鼻，配我這冰鎮仙草凍正是絕配呢！」 {caller_b}哈哈笑道：「鄰攤掌事說得對，一個燙口酥脆一個沁涼消暑，稍後我也來一盞！」")
    } else if (id_a.contains("tofu_pudding") && id_b.contains("herbal_tea")) || (id_b.contains("tofu_pudding") && id_a.contains("herbal_tea")) {
        format!("{caller_a}笑著打招呼道：「鄰攤掌事，今天仙草凍備得足呀，井泉鎮得真透涼！」 {caller_b}點頭應道：「老伯，您的黑糖薑汁豆花也是香飄整條街呢，大家今晚生意都旺！」")
    } else if (id_a.contains("rice_roll") && id_b.contains("soy_milk")) || (id_b.contains("rice_roll") && id_a.contains("soy_milk")) {
        format!("{caller_a}隔空喊道：「掌攤大哥，現磨熱米漿幫我盛一碗，搭我親手捏好的紫米飯糰正合適！」 {caller_b}朗聲回道：「沒問題大姨，我再給你挑根最酥脆的現炸油條！」")
    } else if (id_a.contains("morning_produce") && id_b.contains("morning_fish")) || (id_b.contains("morning_produce") && id_a.contains("morning_fish")) {
        format!("{caller_a}朝隔壁笑喊：「掌攤大哥，今天這河蝦個個活蹦亂跳啊，搭我的鮮嫩水芹菜下鍋清炒最甜了！」 {caller_b}高聲笑應：「對攤老弟眼光好！稍後收市，我們換點菜蝦回家下酒！」")
    } else if hour >= 18 || hour < 6 {
        format!("{caller_a}望向隔壁攤笑道：「今晚夜市人潮真旺，我這鍋備料都快見底啦！」 {caller_b}點頭應道：「是啊，趁著夜風涼爽、街燈亮堂，大夥手腳都俐落些！」")
    } else {
        format!("{caller_a}望向隔壁攤笑道：「今天生意不錯呀，街上往來買賣的人潮挺旺！」 {caller_b}點頭應道：「是啊，趁著日頭正好、市面熱鬧，大夥都精神！」")
    }
}
fn sanitize_thought_string(raw: &str, role_hint: &str, seed: u64) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }

    if t == concat!("跟人說了會", "兒話") || t.starts_with(concat!("跟人說了會", "兒話")) {
        let varied = [
            "與街坊聊起近日市集物價與水路貨況，心裡有了底",
            "同熟識的鄰里寒暄幾句家常，神色舒緩",
            "同過路的行商打聽各處商道物料行情，頗有收穫",
            "與街邊鄰里互道安好，說了些坊間近況，心緒平靜",
        ];
        return varied[(seed as usize) % varied.len()].to_string();
    }

    if t.contains("並結清帳目，手頭進帳") || t.contains("結清帳目") || t.contains("手頭進帳") {
        let venue = if let Some(first_part) = t.split("完成「").next() {
            let v = first_part.trim_start_matches("在").trim();
            if v.is_empty() { "坊間" } else { v }
        } else {
            "坊間"
        };

        if t.contains("浴") || t.contains("沐浴") {
            return format!("在{}泡過溫水洗去一身疲乏，筋骨格外舒坦", venue);
        } else if t.contains("食堂") || t.contains("茶") || t.contains("納涼") || t.contains("用膳") || t.contains("麵") || t.contains("飯") {
            return format!("在{}歇過腳、喝過熱茶，精神舒爽不少", venue);
        } else if t.contains("診") || t.contains("藥") || t.contains("調理") {
            return format!("在{}請醫工切脈調理氣息，身上輕鬆了許多", venue);
        } else if t.contains("歇") || t.contains("宿") || t.contains("客寮") || t.contains("旅社") {
            return format!("在{}歇息養神，精神健旺不少", venue);
        } else if t.contains("散步") || t.contains("閒談") || t.contains("寒暄") {
            return "同熟識的鄰里寒暄幾句家常，神色舒緩".to_string();
        } else {
            let is_forager = role_hint.contains("獵")
                || role_hint.contains("入山百工")
                || role_hint.contains("採")
                || role_hint.contains("樵")
                || role_hint.contains("礦工")
                || role_hint.contains("行旅");
            let manufactured = ["麻繩卷", "塗焦麻繩", "補綴工靴", "粗鹽包", "木炭袋", "鐵料盒"]
                .iter()
                .any(|m| t.contains(m));
            if !is_forager && manufactured {
                return "在市集核對物資採辦，把各色日常用度置辦妥當，心裡踏實".to_string();
            }
        }
    }

    if t.contains("記掛著「") || t.contains("記掛著『") {
        let inside = if let Some(p) = t.split("記掛著「").nth(1).and_then(|p| p.split("」").next()) {
            Some(p)
        } else if let Some(p) = t.split("記掛著『").nth(1).and_then(|p| p.split("』").next()) {
            Some(p)
        } else {
            None
        };
        if let Some(inner) = inside {
            if inner.contains("內室榻上熄燈安歇") || inner.contains("熄燈安歇") {
                if role_hint.contains("水輪") || role_hint.contains("石磨") {
                    return "檢查完老堰頭引水石堰插板，伴著窗外水輪規律的水聲安穩養神".to_string();
                }
                return "夜深熄燈安歇，養足精神以備明日早工".to_string();
            }
            if inner.contains("伴著水輪轉動的規律水聲歇息") {
                return "檢查引水石堰的水門插板，伴著水輪轉動的規律水聲安穩歇息".to_string();
            }
            if inner.chars().count() > 28 {
                let concise = if inner.contains("石磨") || inner.contains("水輪") || inner.contains("碾") || inner.contains("米漿") {
                    "水輪石磨碾米與現磨米漿工序"
                } else if inner.contains("引水入渠") || inner.contains("水門") || inner.contains("堰渠") {
                    "堰渠水門調度與明日引水時刻"
                } else if inner.contains("家常") || inner.contains("家務") || inner.contains("居") || inner.contains("宅") {
                    "家中雜務"
                } else if inner.contains("採辦") || inner.contains("採買") || inner.contains("市集") {
                    "市集採辦"
                } else if inner.contains("工坊") || inner.contains("活計") || inner.contains("打磨") || inner.contains("鐵") || inner.contains("木") {
                    "手頭活計"
                } else if inner.contains("診所") || inner.contains("抓藥") || inner.contains("切脈") {
                    "診所抓藥"
                } else {
                    "手頭差事"
                };
                return format!("心裡記掛著『{}』，正按部就班料理妥當", concise);
            }
        }
    }
    if (role_hint.contains("水輪") || role_hint.contains("石磨"))
        && (t.contains("記掛著『診所抓藥』") || t.contains("記掛著『手頭差事』"))
    {
        return "心裡記掛著『水輪石磨碾米與現磨米漿工序』，正按部就班料理妥當".to_string();
    }

    t.to_string()
}

pub(super) fn reconcile_simulation_errands_and_thoughts(_hour: i32) {
    let Some(st_arc) = crate::store::get_store() else {
        return;
    };

    // 1. Audit and correct live NPC grid states (snapshot titles first so STORE and LIVE_NPC_GRID_STATES never invert lock order)
    let npc_titles: std::collections::HashMap<String, String> = {
        let st = st_arc.read();
        st.entities
            .iter()
            .filter(|(_, e)| e.kind == "npc")
            .map(|(k, e)| (k.clone(), e.display_title.clone()))
            .collect()
    };
    {
        let mut live_states = crate::store::LIVE_NPC_GRID_STATES.write();

        for (npc_id, state) in live_states.iter_mut() {
            let Some(display_title) = npc_titles.get(npc_id) else {
                continue;
            };
            let slot = crate::store::wenyao_household_slot_for(npc_id);
            let role_full = format!(
                "{} {} {} {} {} {}",
                display_title,
                slot.brief_tag,
                slot.home_name,
                slot.errand_name,
                state.dest_name,
                state.purpose
            );
            let is_forager = [
                "獵", "入山百工", "礦脈百工", "林場百工", "古道行旅", "商號行商",
                "採", "樵", "礦", "行旅", "護送", "押送", "轉運",
            ]
            .iter()
            .any(|k| role_full.contains(k));

            let is_wilderness_dest = [
                "白塔雪巔", "赤石坡", "落日岬", "古河巨木林", "長風草甸", "西嶺礦谷", "清泉溪谷",
                "翠冠巨木林", "北坡雲杉林", "雲腰雪林", "東海峭壁", "淺澤泥口", "驚濤岩岸",
                "巨藻淺灣", "黑石灣", "白茫洲", "貼山深潭", "青籠關", "西麓林道",
            ]
            .iter()
            .any(|z| state.dest_name.contains(z));

            let home_or_errand_in_wilderness = [
                "白塔雪巔", "赤石坡", "落日岬", "古河巨木林", "長風草甸", "西嶺礦谷", "清泉溪谷",
                "翠冠巨木林", "北坡雲杉林", "雲腰雪林", "東海峭壁", "淺澤泥口", "驚濤岩岸",
                "巨藻淺灣", "黑石灣", "白茫洲", "貼山深潭", "青籠關", "西麓林道",
            ]
            .iter()
            .any(|z| slot.home_name.contains(z) || slot.errand_name.contains(z));

            let has_procure = state.purpose.contains("採辦急缺")
                || state.purpose.contains("採辦應急物產")
                || state.stay_activity.contains("急缺");

            let manufactured = ["麻繩卷", "塗焦麻繩", "補綴工靴", "粗鹽包", "木炭袋", "鐵料盒", "杉木厚板"]
                .iter()
                .any(|m| state.purpose.contains(m) || state.stay_activity.contains(m));

            if !is_forager && !home_or_errand_in_wilderness && (is_wilderness_dest || (has_procure && manufactured)) {
                let is_in_wilderness = [
                    "白塔雪巔", "赤石坡", "落日岬", "古河巨木林", "長風草甸", "西嶺礦谷", "清泉溪谷",
                    "翠冠巨木林", "北坡雲杉林", "雲腰雪林", "東海峭壁", "淺澤泥口", "驚濤岩岸",
                    "巨藻淺灣", "黑石灣", "白茫洲", "貼山深潭", "青籠關", "西麓林道",
                ]
                .iter()
                .any(|z| state.origin_name.contains(z));

                if is_in_wilderness {
                    state.dest = (slot.home_x, slot.home_y);
                    state.dest_name = slot.home_name.to_string();
                    state.purpose = "返家照看家務".to_string();
                    state.stay_activity = slot.home_stay_activity.to_string();
                    state.stay_ticks_remaining = 4;
                } else {
                    state.dest = (slot.errand_x, slot.errand_y);
                    state.dest_name = slot.errand_name.to_string();
                    state.purpose = slot.errand_purpose.to_string();
                    state.stay_activity = slot.errand_stay_activity.to_string();
                    state.stay_ticks_remaining = 4;
                }
                crate::store::clear_active_npc_obligation(npc_id, "");
            } else if is_forager && has_procure && manufactured {
                let raw_material = match (npc_id.len()) % 4 {
                    0 => "山野鮮藥草",
                    1 => "粗生礦石",
                    2 => "山林硬木料",
                    _ => "野味肉料",
                };
                state.purpose = format!("入山採辦{}", raw_material);
                state.stay_activity = format!("正於{}一帶尋覓並採集野生的「{}」", state.dest_name, raw_material);
            }
        }
    }

    // 2. Audit and sanitize live thoughts in entity store
    {
        let mut st = st_arc.write();
        for ent in st.entities.values_mut() {
            if ent.kind != "npc" {
                continue;
            }
            let mut mutated = false;
            let seed = ent.soul_seed.unwrap_or(0).unsigned_abs();
            let title = ent.display_title.clone();
            for t in &mut ent.thoughts {
                if let Some(flav) = &t.flavor {
                    let clean = sanitize_thought_string(flav, &title, seed);
                    if clean != *flav {
                        t.flavor = Some(clean);
                        mutated = true;
                    }
                }
            }
            if mutated {
                ent.disposition = ent.disposition.clamp(-100, 100);
                crate::pg::entity::upsert_async(ent.clone());
            }
        }
    }
}


fn resolve_ambient_caller_label(player_id: &str, npc_id: &str) -> String {
    let named = crate::store::player_knows_npc(player_id, npc_id);
    let person = get_npc_person_display_name(npc_id).unwrap_or_default();
    if named && !person.is_empty() {
        return person;
    }
    let store_opt = crate::store::get_store();
    let ent_opt = store_opt.as_ref().and_then(|s| s.read().get_entity(npc_id));
    let mut title = get_npc_title_from_assignments(npc_id);
    if title.is_empty() {
        if let Some(ent) = &ent_opt {
            title = ent.display_title.clone();
        }
    }
    if title.is_empty() {
        title = gametext::default_manager_title();
    }
    let appearance = crate::store::npc_brief_appearance_tag(npc_id, &title);
    if let Some(ent) = &ent_opt {
        let room_name = if let (Some(gx), Some(gy)) = (ent.grid_x, ent.grid_y) {
            crate::server::grid_manager::get_square_grid()
                .and_then(|g| g.get(crate::grid::SquareCoord::new(gx, gy)).map(|c| c.name.clone()))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let (is_staff, has_uniform, in_counter_zone) = crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            &room_name,
            npc_id,
            &ent.display_title,
            &ent.current_activity,
            &ent.equipment_slots,
        );
        if is_staff {
            let host_title = crate::grid::resolve_any_room_venue_object(&room_name, &ent.display_title)
                .map(|v| v.default_host_title)
                .unwrap_or("當值主事");
            if in_counter_zone {
                return if !appearance.is_empty() {
                    format!("站在櫃檯內側的{host_title}（{appearance}）")
                } else {
                    format!("站在櫃檯內側的{host_title}")
                };
            } else if has_uniform {
                return if !appearance.is_empty() {
                    format!("身著工作制服的{host_title}（{appearance}）")
                } else {
                    format!("身著工作制服的{host_title}")
                };
            }
        }
    }

    let role = if title.contains("鐵") || title.contains("鍛造") || title.contains("木作") || title.contains("工坊") {
        "匠人"
    } else if title.contains("商") || title.contains("鋪") || title.contains("市集") || title.contains("行") {
        "行商"
    } else if title.contains("醫") || title.contains("藥") || title.contains("診") {
        "醫工"
    } else if title.contains("書") || title.contains("學") || title.contains("講堂") || title.contains("觀星") {
        "學者"
    } else {
        "街坊"
    };
    if !appearance.is_empty() {
        format!("{appearance}的{role}")
    } else {
        format!("一名{role}")
    }
}

pub(super) fn run_idle_wander_section(
    sessions: &SessionStore,
    cfg: &Server,
    hour: i32,
    state: &Arc<Mutex<MainLoopTickState>>,
) {
    let run = {
        let Ok(mut st) = state.lock() else {
            return;
        };
        st.idle_tick_count += 1;
        if st.idle_tick_count >= st.next_idle_trigger {
            st.idle_tick_count = 0;
            st.next_idle_trigger = roll_next_idle_interval();
            true
        } else {
            false
        }
    };
    if !run {
        return;
    }
    reconcile_simulation_errands_and_thoughts(hour);
    let period = get_time_period(hour);
    let mut player_room_set = sessions.player_room_ids();
    for pid in sessions.all_player_ids() {
        if let Ok(Some(ent)) = crate::db::get_entity(&pid) {
            if let (Some(gx), Some(gy)) = (ent.grid_x, ent.grid_y) {
                player_room_set.insert(crate::grid::grid_room_id_from_coord(gx, gy));
            }
        }
    }
    let ollama_ready = !cfg.ollama_disable;
    let mut dialogue_done = false;
    if ollama_ready {
        for room_id in &player_room_set {
            let topic_hint =
                pick_random_npc_npc_topic_by_mask(&topic_mask_for_room(room_id, hour), "")
                    .map(|t| t.hint)
                    .unwrap_or_default();
            if try_trigger_npc_npc_in_room(sessions, cfg, room_id, &topic_hint, hour) {
                dialogue_done = true;
                break;
            }
        }
        if !dialogue_done {
            let mut rng = rand::rng();
            if rng.random_range(0..100) < 25 {
                if let Some(store) = crate::store::get_store() {
                    let candidate_rooms: Vec<String> = {
                        let st = store.read();
                        let live_states = crate::store::LIVE_NPC_GRID_STATES.read();
                        let mut coord_map: HashMap<(i32, i32), usize> = HashMap::new();
                        for e in st.entities.values() {
                            if e.kind == "npc" && e.vit > 0 {
                                let (coord, act) = if let Some(state) = live_states.get(&e.id) {
                                    (Some(state.current), state.current_activity())
                                } else {
                                    let c = match (e.grid_x, e.grid_y) {
                                        (Some(x), Some(y)) => Some((x, y)),
                                        _ => None,
                                    };
                                    (c, e.current_activity.clone())
                                };
                                let is_awake = !act.contains("安歇")
                                    && !act.contains("熟睡")
                                    && !act.contains("熄燈")
                                    && !act.contains("就寢")
                                    && !crate::store::is_npc_paused_for_interaction(&e.id);
                                if is_awake {
                                    if let Some(c) = coord {
                                        *coord_map.entry(c).or_insert(0) += 1;
                                    }
                                }
                            }
                        }
                        coord_map
                            .into_iter()
                            .filter(|((x, y), count)| {
                                if *count < 2 {
                                    return false;
                                }
                                let rid = crate::grid::grid_room_id_from_coord(*x, *y);
                                !player_room_set.contains(&rid)
                            })
                            .map(|((x, y), _)| crate::grid::grid_room_id_from_coord(x, y))
                            .collect()
                    };

                    if !candidate_rooms.is_empty() {
                        let idx = rng.random_range(0..candidate_rooms.len());
                        let offscreen_room_id = &candidate_rooms[idx];
                        let topic_hint = pick_random_npc_npc_topic_by_mask(
                            &topic_mask_for_room(offscreen_room_id, hour),
                            "",
                        )
                        .map(|t| t.hint)
                        .unwrap_or_default();
                        if try_trigger_npc_npc_in_room(
                            sessions,
                            cfg,
                            offscreen_room_id,
                            &topic_hint,
                            hour,
                        ) {
                            dialogue_done = true;
                        }
                    }
                }
            }
        }
    }
    if !dialogue_done {
        let mut micro_pct = sim().micro_interaction_chance_percent;
        if micro_pct <= 0 {
            micro_pct = 15;
        }
        let mut rng = rand::rng();
        let is_deep_night = hour >= 22 || hour < 5;
        for room_id in &player_room_set {
            let room_name = if let Some(coord) = crate::grid::parse_grid_room_id(room_id)
                && let Some(cell) = crate::server::grid_manager::get_runtime_grid_cell(coord)
            {
                if cell.name.trim().is_empty() {
                    crate::grid::terrain_name_zh(cell.terrain).to_string()
                } else {
                    cell.name
                }
            } else {
                get_room_name(room_id).unwrap_or_default()
            };
            let is_daytime_facility = room_name.contains("工坊")
                || room_name.contains("印行")
                || room_name.contains("造紙")
                || room_name.contains("典藏庫")
                || room_name.contains("學院")
                || room_name.contains("講堂")
                || room_name.contains("講舍")
                || room_name.contains("校舍")
                || room_name.contains("蒙學")
                || room_name.contains("學舍")
                || room_name.contains("商行")
                || room_name.contains("商會")
                || room_name.contains("鋪")
                || room_name.contains("鍛造")
                || room_name.contains("木作")
                || room_name.contains("織造")
                || room_name.contains("市集")
                || room_name.contains("藏書閣")
                || room_name.contains("烘藥房")
                || room_name.contains("曆算");

            let (_npc_ids, active_npcs) = if let Some(store) = crate::store::get_store() {
                let st = store.read();
                let ids = st.entity_ids_in_room(room_id);
                let alive: Vec<String> = ids
                    .iter()
                    .filter(|id| st.get_entity(id).is_some_and(|e| e.kind == "npc" && e.vit > 0))
                    .cloned()
                    .collect();
                (ids, alive)
            } else {
                (Vec::new(), Vec::new())
            };

            // 深夜時段日間機構（工坊、印行館、學院、商行等）若無清醒活動人員，改為寧靜夜色描寫，不廣播日間勞作句子
            if is_deep_night && is_daytime_facility {
                if rng.random_range(0..100) < micro_pct {
                    let night_lines = [
                        "夜深人靜，廊下防風燈映著緊閉的木門與靜置的案几。",
                        "四下萬籟俱寂，夜風吹拂著屋簷銅鈴，屋內陳設在夜色中安寧沉靜。",
                        "長夜漫漫，街坊燈火漸熄，唯有案角微弱的貝母珠光靜靜映照著沉睡的館閣。",
                    ];
                    let pick = night_lines[rng.random_range(0..night_lines.len())];
                    send_narrate_to_room(sessions, room_id, pick);
                    break;
                }
                continue;
            }

            // 優先根據在場實體生成現場生活動靜（攤販叫賣與雙人真實開口交談）
            let want = crate::db::canonical_location_key(room_id);
            let now_ts = crate::game::now_unix();

            // 推進自主經濟循環與百工生產步調（必須在取得 store.read() 前執行，避免 RwLock 讀寫重入死鎖）
            let can_run_eco = {
                let lock = ROOM_IDLE_CHAT_LAST_TS.read().unwrap();
                let last = lock.get(room_id).copied().unwrap_or(0);
                now_ts.saturating_sub(last) >= 35
            };
            if can_run_eco && (now_ts as usize / 10) % 3 == 0 {
                if let Some(eco_narrate) = crate::game::object_ecology::run_npc_autonomous_production_step(room_id, &room_name, &active_npcs, now_ts) {
                    ROOM_IDLE_CHAT_LAST_TS.write().unwrap().insert(room_id.to_string(), now_ts);
                    for s in sessions.all_sessions() {
                        if crate::server::session::player_location_key(&s.player_id) == want {
                            crate::server::send_narrate_to_player(&s, &eco_narrate);
                        }
                    }
                    break;
                }
            }

            if let Some(store) = crate::store::get_store() {
                let npc_snapshots: HashMap<String, (crate::store::Entity, String)> = {
                    let st = store.read();
                    let mut map = HashMap::new();
                    for id in &active_npcs {
                        if let Some(ent) = st.get_entity(id) {
                            let act = st.effective_npc_activity(&ent);
                            map.insert(id.clone(), (ent, act));
                        }
                    }
                    map
                };
                let mut presence_reflex_done = false;
                let awake_present: Vec<String> = active_npcs
                    .iter()
                    .filter(|id| {
                        if crate::store::is_npc_paused_for_interaction(id) {
                            return false;
                        }
                        npc_snapshots.get(id.as_str()).is_some_and(|(e, _)| {
                            !e.current_activity.contains("安歇")
                                && !e.current_activity.contains("熟睡")
                                && !e.current_activity.contains("熄燈")
                                && !e.current_activity.contains("就寢")
                        })
                    })
                    .cloned()
                    .collect();

                if rng.random_range(0..100) < 30 {
                    for s in sessions.all_sessions() {
                        if crate::server::session::player_location_key(&s.player_id) == want {
                            if let Some(known_id) = awake_present.iter().find(|id| crate::store::player_knows_npc(&s.player_id, id)) {
                                if let Some((ea, _)) = npc_snapshots.get(known_id.as_str()) {
                                    let caller = resolve_caller_name(&s.player_id, known_id, ea);
                                    let soul = ea
                                        .soul_seed
                                        .map(crate::db::expand_soul_seed_to_personality)
                                        .unwrap_or(crate::db::Personality {
                                            boldness: 0.5,
                                            sensitivity: 0.5,
                                            orderliness: 0.5,
                                        });
                                    let act_now = ea.current_activity.clone();
                                    let disp = ea.disposition;
                                    let fav = crate::db::get_favorability(known_id, &s.player_id);
                                    let (fam_score, bal_score) = if let Some(pool) = crate::pg::pool() {
                                        let rel = crate::pg::living_world::get_relationship_summary(&pool, known_id, &s.player_id).unwrap_or_default();
                                        let fam = rel.familiarity_score.max(if crate::store::player_knows_npc(&s.player_id, known_id) { 2 } else { 1 });
                                        let bal = rel.balance_score + (fav / 10) + (disp / 25);
                                        (fam, bal)
                                    } else {
                                        let fam = if crate::store::player_knows_npc(&s.player_id, known_id) { 2 } else { 1 };
                                        let bal = (fav / 10) + (disp / 25);
                                        (fam, bal)
                                    };
                                    let stance = crate::ai::decision_scorer::score_npc_interaction_stance(
                                        &crate::ai::decision_scorer::NpcDecisionInput {
                                            npc_name: &caller,
                                            boldness: soul.boldness,
                                            sensitivity: soul.sensitivity,
                                            orderliness: soul.orderliness,
                                            familiarity_score: fam_score,
                                            balance_score: bal_score,
                                            open_obligations: &[],
                                            current_activity: &act_now,
                                            action_kind: "presence_reflex",
                                            player_utterance: "行旅者駐足於身側",
                                        },
                                    );
                                    if stance.choice != crate::ai::decision_scorer::NpcStanceChoice::Refuse {
                                        let reflex = match stance.choice {
                                            crate::ai::decision_scorer::NpcStanceChoice::Accept => {
                                                format!("{caller}注意到你的步履，溫和地點頭致意：「旅途可還順遂？若有事需要打點隨時開口。」")
                                            }
                                            crate::ai::decision_scorer::NpcStanceChoice::Bargain => {
                                                format!("{caller}停下手邊活計望向你，爽快招呼道：「來得正好！手頭正有一樁活計與市面貨源想與熟人合計合計。」")
                                            }
                                            _ => {
                                                format!("{caller}抬眼見你在側，目光微動，含笑頷首示意。")
                                            }
                                        };
                                        crate::server::send_narrate_to_player(&s, &reflex);
                                        presence_reflex_done = true;
                                        break;
                                    }
                                }
                            }
                            if !presence_reflex_done {
                                if let Some(artisan_id) = awake_present.iter().find(|id| {
                                    id.starts_with("wt_vendor_")
                                        || id.contains("smith")
                                        || id.contains("carpenter")
                                        || crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(&room_name, id, "", "", "").0
                                }) {
                                    if let Some((ea, _)) = npc_snapshots.get(artisan_id.as_str()) {
                                        let caller = resolve_caller_name(&s.player_id, artisan_id, ea);
                                        let soul = ea
                                            .soul_seed
                                            .map(crate::db::expand_soul_seed_to_personality)
                                            .unwrap_or(crate::db::Personality {
                                                boldness: 0.5,
                                                sensitivity: 0.5,
                                                orderliness: 0.5,
                                            });
                                        let act_now = ea.current_activity.clone();
                                        let stance = crate::ai::decision_scorer::score_npc_interaction_stance(
                                            &crate::ai::decision_scorer::NpcDecisionInput {
                                                npc_name: &caller,
                                                boldness: soul.boldness,
                                                sensitivity: soul.sensitivity,
                                                orderliness: soul.orderliness,
                                                familiarity_score: 20,
                                                balance_score: 1,
                                                open_obligations: &[],
                                                current_activity: &act_now,
                                                action_kind: "workshop_presence_reflex",
                                                player_utterance: "行旅者攜帶原料與器物步入工坊",
                                            },
                                        );
                                        if stance.choice != crate::ai::decision_scorer::NpcStanceChoice::Refuse {
                                            let reflex = format!("{caller}留意到你在旁端詳，善意地提醒道：「客友若有採集得來的草木金石原料，或想將隨身器具百工煉製、催動氣脈異變，爐前台案隨時可用。」");
                                            crate::server::send_narrate_to_player(&s, &reflex);
                                            presence_reflex_done = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if presence_reflex_done {
                    break;
                }
                // 優先檢查看是否有正在營業擺攤的攤販
                let vendor_npcs: Vec<String> = active_npcs
                    .iter()
                    .filter(|id| {
                        if let Some((e, _)) = npc_snapshots.get(id.as_str()) {
                            let act = e.current_activity.as_str();
                            let is_excluded = act.contains("正在前往")
                                || act.contains("動身前往")
                                || act.contains("返回【")
                                || act.contains("正步行前往")
                                || act.contains("正沿街前往")
                                || act.contains("正推著攤車")
                                || act.contains("已收攤")
                                || act.contains("邁步離開")
                                || act.contains("安歇")
                                || act.contains("熟睡")
                                || act.contains("就寢");
                            if is_excluded {
                                return false;
                            }
                            let is_vending = act.contains('攤')
                                || act.contains("叫賣")
                                || act.contains("吆喝")
                                || act.contains("招呼")
                                || act.contains("楦頭");
                            (id.starts_with("wt_vendor_") || id.as_str() == "wt_npc_ma_buxue") && is_vending
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();

                let get_vendor_bark = |v_id: &str, caller: &str| -> String {
                    if v_id == "wt_vendor_roast_beef" {
                        format!("{caller}一邊翻動炭火烤網上滋滋冒油的肉串，一邊高聲叫賣：「街角炭烤原牛肉串配焦蜜烤番薯喔——炭火現烤、焦香鮮嫩，來一串暖暖胃！」")
                    } else if v_id == "wt_vendor_noodle_soup" {
                        format!("{caller}掀開大銅鍋蓋，伴著滾燙大骨白煙清脆招呼：「切仔湯麵配滷蛋熱騰騰出鍋喔！大骨高湯現點現煮，來坐吃麵！」")
                    } else if v_id == "wt_vendor_pepper_bun" {
                        format!("{caller}用扁鐵鏟從鐵桶內壁鏟起金黃焦脆的熱餅，高聲吆喝：「現烤出爐的酥皮鮮肉胡椒餅！老麵貼爐、皮酥蔥香肉汁滿，小心燙口喔——」")
                    } else if v_id == "wt_vendor_herbal_tea" {
                        format!("{caller}手搖清脆銅盞叮噹作響，笑著叫賣：「寒水玉冰鎮黑糖仙草凍來囉！慢熬仙草沁涼滑嫩、生津止渴，解膩消暑最對味喔！」")
                    } else if v_id == "wt_vendor_tofu_pudding" {
                        format!("{caller}手持扁銅杓片出滑嫩豆花，朗聲招呼：「古早味黑糖薑汁熱豆花——井泉鹽滷、慢火黑糖粉圓配薑汁暖胃補氣，來坐喔！」")
                    } else if v_id == "wt_vendor_rice_roll" {
                        format!("{caller}掀開大木甑散發出撲鼻荷葉香氣，清脆吆喝：「熱騰騰蒸好的荷葉糯米炊飯！粒粒糯米透著竹香，吃一顆飽足頂餓喔！」")
                    } else if v_id == "wt_vendor_soy_milk" {
                        format!("{caller}攪拌著滾燙濃香的銅鍋，朗聲招呼：「老堰頭現磨溫米漿配現炸酥脆油條喔！石磨現磨香濃滑順，熱氣騰騰喝一碗神清氣爽！」")
                    } else if v_id == "wt_vendor_morning_produce" {
                        format!("{caller}往竹簍鮮菜灑了點清泉水，笑著吆喝：「老堰頭水芹菜脆嫩莖山泉現採下山喔！山泉灌溉、梗脆帶露清甜，隨便挑隨便選！」")
                    } else if v_id == "wt_vendor_morning_fish" {
                        format!("{caller}指著木盆裡水花直濺的鮮活河蝦，大聲叫賣：「水尾寮淺澤青殼河蝦現撈上岸！隻隻鮮活生猛、殼薄肉甜，買回去煮湯下酒最鮮甜！」")
                    } else if v_id == "wt_npc_ma_buxue" {
                        format!("{caller}敲了敲硬木楦頭上的厚牛皮掌，朗聲招呼：「釘鞋底、換皮掌、修補韌竹骨桐油晴雨摺疊傘——現修現拿，耐走防雨喔！」")
                    } else {
                        format!("{caller}熱情招呼道：「走過路過來瞧瞧，新出爐的手藝，新鮮道地喔！」")
                    }
                };

                if !vendor_npcs.is_empty() {
                    let step = (now_ts as usize) / 4;
                    let v_idx = step % vendor_npcs.len();
                    let mode = step % 3;
                    if mode == 2 && vendor_npcs.len() >= 2 {
                        let id_a = vendor_npcs[v_idx].clone();
                        let id_b = vendor_npcs[(v_idx + 1) % vendor_npcs.len()].clone();
                        if let (Some((ea, _)), Some((eb, _))) = (npc_snapshots.get(id_a.as_str()), npc_snapshots.get(id_b.as_str())) {
                            for s in sessions.all_sessions() {
                                if crate::server::session::player_location_key(&s.player_id) == want {
                                    let caller_a = resolve_caller_name(&s.player_id, &id_a, ea);
                                    let caller_b = resolve_caller_name(&s.player_id, &id_b, eb);
                                    let banter = generate_vendor_banter(&caller_a, &caller_b, &id_a, &id_b, hour);
                                    crate::store::record_active_npc_dialogue(room_id, &id_a, &id_b, &banter);
                                    crate::server::send_npc_dialogue_to_player(&s, &id_a, &id_b, &banter);
                                }
                            }
                            break;
                        }
                    } else {
                        let v_id = vendor_npcs[v_idx].clone();
                        if let Some((v_ent, _)) = npc_snapshots.get(v_id.as_str()) {
                            let customer_id = active_npcs.iter().find(|id| !vendor_npcs.contains(id)).cloned();
                            let c_ent = customer_id
                                .as_ref()
                                .and_then(|c_id| npc_snapshots.get(c_id.as_str()))
                                .map(|(e, _)| e)
                                .filter(|e| {
                                    !e.current_activity.contains("安歇")
                                        && !e.current_activity.contains("熟睡")
                                        && !e.current_activity.contains("熄燈")
                                        && !e.current_activity.contains("就寢")
                                });
                            if let (Some(c_id), Some(c_ent)) = (customer_id, c_ent) {
                                let purchase_outcome = crate::server::action::trade::settle_live_vendor_npc_purchase(
                                    &v_id,
                                    &c_id,
                                    room_id,
                                    now_ts,
                                );
                                match purchase_outcome {
                                    Some(crate::server::action::trade::VendorNpcPurchaseOutcome::Purchased {
                                        item_name,
                                        price_mg,
                                        remaining_stock: _,
                                        ..
                                    }) => {
                                        for s in sessions.all_sessions() {
                                            if crate::server::session::player_location_key(&s.player_id) == want {
                                                let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                                let c_name = resolve_caller_name(&s.player_id, &c_id, c_ent);
                                                let bark = get_vendor_bark(&v_id, &v_name);
                                                let is_raw_or_craft = [
                                                    "芹菜", "鮮菜", "河蝦", "文蛤", "鮮魚", "青蟹",
                                                    "牡蠣", "乾貨", "魚乾", "粗鹽", "海帶", "紫菜",
                                                    "茶葉", "茶磚", "竹筍", "野蕈", "山藥", "鮮果",
                                                    "皮掌", "鞋底", "摺疊傘", "冰鎮", "仙草凍",
                                                    "涼茶", "冷泡",
                                                ]
                                                .iter()
                                                .any(|kw| item_name.contains(kw));
                                                let pattern_idx = (now_ts as usize + v_id.len()) % 5;
                                                let interaction = if is_raw_or_craft {
                                                    match pattern_idx % 3 {
                                                        0 => format!("{bark} 身旁的{c_name}行色匆匆趕路經過，停下腳步喊道：「東家，幫我快手包一份【{item_name}】帶走！」說著遞上 {price_mg} 鎂。 {v_name}手腳利索包妥遞過，笑道：「好咧收您 {price_mg} 鎂！細心挑揀好的成色最地道，路上慢走啊！」"),
                                                        1 => format!("{bark} 身旁的{c_name}駐足端詳片刻，溫言道：「東家，家裡最認你這攤的【{item_name}】，幫我備一份帶回去！」說罷取出 {price_mg} 鎂付帳。 {v_name}格外細心地包妥，樂呵呵道：「識貨！這批【{item_name}】品質上乘，收您 {price_mg} 鎂，慢走啊！」"),
                                                        _ => format!("{bark} 身旁的{c_name}迎面熟絡地點頭招呼：「頭家，老樣子來一份【{item_name}】！」隨手將 {price_mg} 鎂整齊排在攤前。 {v_name}會心一笑，手腳極快地備齊遞上：「熟客就是內行！早幫您留著好貨呢，收您 {price_mg} 鎂，回頭再來！」"),
                                                    }
                                                } else {
                                                    match pattern_idx {
                                                        0 => format!("{bark} 身旁的{c_name}行色匆匆趕路經過，停下腳步喊道：「東家，手頭趕工時間緊，幫我快手包一份【{item_name}】！」說著遞上 {price_mg} 鎂。 {v_name}手腳極快包好遞過，笑道：「好咧收您 {price_mg} 鎂！路上慢走趁熱吃，幹工順遂啊！」"),
                                                        1 => format!("{bark} 身旁的{c_name}深吸一口氣、聞香靠攏過來笑道：「遠遠就聞到香味，實在忍不住嘴饞！東家，來一份【{item_name}】嚐嚐！」隨即掏出 {price_mg} 鎂遞過去。 {v_name}笑臉盈盈盛妥遞上：「識貨！熱騰騰起鍋的滋味最地道，收您 {price_mg} 鎂，慢用啊！」"),
                                                        2 => format!("{bark} 身旁的{c_name}駐足端詳片刻，溫言道：「東家，家裡長輩最念叨你這手藝，幫我包一份【{item_name}】帶回去給老人家嚐鮮！」說罷取出 {price_mg} 鎂付帳。 {v_name}格外細心地用厚紙包妥，樂呵呵道：「孝順！給長輩帶的特別挑了份量足的，收您 {price_mg} 鎂，替我向老人家問好！」"),
                                                        3 => format!("{bark} 身旁的{c_name}拍了拍腰間沉甸甸的工錢袋，朗聲笑道：「辛苦奔波半天，領了工錢，非得來一份【{item_name}】犒勞犒勞自己不可！」利落地數出 {price_mg} 鎂遞上。 {v_name}連忙接過笑道：「踏實幹活就該吃點好的！收您 {price_mg} 鎂，吃飽喝足渾身都是力氣！」"),
                                                        _ => format!("{bark} 身旁的{c_name}迎面熟絡地點頭招呼：「頭家，老樣子一份【{item_name}】，口味你知道的！」隨手將 {price_mg} 鎂整齊排在攤前。 {v_name}會心一笑，手腳極快地備齊遞上：「熟客就是懂吃！早幫您留心著呢，收您 {price_mg} 鎂，回頭再來坐啊！」"),
                                                    }
                                                };
                                                crate::store::record_active_npc_dialogue(room_id, &v_id, &c_id, &interaction);
                                                crate::server::send_npc_dialogue_to_player(&s, &v_id, &c_id, &interaction);
                                            }
                                        }
                                        break;
                                    }
                                    Some(crate::server::action::trade::VendorNpcPurchaseOutcome::OutOfStock {
                                        item_name,
                                    }) => {
                                        for s in sessions.all_sessions() {
                                            if crate::server::session::player_location_key(&s.player_id) == want {
                                                let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                                let c_name = resolve_caller_name(&s.player_id, &c_id, c_ent);
                                                let interaction = format!("一旁的{c_name}駐足問道：「東家，想買一份【{item_name}】！」 {v_name}看了一眼見底的鍋具，抱歉笑道：「真不巧，最後一份【{item_name}】賣完啦，我得趕緊去補料，勞駕稍候再來！」");
                                                crate::store::record_active_npc_dialogue(room_id, &v_id, &c_id, &interaction);
                                                crate::server::send_npc_dialogue_to_player(&s, &v_id, &c_id, &interaction);
                                            }
                                        }
                                        break;
                                    }
                                    Some(crate::server::action::trade::VendorNpcPurchaseOutcome::InsufficientFunds {
                                        item_name,
                                        price_mg,
                                        ..
                                    }) => {
                                        for s in sessions.all_sessions() {
                                            if crate::server::session::player_location_key(&s.player_id) == want {
                                                let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                                let c_name = resolve_caller_name(&s.player_id, &c_id, c_ent);
                                                let bark = get_vendor_bark(&v_id, &v_name);
                                                let interaction = format!("{bark} 一旁的{c_name}聞香駐足，摸了摸錢袋嘆道：「【{item_name}】一份要 {price_mg} 鎂，身上零錢不夠，等領了工錢再來捧場！」");
                                                crate::store::record_active_npc_dialogue(room_id, &v_id, &c_id, &interaction);
                                                crate::server::send_npc_dialogue_to_player(&s, &v_id, &c_id, &interaction);
                                            }
                                        }
                                        break;
                                    }
                                    None => {
                                        for s in sessions.all_sessions() {
                                            if crate::server::session::player_location_key(&s.player_id) == want {
                                                let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                                let c_name = resolve_caller_name(&s.player_id, &c_id, c_ent);
                                                let bark = get_vendor_bark(&v_id, &v_name);
                                                let idle_idx = (now_ts as usize + v_id.len()) % 2;
                                                let line = match idle_idx {
                                                    0 => format!("{bark} 一旁的{c_name}在攤前停下打量，好奇問道：「東家，今天這料看著挺新鮮，火候正好呢！」 {v_name}笑著抬頭應道：「那是自然，天天現備的好料，您先瞧瞧，待會想吃隨時吩咐！」"),
                                                    _ => format!("{bark} 一旁的{c_name}駐足探頭望著攤車，客氣問道：「頭家，今天生意挺熱絡啊？」 {v_name}擦了擦汗爽朗笑道：「承蒙街坊照顧！您先慢慢看，有合意的儘管開口！」"),
                                                };
                                                crate::store::record_active_npc_dialogue(room_id, &v_id, &c_id, &line);
                                                crate::server::send_npc_dialogue_to_player(&s, &v_id, &c_id, &line);
                                            }
                                        }
                                        break;
                                    }
                                }
                            } else {
                                for s in sessions.all_sessions() {
                                    if crate::server::session::player_location_key(&s.player_id) == want {
                                        let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                        let bark = get_vendor_bark(&v_id, &v_name);
                                        crate::server::send_narrate_to_player(&s, &bark);
                                    }
                                }
                                break;
                            }
                        }
                    }
                }

                // 若無攤販營業，且 >= 2 名清醒 NPC：輸出帶引號真實雙人對白
                let awake_npcs: Vec<String> = active_npcs
                    .iter()
                    .filter(|id| {
                        if crate::store::is_npc_paused_for_interaction(id) {
                            return false;
                        }
                        if let Some((_e, act)) = npc_snapshots.get(id.as_str()) {
                            !act.contains("安歇")
                                && !act.contains("熄燈")
                                && !act.contains("熟睡")
                                && !act.contains("就寢")
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();

                static IDLE_CHAT_SEQ: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                let can_chat = {
                    let lock = ROOM_IDLE_CHAT_LAST_TS.read().unwrap();
                    let last = lock.get(room_id).copied().unwrap_or(0);
                    now_ts.saturating_sub(last) >= 45
                };
                if !can_chat {
                    break;
                }
                ROOM_IDLE_CHAT_LAST_TS.write().unwrap().insert(room_id.to_string(), now_ts);

                if awake_npcs.len() >= 2 {
                    let id_a = &awake_npcs[0];
                    let id_b = &awake_npcs[1];
                    let pair_entities = npc_snapshots.get(id_a.as_str()).zip(npc_snapshots.get(id_b.as_str()));
                    if let Some(((ea, _), (eb, _))) = pair_entities {
                        let ecology_tick = ((now_ts as usize) / 45) + id_a.len();
                        let mut exchange_action: Option<String> = None;
                        if ecology_tick % 2 == 0 {
                            if room_name.contains("石磨") || room_name.contains("水輪") || room_name.contains("磨坊") {
                                let (item_id, item_name) = match ecology_tick % 3 {
                                    0 => ("wt_cat_0388", "現磨溫米漿"),
                                    1 => ("wt_cat_0279", "特級在來米粉"),
                                    _ => ("wt-good-009", "精碾白米"),
                                };
                                let _ = crate::db::add_to_inventory(id_b, item_id, 1);
                                exchange_action = Some(format!("遞過一袋新過篩封妥的{}", item_name));
                            } else if room_name.contains("鐵坊") || room_name.contains("鍛造") || room_name.contains("木作") || room_name.contains("工坊") {
                                let (item_id, item_name) = match ecology_tick % 3 {
                                    0 => ("mat_repair_strip", "鉚補鐵皮"),
                                    1 => ("mat_hemp_rope_roll", "麻繩卷"),
                                    _ => ("mat_iron_scrap", "碎鐵片"),
                                };
                                let _ = crate::db::add_to_inventory(id_b, item_id, 1);
                                exchange_action = Some(format!("遞過一包清點妥當的{}", item_name));
                            } else if room_name.contains("診所") || room_name.contains("藥房") || room_name.contains("烘藥") || room_name.contains("脈絡") {
                                let _ = crate::db::add_to_inventory(id_b, "craft_meridian_salve", 1);
                                exchange_action = Some("將一罐封好的百草敷料交到對方手裡".to_string());
                            } else if room_name.contains("蒙學") || room_name.contains("學院") || room_name.contains("書局") || room_name.contains("講堂") || room_name.contains("藏書") || room_name.contains("印行") {
                                let _ = crate::db::add_to_inventory(id_b, "mat_coarse_paper_ream", 1);
                                exchange_action = Some("將一刀裁好的粗棉紙交給對方收納".to_string());
                            } else if room_name.contains("商行") || room_name.contains("市集") || room_name.contains("鋪") || room_name.contains("店") || room_name.contains("帳房") {
                                let _ = crate::db::add_to_inventory(id_b, "mat_sackcloth_bolt", 1);
                                crate::server::action::trade::restock_shop_items_for_seller(id_a);
                                crate::server::action::trade::restock_shop_items_for_seller(id_b);
                                exchange_action = Some("核對完貨架並整齊補充了一批新到商品".to_string());
                            }
                        }

                        for s in sessions.all_sessions() {
                            if crate::server::session::player_location_key(&s.player_id) == want {
                                let caller_a = resolve_caller_name(&s.player_id, id_a, ea);
                                let caller_b = resolve_caller_name(&s.player_id, id_b, eb);

                                let seq_idx = IDLE_CHAT_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                let project_npc_utterance = |npc_id: &str, ent: &crate::store::Entity, is_reply: bool, seq: usize, avoid_th: Option<&str>, avoid_ob: Option<&str>| -> (String, Option<String>, Option<String>) {
                                    let raw_ob_opt = crate::store::get_active_npc_obligation(npc_id)
                                        .and_then(|(kind, subj)| match kind.as_str() {
                                            "restock_goods" => Some("鋪子補貨".to_string()),
                                            "repair_tool" => Some("修整家什".to_string()),
                                            "stranded_cargo" => Some("貨物調度".to_string()),
                                            "repay_debt" => Some("手頭帳務結算".to_string()),
                                            "treat_injury" => Some("調養傷勢".to_string()),
                                            _ if !subj.trim().is_empty() && !subj.is_ascii() => {
                                                let clean = subj.trim().trim_end_matches('。');
                                                if clean.chars().count() > 14 {
                                                    if clean.contains("採辦") || clean.contains("採買") {
                                                        Some("坊間採辦".to_string())
                                                    } else if clean.contains("修整") || clean.contains("打磨") {
                                                        Some("修整隨身器物".to_string())
                                                    } else if clean.contains("家務") || clean.contains("收拾") {
                                                        Some("家中瑣務".to_string())
                                                    } else {
                                                        Some("手頭要務".to_string())
                                                    }
                                                } else {
                                                    Some(clean.to_string())
                                                }
                                            }
                                            _ => None,
                                        });
                                    let ob_opt = raw_ob_opt.filter(|ob| {
                                        avoid_ob.map_or(true, |av| av != ob.as_str())
                                    });
                                    let thought_opt = ent
                                        .thoughts
                                        .iter()
                                        .rev()
                                        .filter_map(|t| t.flavor.as_deref())
                                        .map(|f| sanitize_thought_string(f, &ent.display_title, seq as u64))
                                        .map(|f| {
                                            let mut s = f.trim().trim_end_matches('。').replace("文曜・", "");
                                            if s.starts_with('在') {
                                                let marks = ["飲過", "用過", "歇過"];
                                                for mark in marks {
                                                    if let Some(idx) = s.find(mark) {
                                                        let before = &s[..idx];
                                                        if ["居", "宅", "舍", "宿", "值房"].iter().any(|k| before.contains(k)) {
                                                            s = format!("在家裡{}", &s[idx..]);
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                            s
                                        })
                                        .find(|ft| {
                                            let clean = ft.trim();
                                            let same_as_avoid = avoid_th.map_or(false, |av| {
                                                clean == av || (clean.chars().count() >= 6 && av.chars().count() >= 6 && clean.chars().take(6).eq(av.chars().take(6)))
                                            });
                                            !clean.is_empty()
                                                && !same_as_avoid
                                                && !clean.contains("發愁")
                                                && !clean.contains("枚鎂幣")
                                                && !clean.contains("結清帳目")
                                                && !clean.contains(concat!("跟人說了會", "兒話"))
                                                && !clean.contains("過後，")
                                                && !clean.contains("舒緩了許多")
                                                && !clean.contains("神清氣爽")
                                                && !clean.contains("完成「")
                                                && !clean.contains('「')
                                        });
                                    let raw_act = ent
                                        .current_activity
                                        .trim()
                                        .trim_end_matches('。')
                                        .trim_start_matches("正在")
                                        .trim_start_matches('正')
                                        .trim();
                                    let act_clean = if raw_act.is_empty() {
                                        String::new()
                                    } else {
                                        let is_outdoor = ["橋", "街", "道", "口", "坪", "廣場", "碼頭"].iter().any(|k| room_name.contains(k));
                                        let has_indoor_rest = ["安歇", "入眠", "酣眠", "木榻", "床鋪", "內室", "單間", "浴池", "堂屋"].iter().any(|k| raw_act.contains(k));
                                        let has_transit = raw_act.contains("動身前往")
                                            || raw_act.contains("前往【")
                                            || raw_act.contains("返回【")
                                            || raw_act.contains("趕往")
                                            || raw_act.contains('【');
                                        let concise = if is_outdoor && has_indoor_rest {
                                            "在街邊歇腳喘口氣"
                                        } else if has_transit {
                                            "沿街趕路辦差"
                                        } else if raw_act.chars().count() > 14 {
                                            if raw_act.contains("話家常") || raw_act.contains("堂屋") || raw_act.contains("院落") {
                                                "在院落料理家務"
                                            } else if raw_act.contains("打磨") || raw_act.contains("工位") || raw_act.contains("工坊") {
                                                "在工坊打理活計"
                                            } else if raw_act.contains("診所") || raw_act.contains("藥") {
                                                "在診所抓藥調理"
                                            } else if raw_act.contains("食堂") || raw_act.contains("麵") || raw_act.contains("茶") {
                                                "在茶樓用膳歇腳"
                                            } else if raw_act.contains("市集") || raw_act.contains("採買") || raw_act.contains("採辦") {
                                                "在市集採辦日用"
                                            } else if raw_act.contains("碼頭") || raw_act.contains("船") {
                                                "在碼頭檢整舟楫"
                                            } else if raw_act.contains("巡") || raw_act.contains("守") {
                                                "在各處哨位巡查"
                                            } else {
                                                "按部就班打理差事"
                                            }
                                        } else {
                                            raw_act
                                        };
                                        if concise.starts_with('在')
                                            || concise.starts_with('往')
                                            || concise.starts_with('去')
                                            || concise.starts_with('到')
                                            || concise.starts_with('回')
                                            || concise.starts_with("於")
                                            || concise.starts_with("巡")
                                            || concise.starts_with("守")
                                            || concise.starts_with("看")
                                            || concise.starts_with("整理")
                                            || concise.starts_with("盤點")
                                            || concise.starts_with("打理")
                                            || concise.starts_with("核對")
                                            || concise.starts_with("挑選")
                                            || concise.starts_with("修")
                                            || concise.starts_with("搬")
                                            || concise.starts_with("順道")
                                            || concise.starts_with("趁")
                                        {
                                            concise.to_string()
                                        } else {
                                            format!("在{concise}")
                                        }
                                    };

                                    let act_bare = act_clean.trim_start_matches('在');

                                    let line = match (ob_opt.as_deref(), thought_opt.as_deref(), act_clean.is_empty()) {
                                        (Some(ob), Some(th), false) => {
                                            if !is_reply {
                                                match seq % 3 {
                                                    0 => format!("「{}，手頭還記掛著{}的事，眼下{}。」", th, ob, act_clean),
                                                    1 => format!("「眼下{}，心裡倒還想著{}那樁差事。」", act_clean, ob),
                                                    _ => format!("「先把{}的事處置妥當要緊，{}。」", ob, th),
                                                }
                                            } else {
                                                match seq % 3 {
                                                    0 => format!("「可不是嗎，{}，我也正{}，好把{}的事辦妥。」", th, act_bare, ob),
                                                    1 => format!("「誰說不是呢，我手裡也壓著{}的差事，趁現在{}最穩妥。」", ob, act_bare),
                                                    _ => format!("「彼此彼此，{}，等忙完{}再歇口氣。」", th, ob),
                                                }
                                            }
                                        }
                                        (Some(ob), _, false) => {
                                            if !is_reply {
                                                format!("「手頭還惦記著{}的事，趁著眼下{}。」", ob, act_bare)
                                            } else {
                                                format!("「我這頭也記掛著{}的活計，眼下正{}呢。」", ob, act_bare)
                                            }
                                        }
                                        (None, Some(th), false) => {
                                            if !is_reply {
                                                format!("「{}，趁著精神還好，{}最是舒坦。」", th, act_bare)
                                            } else {
                                                format!("「說的是，{}，我也正{}呢。」", th, act_bare)
                                            }
                                        }
                                        (_, Some(th), true) => {
                                            if !is_reply {
                                                format!("「{}，總算能鬆口氣。」", th)
                                            } else {
                                                format!("「是啊，{}，按部就班最踏實。」", th)
                                            }
                                        }
                                        _ => {
                                            let act_str = if act_clean.is_empty() { "把眼下手頭的差事理順" } else { act_clean.as_str() };
                                            let act_fallback_bare = if act_clean.is_empty() { "按部就班理順差事" } else { act_bare };
                                            if !is_reply {
                                                format!("「忙完手頭的活計，{}最是舒心。」", act_str)
                                            } else {
                                                format!("「可不是嘛，我也正{}，忙妥了心裡踏實。」", act_fallback_bare)
                                            }
                                        }
                                    };
                                    (line, thought_opt, ob_opt)
                                };
                                let dialogue_narrate = if ecology_tick % 3 == 0 {
                                    if let Some((_pack_id, _step, script_line)) = crate::game::peninsula_scripts::co_enact_script_pack(&room_name, &caller_a, &caller_b, now_ts) {
                                        Some(script_line)
                                    } else if let Some(venue) = crate::grid::resolve_any_room_venue_object(&room_name, "") {
                                        if !room_name.contains("石磨") && !room_name.contains("鐵坊") && !room_name.contains("診所") && !room_name.contains("蒙學") && !room_name.contains("商行") {
                                            Some(format!("【{}・當值協作】{caller_a}（{}）與{caller_b}在{}默契配合，一邊巡檢案頭器物，一邊核對手頭交割的活計。", venue.venue_kind, venue.on_duty_fixture_activity, venue.venue_kind))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }.unwrap_or_else(|| {
                                    let (dlg_a, used_th_a, used_ob_a) = project_npc_utterance(id_a, ea, false, seq_idx, None, None);
                                    let (dlg_b, _, _) = project_npc_utterance(id_b, eb, true, seq_idx.wrapping_add(1), used_th_a.as_deref(), used_ob_a.as_deref());

                                    let action_prefix = if let Some(act) = &exchange_action {
                                        if act.contains("完") || act.contains("了一") {
                                            format!("{caller_a}在{act}後，")
                                        } else {
                                            format!("{caller_a}邊{act}，邊")
                                        }
                                    } else {
                                        caller_a.clone()
                                    };
                                    match seq_idx % 6 {
                                        0 => format!("{action_prefix}側身向{caller_b}言道：{dlg_a}\n{caller_b}微微點頭，隨口應道：{dlg_b}"),
                                        1 => format!("{action_prefix}將手頭物事稍作整理，對{caller_b}開口：{dlg_a}\n{caller_b}抬眼笑應：{dlg_b}"),
                                        2 => format!("{action_prefix}望向四下市面，同{caller_b}搭話：{dlg_a}\n{caller_b}順著話頭接道：{dlg_b}"),
                                        3 => format!("{action_prefix}朝{caller_b}低聲說道：{dlg_a}\n{caller_b}神情專注，答道：{dlg_b}"),
                                        4 => format!("{action_prefix}手頭活計未停，對{caller_b}言道：{dlg_a}\n{caller_b}語調平穩地回道：{dlg_b}"),
                                        _ => format!("{action_prefix}向{caller_b}問道：{dlg_a}\n{caller_b}略一思索，溫聲回道：{dlg_b}"),
                                    }
                                });
                                crate::store::record_active_npc_dialogue(room_id, id_a, id_b, &dialogue_narrate);
                                crate::server::send_npc_dialogue_to_player(&s, id_a, id_b, &dialogue_narrate);
                            }
                        }
                        break;
                    }
                } else if active_npcs.len() >= 2 {
                    // 全在安歇
                    let id_a = &active_npcs[0];
                    let id_b = &active_npcs[1];
                    if let (Some((ea, _)), Some((eb, _))) = (npc_snapshots.get(id_a.as_str()), npc_snapshots.get(id_b.as_str())) {
                        for s in sessions.all_sessions() {
                            if crate::server::session::player_location_key(&s.player_id) == want {
                                let caller_a = resolve_caller_name(&s.player_id, id_a, ea);
                                let caller_b = resolve_caller_name(&s.player_id, id_b, eb);
                                let two_npc_narrate = format!("屋內燈火微茫，{}與{}皆已歇息，四下唯聞平穩均勻的呼吸聲與遠處夜蟲微鳴。", caller_a, caller_b);
                                crate::server::send_narrate_to_player(&s, &two_npc_narrate);
                            }
                        }
                        break;
                    }
                } else if let Some(npc_id) = active_npcs.first() {
                    if crate::store::is_npc_paused_for_interaction(npc_id) {
                        break;
                    }
                    if let Some((ent, raw_act)) = npc_snapshots.get(npc_id.as_str()) {
                        let activity = raw_act.trim();
                        let seq_idx = IDLE_CHAT_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        for s in sessions.all_sessions() {
                            if crate::server::session::player_location_key(&s.player_id) == want {
                                let caller = resolve_caller_name(&s.player_id, npc_id, ent);

                                let live_detail = if is_deep_night
                                    && (activity.contains("安歇")
                                        || activity.contains("熄燈")
                                        || activity.contains("熟睡")
                                        || activity.contains("就寢"))
                                {
                                    format!("屋內燈火已熄，{}已在床鋪上安歇，四周寧靜無聲。", caller)
                                } else if !activity.is_empty() {
                                    let act_trimmed = activity.trim_end_matches(['。', '，', ' ']);
                                    let clean_raw = act_trimmed
                                        .trim_start_matches("正在")
                                        .trim_start_matches('正')
                                        .trim();
                                    let clean = if clean_raw.chars().count() > 16 {
                                        if clean_raw.contains("話家常") || clean_raw.contains("堂屋") || clean_raw.contains("院落") {
                                            "在家中料理家常手作雜務"
                                        } else if clean_raw.contains("打磨") || clean_raw.contains("工位") || clean_raw.contains("工坊") {
                                            "在工位打理手頭活計"
                                        } else if clean_raw.contains("市集") || clean_raw.contains("採買") || clean_raw.contains("採辦") {
                                            "在市集採辦日常用度"
                                        } else if clean_raw.contains("診所") || clean_raw.contains("藥") {
                                            "在診所抓藥調理"
                                        } else {
                                            "按部就班打理差事"
                                        }
                                    } else {
                                        clean_raw
                                    };
                                    let ecology_tick = ((now_ts as usize) / 45) + npc_id.len();
                                    let mutter: String = if clean_raw.contains("巡") || clean_raw.contains("守") {
                                        let lines = [
                                            "自語道：「四下平靜，各處燈火安穩。」",
                                            "抬眼望了望橋下水勢，低聲道：「夜裡水色平緩，沿岸船隻都栓牢了。」",
                                            "撥亮了手頭的防風提燈，輕聲自語：「再繞過前面街口巡一圈，差不多就到換班鐘點了。」",
                                        ];
                                        lines[seq_idx % lines.len()].to_string()
                                    } else if clean.contains("帳") || clean.contains("算") || clean.contains("冊") || clean.contains("批改") {
                                        let lines = [
                                            "低聲沉吟：「這筆紀錄核對清楚，分毫不差。」",
                                            "提筆在冊頁旁添了幾筆批註，點頭道：「這樣整理出來，明日查閱就一目了然了。」",
                                            "合上手頭的卷冊舒了口氣：「這幾頁總算批閱完畢，喝口溫茶歇一歇。」",
                                        ];
                                        lines[seq_idx % lines.len()].to_string()
                                    } else if clean.contains("火") || clean.contains("爐") || clean.contains("鍛") {
                                        let lines = [
                                            "擦了擦汗笑道：「這爐火候正旺，淬火火候恰到好處！」",
                                            "端詳著手裡的鐵件點頭：「器口平整厚實，這批成色真不錯。」",
                                            "添了一鏟耐火炭，自語道：「趁著爐溫勻稱，把這幾件活計一鼓作氣打完。」",
                                        ];
                                        lines[seq_idx % lines.len()].to_string()
                                    } else if clean.contains("藥") || clean.contains("焙") {
                                        let lines = [
                                            "聞了聞藥香自語道：「百草烘焙得極勻，藥氣純正。」",
                                            "仔細封妥陶罐木塞，低聲道：「防潮封口做紮實些，藥效才能留得長久。」",
                                        ];
                                        lines[seq_idx % lines.len()].to_string()
                                    } else if let Some((ep, _sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(&room_name, Some(&caller), now_ts) {
                                        format!("{caller}一邊忙碌一邊低聲道：「{}」", ep.trigger_and_causal_chain.replace("{當值主事}", &caller).replace("{協力對口}", "同僚"))
                                    } else if let Some(venue) = crate::grid::resolve_any_room_venue_object(&room_name, "")
                                        && !venue.on_duty_talk_lines.is_empty() {
                                        let line = venue.on_duty_talk_lines[(ecology_tick as usize) % venue.on_duty_talk_lines.len()];
                                        format!("{caller}（{}）隨口道：「{line}」", venue.on_duty_fixture_activity)
                                    } else {
                                        let lines = [
                                            "輕聲笑道：「手頭這樁事俐落辦完，稍後去街上走走。」",
                                            "整理了一下衣袖，自語道：「按部就班把事情打理妥當，心裡也踏實。」",
                                            "抬頭看了看天色，低聲道：「今天天氣清爽，做事也格外順手。」",
                                        ];
                                        lines[seq_idx % lines.len()].to_string()
                                    };
                                    if mutter.starts_with(&caller) {
                                        format!("一旁{}", mutter)
                                    } else if clean.contains("前往")
                                        || clean.contains("趕往")
                                        || clean.contains("返回")
                                        || clean.contains('【')
                                    {
                                        format!("一旁{}，{}", caller, mutter)
                                    } else {
                                        format!("一旁{}正{}，{}", caller, clean, mutter)
                                    }
                                } else {
                                    format!("一旁{}整理了一下手邊的器物，神情從容。", caller)
                                };
                                crate::server::send_narrate_to_player(&s, &live_detail);
                            }
                        }
                        break;
                    }
                }
            }

            // 若在場無清醒 NPC，才退回環境微互動
            let line = pick_micro_interaction(room_id, micro_pct, hour);
            if !line.is_empty() {
                send_narrate_to_room(sessions, room_id, &line);
                break;
            }
        }

        // 離線無玩家場所之自主百工與物料生態演化步調
        if !is_deep_night {
            let offscreen_rooms: Vec<(String, Vec<String>)> = if let Some(store) = crate::store::get_store() {
                let st = store.read();
                let mut room_map: HashMap<String, Vec<String>> = HashMap::new();
                for e in st.entities.values() {
                    if e.kind == "npc" && e.vit > 0 {
                        let rid = st.get_entity_room(&e.id);
                        if !rid.is_empty() && !player_room_set.contains(&rid) {
                            room_map.entry(rid).or_default().push(e.id.clone());
                        }
                    }
                }
                room_map
                    .into_iter()
                    .filter(|(_, npcs)| !npcs.is_empty())
                    .collect()
            } else {
                Vec::new()
            };

            if !offscreen_rooms.is_empty() {
                let now_ts = crate::game::now_unix();
                let mut eligible: Vec<(String, Vec<String>)> = offscreen_rooms
                    .into_iter()
                    .filter(|(rid, _)| {
                        let lock = ROOM_IDLE_CHAT_LAST_TS.read().unwrap();
                        let last = lock.get(rid).copied().unwrap_or(0);
                        now_ts.saturating_sub(last) >= 35
                    })
                    .collect();

                let count = eligible.len().min(2);
                for _ in 0..count {
                    if eligible.is_empty() {
                        break;
                    }
                    let idx = rng.random_range(0..eligible.len());
                    let (off_room_id, off_active_npcs) = eligible.swap_remove(idx);
                    let off_room_name = if let Some(coord) = crate::grid::parse_grid_room_id(&off_room_id)
                        && let Some(cell) = crate::server::grid_manager::get_runtime_grid_cell(coord)
                    {
                        if cell.name.trim().is_empty() {
                            crate::grid::terrain_name_zh(cell.terrain).to_string()
                        } else {
                            cell.name
                        }
                    } else {
                        get_room_name(&off_room_id).unwrap_or_default()
                    };
                    if crate::game::object_ecology::run_npc_autonomous_production_step(
                        &off_room_id,
                        &off_room_name,
                        &off_active_npcs,
                        now_ts,
                    )
                    .is_some()
                    {
                        ROOM_IDLE_CHAT_LAST_TS
                            .write()
                            .unwrap()
                            .insert(off_room_id, now_ts);
                    }
                }
            }
        }
    }
    let Ok(schedules) = get_all_schedules() else {
        return;
    };
    let rf = gametext::runtime_fmt();
    let mut wmax = sim().wander_roll_max;
    if wmax <= 0 {
        wmax = 10;
    }
    let mut rng = rand::rng();
    for sch in schedules {
        if !sch.is_on_duty(hour) || hour >= 22 || hour < 6 {
            continue;
        }
        if crate::db::get_entity_place_id(&sch.entity_id).is_some_and(|place_id| {
            crate::store::get_store().is_some_and(|store| {
                store
                    .read()
                    .get_place(&place_id)
                    .is_some_and(|place| place.source_kind == "azgaar_runtime")
            })
        }) {
            continue;
        }
        if crate::store::is_npc_paused_for_interaction(&sch.entity_id) {
            continue;
        }
        let Ok(npc_room) = get_entity_room(&sch.entity_id) else {
            continue;
        };
        let mut title = get_npc_title_from_assignments(&sch.entity_id);
        if title.is_empty() {
            title = gametext::default_manager_title();
        }
        let person = get_npc_person_display_name(&sch.entity_id).unwrap_or_default();
        let wander_rooms = get_wander_rooms(&title);
        if wander_rooms.len() > 1 && rng.random_range(0..wmax) == 0 {
            let npc_key = canonical_location_key(&npc_room);
            let candidates: Vec<String> = wander_rooms
                .into_iter()
                .filter(|wr| canonical_location_key(wr) != npc_key)
                .collect();
            if !candidates.is_empty() {
                let dest = candidates[rng.random_range(0..candidates.len())].clone();
                let dest_name = get_room_name(&dest).unwrap_or_else(|_| String::new());
                let src_name = get_room_name(&npc_room).unwrap_or_else(|_| String::new());
                let want_src = canonical_location_key(&npc_room);
                for s in sessions.all_sessions() {
                    if crate::server::session::player_location_key(&s.player_id) == want_src {
                        let viewer_person = resolve_ambient_caller_label(&s.player_id, &sch.entity_id);
                        let leave_text = get_wander_flavor(&title, &viewer_person, &dest_name, true);
                        if !leave_text.is_empty() {
                            crate::server::broadcast::send_narrate_to_player(&s, &leave_text);
                        }
                    }
                }
                if set_entity_room(&sch.entity_id, &dest).is_ok() {
                    let want_dest = canonical_location_key(&dest);
                    for s in sessions.all_sessions() {
                        if crate::server::session::player_location_key(&s.player_id) == want_dest {
                            let viewer_person = resolve_ambient_caller_label(&s.player_id, &sch.entity_id);
                            let arrive_text = get_wander_flavor(&title, &viewer_person, &src_name, false);
                            if !arrive_text.is_empty() {
                                crate::server::broadcast::send_narrate_to_player(&s, &arrive_text);
                            }
                        }
                    }
                }
                push_room_event(
                    &npc_room,
                    "leave",
                    &person,
                    &sprintf_s(&rf.push_leave_fmt, &[dest_name.as_str()]),
                );
                push_room_event(
                    &dest,
                    "arrive",
                    &person,
                    &sprintf_s(&rf.push_arrive_fmt, &[src_name.as_str()]),
                );
                refresh_room_views_for_room(sessions, cfg, &npc_room);
                refresh_room_views_for_room(sessions, cfg, &dest);
                continue;
            }
        }
        if !player_room_set.contains(&canonical_location_key(&npc_room)) {
            continue;
        }
        let disp = get_disposition(&sch.entity_id);
        let want_room = canonical_location_key(&npc_room);
        let mut sent_any = false;
        for s in sessions.all_sessions() {
            if crate::server::session::player_location_key(&s.player_id) == want_room {
                let viewer_person = resolve_ambient_caller_label(&s.player_id, &sch.entity_id);
                let emote = pick_idle_emote(&title, &period, &viewer_person, disp);
                if !emote.is_empty() {
                    crate::server::broadcast::send_narrate_to_player(&s, &emote);
                    sent_any = true;
                }
            }
        }
        if sent_any {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vendor_banter_zero_hardcoded_personal_names() {
        let pairs = [
            ("roast_beef", "noodle_soup"),
            ("pepper_bun", "tofu_pudding"),
            ("herbal_tea", "roast_beef"),
            ("herbal_tea", "pepper_bun"),
            ("tofu_pudding", "herbal_tea"),
            ("rice_roll", "soy_milk"),
            ("morning_produce", "morning_fish"),
            ("other_vendor_1", "other_vendor_2"),
        ];
        let forbidden_names = [
            "罔市", "火旺", "順興伯", "阿發", "秋月", "文貴", "秀琴", "海生", "春生",
        ];
        for (a, b) in pairs {
            for hour in [8, 14, 20] {
                let banter = generate_vendor_banter("甲掌事", "乙主事", a, b, hour);
                for name in forbidden_names {
                    assert!(!banter.contains(name), "vendor banter for ({a}, {b}) contained hardcoded personal name: {name}");
                }
            }
        }
    }

    #[test]
    fn test_resolve_caller_name_uniform_and_counter_zone_recognition() {
        let mut ent_uniform = crate::store::Entity::new_npc("test_npc_clerk", "員");
        ent_uniform.display_title = "身穿制服店員".to_string();
        let caller_uniform = resolve_caller_name("p_test", &ent_uniform.id, &ent_uniform);
        assert!(caller_uniform.contains("身著工作制服"), "expected uniform caller label, got: {caller_uniform}");

        let mut ent_counter = crate::store::Entity::new_npc("test_npc_mgr", "掌");
        ent_counter.display_title = "在場值守的當值掌事".to_string();
        let caller_counter = resolve_caller_name("p_test", &ent_counter.id, &ent_counter);
        assert!(caller_counter.contains("站在櫃檯內側"), "expected counter caller label, got: {caller_counter}");

        let mut ent_both = crate::store::Entity::new_npc("test_npc_lead", "長");
        ent_both.display_title = "身穿制服店員且在場值守的當值掌事".to_string();
        let caller_both = resolve_caller_name("p_test", &ent_both.id, &ent_both);
        assert!(caller_both.contains("身著工作制服、站在櫃檯內側"), "expected both uniform and counter label, got: {caller_both}");
    }
}

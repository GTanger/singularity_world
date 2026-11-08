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


#[allow(dead_code)]
static IDLE_CHAT_SEQ: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

include!("idle/banter.rs");
include!("idle/vendor_banter.rs");
include!("idle/patrol_and_dialogue.rs");

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
    let dialogue_done = run_npc_npc_dialogue_attempt(sessions, cfg, hour, &player_room_set);
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
                        }
                    }
                }
                if presence_reflex_done {
                    break;
                }
                if try_run_vendor_interactions_in_room(
                    sessions,
                    room_id,
                    &want,
                    now_ts,
                    hour,
                    &active_npcs,
                    &npc_snapshots,
                ) {
                    break;
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
                        let _ecology_tick = ((now_ts as usize) / 45) + id_a.len();
                        let _ = crate::game::object_ecology::run_npc_autonomous_production_step(room_id, &room_name, &awake_npcs, now_ts);
                        let exchange_action: Option<String> = None;

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
                                                    0 => format!("「{}，手頭還記掛著{}的事，現在{}。」", th, ob, act_clean),
                                                    1 => format!("「現在{}，心裡倒還想著{}那樁差事。」", act_clean, ob),
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
                                                format!("「手頭還惦記著{}的事，趁著現在{}。」", ob, act_bare)
                                            } else {
                                                format!("「我這頭也記掛著{}的活計，現在正{}呢。」", ob, act_bare)
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
                                            let act_str = if act_clean.is_empty() { "把現在手頭的差事理順" } else { act_clean.as_str() };
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
                                let dialogue_narrate = {
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
                                };
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
                                    format!("屋內燈火已熄，{}已在床鋪上熄燈就寢，四周寧靜無聲。", caller)
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
                                    } else if let Some(venue) = crate::grid::resolve_any_room_venue_object(&room_name, "")
                                        && crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(&room_name, npc_id, &ent.display_title, &activity, &ent.equipment_slots).0
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

        if !is_deep_night {
            run_offscreen_autonomous_production(&player_room_set, &mut rng);
        }
    }

    run_npc_patrol_and_wander(sessions, cfg, hour, &period, &player_room_set);
}

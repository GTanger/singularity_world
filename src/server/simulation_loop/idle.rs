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
                && let Some(cell) = crate::server::grid_manager::get_runtime_square_grid()
                    .and_then(|g| g.get(coord).cloned())
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
                || room_name.contains("教室")
                || room_name.contains("校舍")
                || room_name.contains("國小")
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

            let (npc_ids, active_npcs) = if let Some(store) = crate::store::get_store() {
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
                                    id.starts_with("wt_vendor_") || id.contains("cen_") || id.contains("ke_") || id.contains("ma_buxue") || id.contains("smith") || id.contains("carpenter")
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
                                            let reflex = format!("{caller}留意到你在旁端詳，善意地提醒道：「客友若有采集得來的草木金石原料，或想將隨身器具百工煉製、催動氣脈異變，爐前台案隨時可用。」");
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
                        format!("{caller}用扁鐵鏟從鐵桶內壁鏟起金黃焦脆的熱餅，高聲吆喝：「方出爐的酥皮鮮肉胡椒餅！老麵貼爐、皮酥蔥香肉汁滿，小心燙口喔——」")
                    } else if v_id == "wt_vendor_herbal_tea" {
                        format!("{caller}手搖清脆銅盞叮噹作響，笑著叫賣：「寒水玉冰鎮黑糖仙草凍來囉！慢熬仙草沁涼滑嫩、生津止渴，解膩消暑最對味喔！」")
                    } else if v_id == "wt_vendor_tofu_pudding" {
                        format!("{caller}手持扁銅杓片出滑嫩豆花，朗聲招呼：「古早味黑糖薑汁熱豆花——井泉鹽滷、慢火黑糖粉圓配薑汁暖胃補氣，來坐喔！」")
                    } else if v_id == "wt_vendor_rice_roll" {
                        format!("{caller}掀開大木甑散發出撲鼻荷葉香氣，清脆吆喝：「方蒸好的荷葉糯米炊飯！粒粒糯米透著竹香，吃一顆飽到中午喔！」")
                    } else if v_id == "wt_vendor_soy_milk" {
                        format!("{caller}攪拌著滾燙濃香的銅鍋，朗聲招呼：「老堰頭現磨溫米漿配現炸酥脆油條喔！石磨現磨香濃滑順，大清早喝一碗神清氣爽！」")
                    } else if v_id == "wt_vendor_morning_produce" {
                        format!("{caller}往竹簍鮮菜灑了點清泉水，笑著吆喝：「老堰頭水芹菜脆嫩莖方採下山喔！山泉灌溉、梗脆帶露清甜，隨便挑隨便選！」")
                    } else if v_id == "wt_vendor_morning_fish" {
                        format!("{caller}指著木盆裡水花直濺的鮮活河蝦，大聲叫賣：「水尾寮淺澤青殼河蝦現撈上岸！隻隻鮮活生猛、殼薄肉甜，買回去煮湯下酒最正點！」")
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
                                    let banter = if (id_a.contains("roast_beef") && id_b.contains("noodle_soup")) || (id_b.contains("roast_beef") && id_a.contains("noodle_soup")) {
                                        format!("{caller_a}抽空朝隔壁攤招呼道：「罔市姊，待會兒幫我留碗大骨湯切仔麵，加顆滷蛋！」 {caller_b}笑著應道：「好勒火旺，等這陣烤肉香氣散了，我親自端過去給你！」")
                                    } else if (id_a.contains("pepper_bun") && id_b.contains("tofu_pudding")) || (id_b.contains("pepper_bun") && id_a.contains("tofu_pudding")) {
                                        format!("{caller_a}抽空朝隔壁攤笑道：「順興伯，待會兒留碗黑糖薑汁豆花，吃完方出爐的胡椒餅配碗熱甜湯最舒坦！」 {caller_b}樂呵呵應道：「好勒阿發！我給你多舀一勺黑糖粉圓，趁熱吃啊！」")
                                    } else if (id_a.contains("herbal_tea") && id_b.contains("roast_beef")) || (id_b.contains("herbal_tea") && id_a.contains("roast_beef")) {
                                        format!("{caller_a}朝隔壁笑道：「火旺哥，烤肉串火氣大，待會兒過來喝盞寒水玉仙草凍降降火！」 {caller_b}爽朗應道：「秋月妹子想得周到！等我烤完這爐牛肉番薯，馬上過去盛一盞！」")
                                    } else if (id_a.contains("herbal_tea") && id_b.contains("pepper_bun")) || (id_b.contains("herbal_tea") && id_a.contains("pepper_bun")) {
                                        format!("{caller_a}笑著朝隔壁打趣道：「阿發哥，胡椒餅方出爐辛香撲鼻，配我這冰鎮仙草凍正是絕配呢！」 {caller_b}哈哈笑道：「秋月說得對，一個燙口酥脆一個沁涼消暑，待會兒我也來一盞！」")
                                    } else if (id_a.contains("tofu_pudding") && id_b.contains("herbal_tea")) || (id_b.contains("tofu_pudding") && id_a.contains("herbal_tea")) {
                                        format!("{caller_a}笑著打招呼道：「秋月妹子，今天仙草凍備得足呀，井泉鎮得真透涼！」 {caller_b}點頭應道：「順興伯，您的黑糖薑汁豆花也是香飄整條街呢，大家今晚生意都旺！」")
                                    } else if (id_a.contains("rice_roll") && id_b.contains("soy_milk")) || (id_b.contains("rice_roll") && id_a.contains("soy_milk")) {
                                        format!("{caller_a}隔空喊道：「文貴哥，現磨熱米漿幫我盛一碗，搭我方捏好的紫米飯糰正合適！」 {caller_b}朗聲回道：「沒問題秀琴姨，我再給你挑根最酥脆的現炸油條！」")
                                    } else if (id_a.contains("morning_produce") && id_b.contains("morning_fish")) || (id_b.contains("morning_produce") && id_a.contains("morning_fish")) {
                                        format!("{caller_a}朝隔壁笑喊：「海生哥，今天這河蝦個個活蹦亂跳啊，搭我的鮮嫩水芹菜下鍋清炒最甜了！」 {caller_b}高聲笑應：「春生老弟眼光好！待會兒收市，我們換點菜蝦回家下酒！」")
                                    } else if hour >= 18 || hour < 6 {
                                        format!("{caller_a}望向隔壁攤笑道：「今晚夜市人潮真旺，我這鍋備料都快見底啦！」 {caller_b}點頭應道：「是啊，趁著夜風涼爽、街燈亮堂，大夥兒手腳都麻利些！」")
                                    } else {
                                        format!("{caller_a}望向隔壁攤笑道：「今天生意不錯呀，街上往來買賣的人潮挺旺！」 {caller_b}點頭應道：「是啊，趁著日頭正好、市面熱鬧，大夥兒都精神！」")
                                    };
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
                                        remaining_stock,
                                        ..
                                    }) => {
                                        for s in sessions.all_sessions() {
                                            if crate::server::session::player_location_key(&s.player_id) == want {
                                                let v_name = resolve_caller_name(&s.player_id, &v_id, v_ent);
                                                let c_name = resolve_caller_name(&s.player_id, &c_id, c_ent);
                                                let bark = get_vendor_bark(&v_id, &v_name);
                                                let pattern_idx = (now_ts as usize + v_id.len()) % 5;
                                                let interaction = match pattern_idx {
                                                    0 => format!("{bark} 一旁的{c_name}行色匆匆趕路經過，停下腳步喊道：「老闆，手頭趕工時間緊，幫我俐落外帶一份【{item_name}】！」說著遞上 {price_mg} 鎂。 {v_name}麻利包好遞過，笑道：「好勒收您 {price_mg} 鎂！路上慢走趁熱吃，幹工順遂啊！」（攤上現貨尚餘 {remaining_stock} 份）"),
                                                    1 => format!("{bark} 一旁的{c_name}深吸一口氣、聞香靠攏過來笑道：「遠遠就聞到香味，實在忍不住嘴饞！老闆，來一份【{item_name}】嚐嚐！」隨即掏出 {price_mg} 鎂遞過去。 {v_name}笑臉盈盈盛妥遞上：「識貨！方起鍋的滋味最正點，收您 {price_mg} 鎂，慢用啊！」（攤上現貨尚餘 {remaining_stock} 份）"),
                                                    2 => format!("{bark} 一旁的{c_name}駐足端詳片刻，溫言道：「老闆，家裡長輩最念叨你這手藝，幫我包一份【{item_name}】帶回去給老人家嚐鮮！」說罷取出 {price_mg} 鎂付帳。 {v_name}格外細心地用厚紙包妥，樂呵呵道：「孝順！給長輩帶的特別挑了份量足的，收您 {price_mg} 鎂，替我向老人家問好！」（攤上現貨尚餘 {remaining_stock} 份）"),
                                                    3 => format!("{bark} 一旁的{c_name}拍了拍腰間方領的工錢袋，朗聲笑道：「辛苦奔波半天，方結了工錢，非得來一份【{item_name}】犒勞犒勞自己不可！」利落地數出 {price_mg} 鎂遞上。 {v_name}連忙接過笑道：「踏實幹活就該吃點好的！收您 {price_mg} 鎂，吃飽喝足渾身都是力氣！」（攤上現貨尚餘 {remaining_stock} 份）"),
                                                    _ => format!("{bark} 一旁的{c_name}迎面熟絡地點頭招呼：「頭家，老樣子一份【{item_name}】，口味你知道的！」隨手將 {price_mg} 鎂整齊排在攤前。 {v_name}會心一笑，手腳極快地備齊遞上：「熟客就是懂吃！早幫您留心著呢，收您 {price_mg} 鎂，回頭再來坐啊！」（攤上現貨尚餘 {remaining_stock} 份）"),
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
                                                let interaction = format!("一旁的{c_name}駐足問道：「老闆，想買一份【{item_name}】！」 {v_name}看了一眼見底的鍋具，抱歉笑道：「真不巧，最後一份【{item_name}】賣完啦，我得趕緊去補料，勞駕稍候再來！」");
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
                                                    0 => format!("{bark} 一旁的{c_name}在攤前停下打量，好奇問道：「老闆，今天這料看著挺新鮮，火候正好呢！」 {v_name}笑著抬頭應道：「那是自然，天天現備的好料，您先瞧瞧，待會兒想吃隨時吩咐！」"),
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
                            if room_name.contains("鐵坊") || room_name.contains("鍛造") || room_name.contains("木作") || room_name.contains("工坊") {
                                let (item_id, item_name) = match ecology_tick % 3 {
                                    0 => ("mat_repair_strip", "鉚補鐵皮"),
                                    1 => ("mat_hemp_rope_roll", "麻繩卷"),
                                    _ => ("mat_iron_scrap", "碎鐵片"),
                                };
                                let _ = crate::db::add_to_inventory(id_b, item_id, 1);
                                exchange_action = Some(format!("遞過一包方清點完的{}", item_name));
                            } else if room_name.contains("診所") || room_name.contains("藥房") || room_name.contains("烘藥") || room_name.contains("脈絡") {
                                let _ = crate::db::add_to_inventory(id_b, "mat_herb_poultice", 1);
                                exchange_action = Some("將一罐方封好的百草敷料交到對方手裡".to_string());
                            } else if room_name.contains("國小") || room_name.contains("學院") || room_name.contains("書局") || room_name.contains("講堂") || room_name.contains("藏書") || room_name.contains("印行") {
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
                                let project_npc_utterance = |npc_id: &str, ent: &crate::store::Entity, is_reply: bool, seq: usize| -> String {
                                    let ob_opt = crate::store::get_active_npc_obligation(npc_id)
                                        .and_then(|(kind, subj)| match kind.as_str() {
                                            "restock_goods" => Some("店內現貨補倉".to_string()),
                                            "repair_tool" => Some("手頭器物修整".to_string()),
                                            "stranded_cargo" => Some("滯留貨物調度".to_string()),
                                            "repay_debt" => Some("手頭帳務結算".to_string()),
                                            "treat_injury" => Some("調養傷勢".to_string()),
                                            _ if !subj.trim().is_empty() && !subj.is_ascii() => Some(subj.trim().trim_end_matches('。').to_string()),
                                            _ => None,
                                        });
                                    let thought_opt = ent
                                        .thoughts
                                        .iter()
                                        .rev()
                                        .filter_map(|t| t.flavor.as_deref())
                                        .find(|f| !f.trim().is_empty() && !f.contains("發愁"))
                                        .map(|f| f.trim().trim_end_matches('。').to_string());
                                    let act_clean = ent
                                        .current_activity
                                        .trim()
                                        .trim_end_matches('。')
                                        .trim_start_matches("正在")
                                        .trim_start_matches('正')
                                        .trim()
                                        .to_string();

                                    match (ob_opt.as_deref(), thought_opt.as_deref(), act_clean.is_empty()) {
                                        (Some(ob), Some(th), false) => {
                                            if !is_reply {
                                                match seq % 3 {
                                                    0 => format!("「{}，偏生手頭還記掛著『{}』，正打算{}。」", th, ob, act_clean),
                                                    1 => format!("「這會兒正要{}，心裡倒還想著『{}』那樁差事。」", act_clean, ob),
                                                    _ => format!("「先把『{}』處置妥當要緊，{}。」", ob, th),
                                                }
                                            } else {
                                                match seq % 3 {
                                                    0 => format!("「可不是嗎，{}，我也正要{}，好把『{}』辦妥。」", th, act_clean, ob),
                                                    1 => format!("「誰說不是呢，我手裡也壓著『{}』，趁現在{}最穩妥。」", ob, act_clean),
                                                    _ => format!("「彼此彼此，{}，等忙完『{}』再歇口氣。」", th, ob),
                                                }
                                            }
                                        }
                                        (Some(ob), _, false) => {
                                            if !is_reply {
                                                format!("「手頭還惦記著『{}』，趁著這會兒得趕緊{}。」", ob, act_clean)
                                            } else {
                                                format!("「我這頭也記掛著『{}』，正打算{}呢。」", ob, act_clean)
                                            }
                                        }
                                        (None, Some(th), false) => {
                                            if !is_reply {
                                                format!("「{}，趁著精神還好，正打算{}。」", th, act_clean)
                                            } else {
                                                format!("「說的是，{}，我也正要{}。」", th, act_clean)
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
                                            if !is_reply {
                                                format!("「趁著四下順暢，我正打算{}。」", act_str)
                                            } else {
                                                format!("「正好，我也正要{}，忙妥了心裡踏實。」", act_str)
                                            }
                                        }
                                    }
                                };
                                let dlg_a = project_npc_utterance(id_a, ea, false, seq_idx);
                                let dlg_b = project_npc_utterance(id_b, eb, true, seq_idx.wrapping_add(1));

                                let action_prefix = if let Some(act) = &exchange_action {
                                    format!("{caller_a}邊{act}，邊")
                                } else {
                                    caller_a.clone()
                                };
                                let dialogue_narrate = match seq_idx % 6 {
                                    0 => format!("{action_prefix}側身向{caller_b}言道：{dlg_a}\n{caller_b}微微點頭，隨口應道：{dlg_b}"),
                                    1 => format!("{action_prefix}將手頭物事稍作整理，對{caller_b}開口：{dlg_a}\n{caller_b}抬眼笑應：{dlg_b}"),
                                    2 => format!("{action_prefix}望向四下市面，同{caller_b}搭話：{dlg_a}\n{caller_b}順著話頭接道：{dlg_b}"),
                                    3 => format!("{action_prefix}朝{caller_b}低聲說道：{dlg_a}\n{caller_b}神情專注，答道：{dlg_b}"),
                                    4 => format!("{action_prefix}手頭活計未停，對{caller_b}言道：{dlg_a}\n{caller_b}語調平穩地回道：{dlg_b}"),
                                    _ => format!("{action_prefix}向{caller_b}問道：{dlg_a}\n{caller_b}略一思索，溫聲回道：{dlg_b}"),
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
                                    format!("屋內燈火已熄，{}已在床鋪上安歇，四周寧靜無聲。", caller)
                                } else if !activity.is_empty() {
                                    let act_trimmed = activity.trim_end_matches(['。', '，', ' ']);
                                    let clean = act_trimmed
                                        .trim_start_matches("正在")
                                        .trim_start_matches('正')
                                        .trim();
                                    let mutter = if clean.contains("巡") || clean.contains("守") {
                                        let lines = [
                                            "自語道：「四下平靜，各處燈火安穩。」",
                                            "抬眼望了望橋下水勢，低聲道：「夜裡水色平緩，沿岸船隻都栓牢了。」",
                                            "撥亮了手頭的防風提燈，輕聲自語：「再繞過前面街口巡一圈，差不多就到換班時辰了。」",
                                        ];
                                        lines[seq_idx % lines.len()]
                                    } else if clean.contains("帳") || clean.contains("算") || clean.contains("冊") || clean.contains("批改") {
                                        let lines = [
                                            "低聲沉吟：「這筆紀錄核對清楚，分毫不差。」",
                                            "提筆在冊頁旁添了幾筆批註，點頭道：「這樣整理出來，明日查閱就一目了然了。」",
                                            "合上手頭的卷冊舒了口氣：「這幾頁總算批閱完畢，喝口溫茶歇一歇。」",
                                        ];
                                        lines[seq_idx % lines.len()]
                                    } else if clean.contains("火") || clean.contains("爐") || clean.contains("鍛") {
                                        let lines = [
                                            "擦了擦汗笑道：「這爐火候正旺，淬火火候恰到好處！」",
                                            "端詳著手裡的鐵件點頭：「器口平整厚實，這批成色真不錯。」",
                                            "添了一鏟耐火炭，自語道：「趁著爐溫勻稱，把這幾件活計一鼓作氣打完。」",
                                        ];
                                        lines[seq_idx % lines.len()]
                                    } else if clean.contains("藥") || clean.contains("焙") {
                                        let lines = [
                                            "聞了聞藥香自語道：「百草烘焙得極勻，藥氣純正。」",
                                            "仔細封妥陶罐木塞，低聲道：「防潮封口做紮實些，藥效才能留得長久。」",
                                        ];
                                        lines[seq_idx % lines.len()]
                                    } else {
                                        let lines = [
                                            "輕聲笑道：「手頭這樁事麻利辦完，待會兒去街上走走。」",
                                            "整理了一下衣袖，自語道：「按部就班把事情打理妥當，心裡也踏實。」",
                                            "抬頭看了看天色，低聲道：「今天天氣清爽，做事也格外順手。」",
                                        ];
                                        lines[seq_idx % lines.len()]
                                    };
                                    format!("一旁{}正{}，{}", caller, clean, mutter)
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
                        && let Some(cell) = crate::server::grid_manager::get_runtime_square_grid()
                            .and_then(|g| g.get(coord).cloned())
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
                        let viewer_person = if crate::store::player_knows_npc(&s.player_id, &sch.entity_id) {
                            person.clone()
                        } else {
                            let brief = crate::store::npc_brief_appearance_tag(&sch.entity_id, &title);
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
                            if !brief.is_empty() {
                                format!("{brief}的{role}")
                            } else {
                                format!("一名{role}")
                            }
                        };
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
                            let viewer_person = if crate::store::player_knows_npc(&s.player_id, &sch.entity_id) {
                                person.clone()
                            } else {
                                let brief = crate::store::npc_brief_appearance_tag(&sch.entity_id, &title);
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
                                if !brief.is_empty() {
                                    format!("{brief}的{role}")
                                } else {
                                    format!("一名{role}")
                                }
                            };
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
                let viewer_person = if crate::store::player_knows_npc(&s.player_id, &sch.entity_id) {
                    person.clone()
                } else {
                    let brief = crate::store::npc_brief_appearance_tag(&sch.entity_id, &title);
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
                    if !brief.is_empty() {
                        format!("{brief}的{role}")
                    } else {
                        format!("一名{role}")
                    }
                };
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

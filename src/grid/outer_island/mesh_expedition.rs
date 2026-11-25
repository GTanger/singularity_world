//! 《奇點世界・銀鷗島》零讀秒等待 × 網狀山海微拓撲 × 48候非阻塞分流執行期核心（Room 4387da46 定案）。
//!
//! 核心契約：
//! 1. 徒步每一步嚴守 0 秒瞬切，絕不塞任何跨格讀秒計時器或原地罰站鎖死。
//! 2. 48 候全時相非阻塞分流：大漲潮淹沒 `low_tide` 潮間礁灘時，引導當下 0 秒改攀 `climb_ridge` 高處崖徑；
//!    `climb_ridge` 與 `night_dim` 永遠不封鎖可行性（僅改變氣力消耗與照明／繩索省力回饋）。
//! 3. `campfire_nook` 背風火塘岩凹支援 0 秒生火熱糧舒緩疲勞；`summit_outlook` 觀日絕頂支援破曉四候觀日採露、
//!    深夜觀星辨秘徑、白晝登高揭露山海路勢，徹底消滅「爬上山頭呆站乾等太陽」。
//! 4. `sync_mesh_v6_topology_on_boot` 確保線上既有玩家、994 名居民與 11 具實體車船無損對齊新拓撲，銅幣守恆零漂移。

use serde_json::{Value, json};

use super::clock_and_lang::SolarClockSnapshot;
use super::live_tide::derive_tide_clock_details;
use super::mind::EventPublicity;
use super::movement::{add_carry_item, push_breath};
use super::persona::ValueAxis;
use super::sim_tick::broadcast_scene_event;
use super::types::{
    CarryItem, IslandWorldDelta, NodeExitDef, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc,
};

/// 每日兩次大滿潮時窗（對應 `live_tide.rs` 48 候之早潮 `15..=20` 與晚潮 `39..=44`，嚴格對齊編譯器 BFS 斷言）
pub fn is_high_tide_slot(tide_slot: u8) -> bool {
    (15..=20).contains(&tide_slot) || (39..=44).contains(&tide_slot)
}

/// 黎明破曉四候黃金觀日時窗（`8..=15`：殘夜、將曉、拂曉、破曉、晨微、朝露、晨光、朝日，約 10 真實分鐘）
pub fn is_dawn_window_slot(tide_slot: u8) -> bool {
    (8..=15).contains(&tide_slot)
}

fn actor_has_climbing_gear(actor: &UnifiedMindBody) -> bool {
    let wname = actor.bound_weapon.name.as_str();
    if wname.contains("鎬") || wname.contains("爪") || wname.contains("杖") {
        return true;
    }
    actor.backpack.iter().any(|i| {
        i.count > 0
            && (i.slug.contains("rope")
                || i.slug.contains("pick")
                || i.name.contains("麻繩")
                || i.name.contains("鐵鎬"))
    })
}

fn actor_has_light_source(actor: &UnifiedMindBody) -> bool {
    if actor.race.contains("精靈") || actor.race.contains("矮人") || actor.race.contains("獸人")
    {
        return true;
    }
    actor.backpack.iter().any(|i| {
        i.count > 0
            && (i.slug.contains("torch")
                || i.slug.contains("lantern")
                || i.slug == "charcoal_sack"
                || i.name.contains("火把")
                || i.name.contains("提燈"))
    })
}

/// 評估跨節點移動之時相分流與氣力代價：
/// - 回傳 `Ok((step_fatigue_delta, optional_flavor_note))` 或在大漲潮淹沒低灘時回傳非阻塞改道提示 `Err`。
pub fn evaluate_exit_env_gate(
    actor: &UnifiedMindBody,
    ex: &NodeExitDef,
    dst_node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
) -> Result<(u8, Option<&'static str>), String> {
    let td = derive_tide_clock_details(clock);
    match ex.env_gate.as_str() {
        "low_tide" => {
            if is_high_tide_slot(td.tide_slot) {
                return Err(format!(
                    "此刻正逢【{}】大潮湧入，白浪已淹沒通往【{}】的低處礁灘！無須在岸邊乾等退潮，請直接改走旁邊的高處山崖徑或折返繞行。",
                    td.tide_name, dst_node.name
                ));
            }
            Ok((1, Some("趁著潮水退落，你踏過裸露岩礁輕快切過淺灘。")))
        }
        "climb_ridge" => {
            if actor_has_climbing_gear(actor) {
                Ok((2, Some("你借隨身器物穩住支點，順著陡峭岩脊穩步攀上。")))
            } else {
                Ok((
                    5,
                    Some("陡坡風勁，你徒手抓穩岩稜老藤攀爬而上，額角微見熱汗。"),
                ))
            }
        }
        "night_dim" => {
            if !clock.is_night
                || actor_has_light_source(actor)
                || actor.mind.ltm.known_nodes.contains(&ex.to)
            {
                Ok((2, None))
            } else {
                Ok((4, Some("谷底夜霧深沉，你放慢腳步順著水聲與岩壁摸索前行。")))
            }
        }
        _ => {
            if dst_node.kind == "wild" || dst_node.dungeon {
                let carry_weight: u32 = actor
                    .backpack
                    .iter()
                    .map(|i| u32::from(i.weight_units) * i.count)
                    .sum();
                Ok((if carry_weight >= 8 { 4 } else { 2 }, None))
            } else {
                Ok((0, None))
            }
        }
    }
}

/// 為出口列表補上即時潮汐／攀脊／夜霧感官提示（讓玩家看一眼出口就能做路線抉擇）
pub fn exit_gate_ambient_cue(ex: &NodeExitDef, clock: &SolarClockSnapshot) -> Option<&'static str> {
    let td = derive_tide_clock_details(clock);
    match ex.env_gate.as_str() {
        "low_tide" => {
            if is_high_tide_slot(td.tide_slot) {
                Some("滿潮白浪淹沒礁面・宜改攀高處山崖徑")
            } else {
                Some("退潮暗礁裸露・可快步省力直切")
            }
        }
        "climb_ridge" => Some("高處山脊全天候暢通・徒手攀登較費氣力"),
        "night_dim" if clock.is_night => Some("谷底夜色幽暗・持照明或走高脊更省力"),
        _ => None,
    }
}

/// 解鎖雙向秘徑：當一端秘徑被打通時，若目標節點亦設有指回本節點的對稱秘徑，同步寫入 `unlocked_secrets`
pub fn unlock_bidirectional_secret(
    doc: &WorldStaticDoc,
    actor: &mut UnifiedMindBody,
    src_node_id: &str,
) {
    actor.unlocked_secrets.insert(src_node_id.to_string());
    if let Some(hid) = doc.hidden_entrances.get(src_node_id) {
        let target_id = hid.target.clone();
        actor.mind.ltm.known_nodes.insert(target_id.clone());
        if let Some(rev_hid) = doc.hidden_entrances.get(&target_id)
            && rev_hid.target == src_node_id
        {
            actor.unlocked_secrets.insert(target_id);
        }
    }
}

/// 在 `scene.objects` 插座掛載「背風石圈火塘（0 秒回氣）」與「山海瞭望岩／破曉觀日台（四候動態高光）」
pub fn append_expedition_scene_objects<F>(
    node: &SpatialNodeDef,
    clock: &SolarClockSnapshot,
    objects: &mut Vec<Value>,
    mut push_audit: F,
) where
    F: FnMut(&str, &str, &str, &str, &str, &str),
{
    let td = derive_tide_clock_details(clock);

    if node.kind == "campfire_nook" {
        let oid = "obj:campfire_nook";
        let lbl = "撥旺火塘烘暖歇腳";
        let sub = "就地生火熱糧 · 舒緩長途跋涉疲乏";
        objects.push(json!({
            "object_id": oid,
            "name": "背風石圈火塘與活水石窩",
            "cue": "餘炭乾爽避風・可生火舒緩腳力",
            "desc": "前人用扁平青石圍起的避風火塘，旁側岩縫滲出清冽山泉，隨時可添柴撥火、烘暖手腳恢復氣力。",
            "direct_action": {
                "id": "camp_hearth_rest",
                "param": "",
                "label": lbl,
            },
            "options": [],
        }));
        push_audit("camp_hearth_rest", "", lbl, sub, "object", oid);
    }

    if node.kind == "summit_outlook" {
        if is_dawn_window_slot(td.tide_slot) {
            let oid = "obj:summit_dawn";
            let lbl = "迎風觀滄海破曉並採集晨露草";
            let sub = format!("正逢【{}】破曉時窗", td.tide_name);
            objects.push(json!({
                "object_id": oid,
                "name": "破曉霞光與凝露岩台",
                "cue": format!("【{}】海天將明・岩縫凝結朝露靈草", td.tide_name),
                "desc": "東方海平線與雲海正翻湧起層層赤金霞光，凜冽晨風掠過峰頂，岩縫草葉尖端凝著晶瑩朝露。",
                "direct_action": {
                    "id": "summit_watch_sunrise",
                    "param": "",
                    "label": lbl,
                },
                "options": [],
            }));
            push_audit("summit_watch_sunrise", "", lbl, &sub, "object", oid);
        } else if clock.is_night {
            let oid = "obj:summit_stars";
            let lbl = "仰觀命星軌跡辨識山海秘徑";
            let sub = "夜空澄澈 · 俯察暗谷風向與古道刻痕";
            objects.push(json!({
                "object_id": oid,
                "name": "無雲夜空與山巔風標石",
                "cue": "星河低垂・可藉星位辨明周遭秘徑",
                "desc": "遠離聚落燈火的高處岩台，漫天星軌與腳下山海輪廓清晰分明，靜心觀望可察覺白晝難辨的暗谷通路。",
                "direct_action": {
                    "id": "summit_stargaze",
                    "param": "",
                    "label": lbl,
                },
                "options": [],
            }));
            push_audit("summit_stargaze", "", lbl, sub, "object", oid);
        } else {
            let oid = "obj:summit_survey";
            let lbl = "登高眺望山海路勢";
            let sub = "一覽本格與鄰境岔路走向";
            objects.push(json!({
                "object_id": oid,
                "name": "分水嶺瞭望岩台",
                "cue": "視野遼闊・可俯瞰山脊礁灣各處路徑",
                "desc": "站在高處裸岩憑風遠眺，周遭山谷獸徑、海岸潮線與遠方聚落炊煙皆歷歷在目。",
                "direct_action": {
                    "id": "summit_survey_terrain",
                    "param": "",
                    "label": lbl,
                },
                "options": [],
            }));
            push_audit("summit_survey_terrain", "", lbl, sub, "object", oid);
        }
    }
}

fn reveal_local_and_adjacent_mesh(
    doc: &WorldStaticDoc,
    actor: &mut UnifiedMindBody,
    center_node: &SpatialNodeDef,
) -> usize {
    let mut newly_revealed = 0usize;
    let cid = center_node.cell_id;
    actor.mind.ltm.known_cells.insert(cid);
    for nd in doc.nodes.values() {
        if nd.cell_id == cid && !nd.indoor && !nd.dungeon {
            if actor.mind.ltm.known_nodes.insert(nd.id.clone()) {
                newly_revealed += 1;
            }
            for ex in &nd.exits {
                if let Some(dst) = doc.nodes.get(&ex.to)
                    && !dst.indoor
                    && !dst.dungeon
                {
                    if actor.mind.ltm.known_nodes.insert(dst.id.clone()) {
                        newly_revealed += 1;
                    }
                }
            }
        }
    }
    newly_revealed
}

/// 執行零讀秒營火歇息與山巔破曉／觀星／眺望互動
pub fn apply_expedition_action(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_id: &str,
    node: &SpatialNodeDef,
    action: &str,
) -> Option<Result<String, String>> {
    match action {
        "camp_hearth_rest" => {
            if node.kind != "campfire_nook" {
                return Some(Err(
                    "此處風大無遮蔽，需在背風火塘或山泉石窩才能生火整頓。".to_string()
                ));
            }
            let win_key = format!("camp:{}@{}", node.id, clock.six_phase_key);
            let already = delta
                .actors
                .get(actor_id)
                .is_some_and(|a| a.explored_windows.contains(&win_key));
            if already {
                return Some(Ok(format!(
                    "這個時段（{}）你已在【{}】生火整頓過，手腳溫熱、氣息尚穩，隨時可繼續趕路。",
                    clock.bell_label, node.name
                )));
            }
            let Some(a) = delta.actors.get_mut(actor_id) else {
                return Some(Err("找不到旅人行蹤。".to_string()));
            };
            super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
            a.explored_windows.insert(win_key);
            a.fatigue = a.fatigue.saturating_sub(38);
            a.mind.stm.affect.fatigue = a.mind.stm.affect.fatigue.saturating_sub(38);
            a.mark_mutated();
            let msg = format!(
                "你在【{}】撥旺石圈餘炭、掬起清泉洗去滿臉風塵，雙腿緊繃的痠乏頓時消散大半。",
                node.name
            );
            push_breath(a, msg.clone());
            Some(Ok(msg))
        }
        "summit_watch_sunrise" => {
            if node.kind != "summit_outlook" {
                return Some(Err(
                    "需登上高處瞭望岩或觀日絕頂方能迎觀滄海日出。".to_string()
                ));
            }
            let td = derive_tide_clock_details(clock);
            if !is_dawn_window_slot(td.tide_slot) {
                return Some(Err(format!(
                    "此刻天色為【{}】，已過破曉觀日時窗；不妨改為登高眺望山海路勢。",
                    td.tide_name
                )));
            }
            let win_key = format!("dawn:{}@d{}", node.id, clock.game_days);
            if delta
                .actors
                .get(actor_id)
                .is_some_and(|a| a.explored_windows.contains(&win_key))
            {
                return Some(Ok(format!(
                    "今日破曉你已在【{}】迎過第一道朝陽並採妥晨露草，胸中神清氣爽。",
                    node.name
                )));
            }
            let actor_name = delta
                .actors
                .get(actor_id)
                .map_or("旅人", |a| a.display_name.as_str())
                .to_string();
            let mut secret_note = String::new();
            {
                let Some(a) = delta.actors.get_mut(actor_id) else {
                    return Some(Err("找不到旅人行蹤。".to_string()));
                };
                super::live_tide::prune_stale_day_window_keys(
                    &mut a.explored_windows,
                    clock.game_days,
                );
                a.explored_windows.insert(win_key);
                a.fatigue = 0;
                a.mind.stm.affect.fatigue = 0;
                a.mind.stm.affect.hope = a.mind.stm.affect.hope.saturating_add(25).min(100);
                reveal_local_and_adjacent_mesh(doc, a, node);
                if doc.hidden_entrances.contains_key(&node.id)
                    && !a.unlocked_secrets.contains(&node.id)
                {
                    unlock_bidirectional_secret(doc, a, &node.id);
                    if let Some(hid) = doc.hidden_entrances.get(&node.id) {
                        secret_note =
                            format!(" 晨光斜照下，你更一眼看清了岩側暗藏的秘徑【{}】！", hid.dir);
                    }
                }
                add_carry_item(
                    &mut a.backpack,
                    CarryItem {
                        slug: "dawn_dew_herb".to_string(),
                        name: "破曉凝露靈草".to_string(),
                        count: 1,
                        weight_units: 1,
                        ..CarryItem::default()
                    },
                );
                a.mark_mutated();
            }
            broadcast_scene_event(
                doc,
                delta,
                clock,
                &node.id,
                actor_id,
                &actor_name,
                EventPublicity::PublicLoud,
                None,
                82,
                55,
                ValueAxis::CuriosityVenture,
                "watch_island_sunrise",
                &format!(
                    "{}翻山越嶺於破曉時分登上【{}】迎觀滄海日出",
                    actor_name, node.name
                ),
            );
            let msg = format!(
                "萬丈金紅霞光刺破寧靜大洋晨霧，一輪朝陽自海天交界處磅礴躍出！連夜攀登的疲憊一掃而空，你順手採下岩縫間一株晶瑩的【破曉凝露靈草】。{}",
                secret_note
            );
            if let Some(a) = delta.actors.get_mut(actor_id) {
                push_breath(a, msg.clone());
            }
            Some(Ok(msg))
        }
        "summit_stargaze" => {
            if node.kind != "summit_outlook" {
                return Some(Err("需在視野無遮的高處岩台方能仰觀星軌。".to_string()));
            }
            let Some(a) = delta.actors.get_mut(actor_id) else {
                return Some(Err("找不到旅人行蹤。".to_string()));
            };
            a.fatigue = a.fatigue.saturating_sub(15);
            let revealed = reveal_local_and_adjacent_mesh(doc, a, node);
            let mut secret_msg = String::new();
            if doc.hidden_entrances.contains_key(&node.id) && !a.unlocked_secrets.contains(&node.id)
            {
                unlock_bidirectional_secret(doc, a, &node.id);
                if let Some(hid) = doc.hidden_entrances.get(&node.id) {
                    secret_msg = format!(
                        " 藉著清冷星輝，你赫然辨出岩壁旁暗藏的秘徑出口【{}】！",
                        hid.dir
                    );
                }
            }
            a.mark_mutated();
            let msg = format!(
                "你立於【{}】仰觀漫天星軌、俯聽暗谷風聲，將周邊山海走向印入腦海（新辨明 {} 處鄰近地段）。{}",
                node.name, revealed, secret_msg
            );
            push_breath(a, msg.clone());
            Some(Ok(msg))
        }
        "summit_survey_terrain" => {
            if node.kind != "summit_outlook" {
                return Some(Err("需登上高處瞭望岩台方能俯瞰周遭路勢。".to_string()));
            }
            let Some(a) = delta.actors.get_mut(actor_id) else {
                return Some(Err("找不到旅人行蹤。".to_string()));
            };
            let revealed = reveal_local_and_adjacent_mesh(doc, a, node);
            let mut secret_hint = String::new();
            if let Some(hid) = doc.hidden_entrances.get(&node.id) {
                if !a.unlocked_secrets.contains(&node.id) {
                    unlock_bidirectional_secret(doc, a, &node.id);
                    secret_hint = format!(
                        " 居高臨下細察，你更找出了通往【{}】的隱秘山海捷徑（{}）！",
                        hid.to_name, hid.dir
                    );
                }
            }
            a.mark_mutated();
            let msg = format!(
                "你憑立【{}】極目遠眺，將本區高低雙線山徑、背風火塘與鄰境坡口盡數收入眼底（新探明 {} 處地段）。{}",
                node.name, revealed, secret_hint
            );
            push_breath(a, msg.clone());
            Some(Ok(msg))
        }
        _ => None,
    }
}

/// 啟動與熱遷移無損同步：確保線上既有角色、NPC 路網認知與 11 具實體車船路線對齊最新網狀拓撲
pub fn sync_mesh_v6_topology_on_boot(doc: &WorldStaticDoc, delta: &mut IslandWorldDelta) {
    let public_walkable_nodes: Vec<String> = doc
        .nodes
        .values()
        .filter(|nd| !nd.dungeon && (!nd.indoor || nd.kind == "shop"))
        .map(|nd| nd.id.clone())
        .collect();

    let actor_ids: Vec<String> = delta.actors.keys().cloned().collect();
    for aid in actor_ids {
        let Some(a_ro) = delta.actors.get(&aid) else {
            continue;
        };
        let node_invalid = !doc.nodes.contains_key(&a_ro.node_id);
        let home_invalid = !a_ro.home_node.is_empty() && !doc.nodes.contains_key(&a_ro.home_node);
        let work_invalid = !a_ro.work_node.is_empty() && !doc.nodes.contains_key(&a_ro.work_node);
        let route_invalid = a_ro
            .route_plan
            .iter()
            .any(|nid| !doc.nodes.contains_key(nid));
        let is_npc = a_ro.controller_id.is_none();
        let npc_missing_mesh =
            is_npc && a_ro.mind.ltm.known_nodes.len() < public_walkable_nodes.len();

        let mut missing_rev_secrets: Vec<String> = Vec::new();
        for u in &a_ro.unlocked_secrets {
            if let Some(hid) = doc.hidden_entrances.get(u)
                && let Some(rev) = doc.hidden_entrances.get(&hid.target)
                && rev.target == *u
                && !a_ro.unlocked_secrets.contains(&hid.target)
            {
                missing_rev_secrets.push(hid.target.clone());
            }
        }

        if !node_invalid
            && !home_invalid
            && !work_invalid
            && !route_invalid
            && !npc_missing_mesh
            && missing_rev_secrets.is_empty()
        {
            continue;
        }

        if let Some(a) = delta.actors.get_mut(&aid) {
            if node_invalid {
                let parsed_cell = a
                    .node_id
                    .strip_prefix("oi:cell")
                    .and_then(|rest| rest.split(':').next())
                    .unwrap_or("");
                let fallback = format!("oi:cell{parsed_cell}:0");
                if !parsed_cell.is_empty() && doc.nodes.contains_key(&fallback) {
                    a.node_id = fallback;
                } else {
                    a.node_id = doc.spawn_node.clone();
                }
            }
            if home_invalid {
                a.home_node = a.node_id.clone();
            }
            if work_invalid {
                a.work_node = a.node_id.clone();
            }
            if route_invalid {
                a.route_plan.clear();
            }
            if npc_missing_mesh {
                for wn in &public_walkable_nodes {
                    a.mind.ltm.known_nodes.insert(wn.clone());
                }
            }
            for rs in missing_rev_secrets {
                a.unlocked_secrets.insert(rs);
            }
            a.mark_mutated();
        }
    }

    // 同步已序列化的 11 具車船路線節點陣列至最新步車分軌樞紐鏈（保留乘客與貨箱不遺失）
    for veh in &mut delta.transit_vehicles {
        if let Some(line) = doc.transit_lines.iter().find(|l| l.line_id == veh.line_id)
            && veh.route_nodes != line.route_nodes
            && !line.route_nodes.is_empty()
        {
            let old_cur = veh
                .route_nodes
                .get(veh.route_idx)
                .cloned()
                .unwrap_or_default();
            let new_idx = line
                .route_nodes
                .iter()
                .position(|x| *x == old_cur)
                .unwrap_or_else(|| veh.route_idx.min(line.route_nodes.len() - 1));
            veh.route_nodes = line.route_nodes.clone();
            veh.route_idx = new_idx;
            delta.extras_dirty = true;
        }
    }
    delta.rebuild_indexes();
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, VecDeque};

    use super::super::actions::apply_world_action;
    use super::super::clock_and_lang::solar_clock_at;
    use super::super::engine::load_static_world_doc;
    use super::super::explore_and_combat::apply_explore;
    use super::super::movement::{apply_move_step, apply_setup_origin};
    use super::super::rural_migrate::ensure_rural_runtime_initialized;
    use super::super::sim_tick::seed_genesis_actors_once;
    use super::super::types::IslandWorldDelta;
    use super::*;

    #[test]
    fn mesh_v6_enforces_zero_gate_collisions_and_48_phase_non_blocking_invariant() {
        let doc = load_static_world_doc();
        assert_eq!(doc.cells.len(), 93);
        assert!(
            doc.nodes.len() >= 840,
            "expected expanded mesh nodes >= 840, got {}",
            doc.nodes.len()
        );
        assert!(
            doc.hidden_entrances.len() >= 25,
            "expected bidirectional mountain/sea secrets >= 25, got {}",
            doc.hidden_entrances.len()
        );

        // 1. 驗證全島 0 處「出村門/城門 1 步直接撞進另一座聚落門」
        for nd in doc.nodes.values() {
            if matches!(nd.kind.as_str(), "village_gate" | "gate") {
                for ex in &nd.exits {
                    let dst = doc.nodes.get(&ex.to).expect("valid exit target");
                    if dst.cell_id != nd.cell_id {
                        assert!(
                            !matches!(dst.kind.as_str(), "village_gate" | "gate"),
                            "1-step gate-to-gate collision found: {} -> {}",
                            nd.id,
                            dst.id
                        );
                    }
                }
            }
        }

        // 2. 驗證全日 48 候任一時相下（含大滿潮封閉 low_tide 邊），全島所有戶外節點 100% 無阻塞可達
        let outdoor_ids: Vec<&str> = doc
            .nodes
            .values()
            .filter(|n| !n.indoor && !n.dungeon)
            .map(|n| n.id.as_str())
            .collect();
        for slot in 0..48u8 {
            let high_tide = is_high_tide_slot(slot);
            let mut vis: BTreeSet<&str> = BTreeSet::new();
            let mut q: VecDeque<&str> = VecDeque::new();
            vis.insert(doc.spawn_node.as_str());
            q.push_back(doc.spawn_node.as_str());
            while let Some(cur) = q.pop_front() {
                let cnode = doc.nodes.get(cur).expect("node exists");
                for ex in &cnode.exits {
                    if high_tide && ex.env_gate == "low_tide" {
                        continue;
                    }
                    let dst = doc.nodes.get(&ex.to).expect("dst exists");
                    if dst.indoor || dst.dungeon {
                        continue;
                    }
                    if vis.insert(dst.id.as_str()) {
                        q.push_back(dst.id.as_str());
                    }
                }
            }
            assert_eq!(
                vis.len(),
                outdoor_ids.len(),
                "48-phase non-blocking invariant broken at tide_slot={slot}"
            );
        }
    }

    #[test]
    fn tide_reroute_campfire_zero_wait_and_dawn_sunrise_preserve_copper_conservation() {
        let doc = load_static_world_doc();
        let mut delta = IslandWorldDelta {
            genesis_total_copper: doc.ledger_genesis.total_conserved_copper,
            municipal_treasury_copper: doc.ledger_genesis.municipal_treasury_copper,
            silver_bank_reserve_copper: doc.ledger_genesis.silver_bank_reserve_copper,
            ..IslandWorldDelta::default()
        };
        seed_genesis_actors_once(&doc, &mut delta, false);
        ensure_rural_runtime_initialized(&doc, &mut delta, 1);
        assert_eq!(delta.conservation_diff(&doc), 0);

        let (epoch, _) = crate::game::game_time_cfg();
        let base_clk = solar_clock_at(epoch + 300);
        let pid = "test_mesh_traveler";
        apply_setup_origin(
            &doc,
            &mut delta,
            &base_clk,
            pid,
            "海崖行者",
            "人類",
            "forest_hunter",
            "背負麻繩草鞋",
        )
        .expect("setup player");
        let aid = delta.resolve_actor_id(pid).expect("actor id");

        // 移至濱海荒野格 #4074 前緣坡口，實測滿潮非阻塞改攀高崖（含疲勞=95極限仍可通行）
        if let Some(a) = delta.actors.get_mut(&aid) {
            a.node_id = "oi:cell4074:0".to_string();
            a.fatigue = 95;
        }
        // 取早潮滿潮時刻：tide_slot = 16
        let high_tide_clock = solar_clock_at(epoch + 16 * 75 + 10);
        let ht_err = apply_move_step(
            &doc,
            &mut delta,
            &high_tide_clock,
            pid,
            "oi:cell4074:low",
            10_000,
        )
        .expect_err("high tide should guide reroute off low reef");
        assert!(ht_err.contains("無須在岸邊乾等退潮"));

        // 即使滿潮且 fatigue == 95，當下 0 秒改攀高處山崖徑依然 100% 暢通不鎖死！
        apply_move_step(
            &doc,
            &mut delta,
            &high_tide_clock,
            pid,
            "oi:cell4074:high",
            11_000,
        )
        .expect("high ridge must remain passable at high tide and max fatigue");

        // 下切背風火塘 0 秒生火回氣
        apply_move_step(
            &doc,
            &mut delta,
            &high_tide_clock,
            pid,
            "oi:cell4074:camp",
            12_000,
        )
        .expect("enter campfire nook");
        apply_world_action(
            &doc,
            &mut delta,
            &high_tide_clock,
            pid,
            "camp_hearth_rest",
            "",
        )
        .expect("zero-wait campfire rest");
        assert!(delta.actors.get(&aid).expect("actor").fatigue <= 60);

        // 攀回高徑並登頂北岬東極望日斷崖，於破曉時窗（tide_slot = 11）迎觀日出＋雙向秘徑解鎖
        apply_move_step(
            &doc,
            &mut delta,
            &high_tide_clock,
            pid,
            "oi:cell4074:high",
            13_000,
        )
        .expect("back to high");
        apply_move_step(
            &doc,
            &mut delta,
            &high_tide_clock,
            pid,
            "oi:cell4074:vantage",
            14_000,
        )
        .expect("climb summit");
        let dawn_clock = solar_clock_at(epoch + 3600 + 11 * 75 + 10);
        let dawn_msg = apply_world_action(
            &doc,
            &mut delta,
            &dawn_clock,
            pid,
            "summit_watch_sunrise",
            "",
        )
        .expect("watch sunrise");
        assert!(dawn_msg.contains("破曉凝露靈草"));
        let a_after = delta.actors.get(&aid).expect("actor");
        assert_eq!(a_after.fatigue, 0);
        assert!(a_after.unlocked_secrets.contains("oi:cell4074:vantage"));
        assert!(a_after.unlocked_secrets.contains("oi:cell4076:0"));
        assert_eq!(delta.conservation_diff(&doc), 0);

        // 驗證現場探索無 rumor_key 之山海秘徑亦直接雙向打通
        if let Some(a) = delta.actors.get_mut(&aid) {
            a.node_id = "oi:cell4615:vantage".to_string();
        }
        apply_explore(&doc, &mut delta, &dawn_clock, pid).expect("explore mountain secret");
        let a_sec = delta.actors.get(&aid).expect("actor");
        assert!(a_sec.unlocked_secrets.contains("oi:cell4615:vantage"));
        assert!(a_sec.unlocked_secrets.contains("oi:cell4526:far"));
        assert_eq!(delta.conservation_diff(&doc), 0);
    }
}

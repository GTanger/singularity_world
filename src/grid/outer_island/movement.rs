//! 全服單一 `UnifiedMindBody` 初抵入籍、背包小工具、語言耳濡目染、街道尋路與步履／篷車移動。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::clock_and_lang::{SolarClockSnapshot, language_name_zh};
use super::mind::{CognitiveMind, EventPublicity};
use super::persona::{CharacterDriftState, ValueAxis};
use super::sim_tick::broadcast_scene_event;
use super::types::{
    BoundTool, CarryItem, IslandWorldDelta, SkirmishState, SpatialNodeDef, TransitState,
    UnifiedMindBody, WorldStaticDoc,
};

pub const MAX_BREATHS: usize = 8;

pub fn push_breath(a: &mut UnifiedMindBody, text: impl Into<String>) {
    let line = text.into();
    if line.trim().is_empty() {
        return;
    }
    a.recent_breaths.push_back(line);
    while a.recent_breaths.len() > MAX_BREATHS {
        a.recent_breaths.pop_front();
    }
}

pub fn new_uninitialized_actor(
    doc: &WorldStaticDoc,
    controller_id: &str,
    name: &str,
    soul_seed: u64,
) -> UnifiedMindBody {
    let mut langs = BTreeMap::new();
    langs.insert("harbor_trade".to_string(), 85);
    let mut known_state_cells = BTreeSet::new();
    let mut mind = CognitiveMind::default();
    if let Some(spawn) = doc.nodes.get(&doc.spawn_node) {
        known_state_cells.insert(spawn.cell_id);
        mind.seed_local_map_knowledge(
            &doc.spawn_node,
            &doc.spawn_node,
            spawn.cell_id,
            std::iter::empty(),
        );
    }

    let mut breaths = VecDeque::new();
    breaths.push_back(
        "海霧散開，雙桅商船緩緩靠上白鷗港石棧橋，潮濕海風裡傳來遠處矮人鐵砧的敲擊聲。".to_string(),
    );

    UnifiedMindBody {
        actor_id: format!("pl:{controller_id}"),
        controller_id: Some(controller_id.to_string()),
        soul_seed,
        rev: 1,
        alive: true,
        setup_done: false,
        display_name: if name.trim().is_empty() {
            controller_id.to_string()
        } else {
            name.trim().to_string()
        },
        race: "人類".to_string(),
        role: "遠航旅人".to_string(),
        appearance: "風塵僕僕的遠航旅人".to_string(),
        origin_pack: "smith_apprentice".to_string(),
        household_id: String::new(),
        home_node: doc.spawn_node.clone(),
        work_node: doc.spawn_node.clone(),
        node_id: doc.spawn_node.clone(),
        route_plan: VecDeque::new(),
        copper: 0,
        fatigue: 8,
        wound_part: None,
        severe_wrist_warning: false,
        bound_weapon: BoundTool {
            slug: "harbor_hammer".to_string(),
            name: "港務舊鍛錘".to_string(),
            provenance: "白鷗港港務局借予初抵學徒的粗鐵短錘，柄尾烙有港務火印".to_string(),
            wear_stage: 0,
            grip_familiarity: 8,
            rune_mark: None,
        },
        backpack: Vec::new(),
        inn_locker: Vec::new(),
        inn_lockers: BTreeMap::new(),
        inn_nights_left: 0,
        owned_house_node: None,
        rescue_pact_active: false,
        conversing_until_ms: 0,
        languages: langs,
        lang_daily_gain: BTreeMap::new(),
        lang_gain_day: 0,
        known_state_cells,
        unlocked_secrets: BTreeSet::new(),
        known_rumors: BTreeSet::new(),
        explored_windows: BTreeSet::new(),
        recent_breaths: breaths,
        active_skirmish: None,
        active_transit: None,
        last_step_unix_ms: 0,
        last_talk_unix_ms: 0,
        last_active_unix: 0,
        drift: CharacterDriftState::default(),
        mind,
        dirty: true,
        flushed_rev: 0,
        flushed_copper: i64::MIN,
    }
}

pub fn add_language_exposure(
    a: &mut UnifiedMindBody,
    game_day: i64,
    lang_code: &str,
    raw_gain: u8,
) -> Option<String> {
    if a.lang_gain_day != game_day {
        a.lang_gain_day = game_day;
        a.lang_daily_gain.clear();
    }
    let used = a.lang_daily_gain.get(lang_code).copied().unwrap_or(0);
    if used >= 30 {
        return None;
    }
    let allowed = raw_gain.min(30 - used);
    if allowed == 0 {
        return None;
    }
    let cur = a.languages.get(lang_code).copied().unwrap_or(0);
    let next = cur.saturating_add(allowed).min(100);
    if next == cur {
        return None;
    }
    a.languages.insert(lang_code.to_string(), next);
    a.lang_daily_gain
        .insert(lang_code.to_string(), used + allowed);
    a.mark_mutated();
    let lzh = language_name_zh(lang_code);
    if cur < 25 && next >= 25 {
        Some(format!(
            "耳濡目染之下，你開始能從{}裡聽出零星關鍵字眼了。",
            lzh
        ))
    } else if cur < 80 && next >= 80 {
        Some(format!("你對{}已漸趨熟稔，日常交談大多能聽懂來意。", lzh))
    } else {
        Some(format!("你默默記下了幾句{}的發音腔調。", lzh))
    }
}

pub fn add_carry_item(list: &mut Vec<CarryItem>, item: CarryItem) {
    if let Some(existing) = list.iter_mut().find(|x| {
        x.slug == item.slug
            && x.bound_owner == item.bound_owner
            && x.stolen_from == item.stolen_from
    }) {
        existing.count += item.count;
        return;
    }
    list.push(item);
}

pub fn take_carry_item(list: &mut Vec<CarryItem>, slug: &str, count: u32) -> Option<CarryItem> {
    let idx = list
        .iter()
        .position(|x| x.slug == slug && x.count >= count)?;
    let mut taken = list[idx].clone();
    taken.count = count;
    if list[idx].count == count {
        list.remove(idx);
    } else {
        list[idx].count -= count;
    }
    Some(taken)
}

#[expect(clippy::too_many_arguments, reason = "explicit origin setup arguments")]
pub fn apply_setup_origin(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    controller_id: &str,
    display_name: &str,
    race: &str,
    appearance: &str,
    origin_pack: &str,
) -> Result<String, String> {
    let valid_race = match race.trim() {
        "人類" | "矮人" | "精靈" | "獸人" => race.trim(),
        _ => "人類",
    };
    let valid_pack = match origin_pack.trim() {
        "forest_hunter" | "fallen_peddler" | "deck_sailor" | "smith_apprentice" => {
            origin_pack.trim()
        }
        _ => "smith_apprentice",
    };
    let clean_name: String = display_name
        .trim()
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '"' | '\'' | '`' | '{' | '}' | '[' | ']'))
        .collect();
    if clean_name.chars().count() < 2 || clean_name.chars().count() > 16 {
        return Err("請在登岸木冊留下 2 至 16 字的名號，方便鎮上街坊稱呼。".to_string());
    }
    let clean_app = if appearance.trim().is_empty() {
        "披著海風舊斗篷、神色沉穩"
    } else {
        appearance.trim()
    };

    let canonical_aid = if let Some(existing_aid) = delta.resolve_controller(controller_id) {
        existing_aid
    } else {
        let init_actor = new_uninitialized_actor(doc, controller_id, &clean_name, 0);
        let aid = init_actor.actor_id.clone();
        delta.upsert_actor(init_actor);
        aid
    };

    let entry = delta
        .actors
        .get_mut(&canonical_aid)
        .ok_or("找不到旅人行蹤")?;
    entry.display_name = clean_name;
    entry.appearance = clean_app.chars().take(40).collect();
    entry.last_active_unix = clock.real_unix;
    entry.mark_mutated();

    if entry.setup_done {
        return Ok("已更新你的行旅名號與外貌裝束。".to_string());
    }

    entry.race = valid_race.to_string();
    entry.origin_pack = valid_pack.to_string();
    entry.languages.clear();
    match valid_race {
        "矮人" => {
            entry.languages.insert("harbor_trade".to_string(), 75);
            entry.languages.insert("dwarven".to_string(), 100);
        }
        "精靈" => {
            entry.languages.insert("harbor_trade".to_string(), 75);
            entry.languages.insert("elvish".to_string(), 100);
        }
        "獸人" => {
            entry.languages.insert("harbor_trade".to_string(), 70);
            entry.languages.insert("orcish".to_string(), 100);
            entry.languages.insert("goblin".to_string(), 30);
        }
        _ => {
            entry.languages.insert("harbor_trade".to_string(), 95);
        }
    }

    entry.backpack.clear();
    let (role_zh, tool, starter_item, intro) = match valid_pack {
        "forest_hunter" => (
            "山林獵戶",
            BoundTool {
                slug: "old_hunting_knife".to_string(),
                name: "舊鹿角獵刀".to_string(),
                provenance: "隨身帶下船的厚背短獵刀，刀鞘纏著防滑麻繩".to_string(),
                wear_stage: 0,
                grip_familiarity: 10,
                rune_mark: None,
            },
            CarryItem {
                slug: "rye_bread".to_string(),
                name: "黑麥乾糧".to_string(),
                count: 2,
                bound_owner: None,
                stolen_from: None,
                weight_units: 1,
            },
            "你緊了緊腰間的舊鹿角獵刀，踏上白鷗港濕潤的石棧橋。",
        ),
        "fallen_peddler" => (
            "行腳商旅",
            BoundTool {
                slug: "iron_walking_rod".to_string(),
                name: "包鐵行商手杖".to_string(),
                provenance: "走南闖北防身撐路用的硬木包鐵長杖".to_string(),
                wear_stage: 0,
                grip_familiarity: 6,
                rune_mark: None,
            },
            CarryItem {
                slug: "lodging_voucher".to_string(),
                name: "船東抵宿券".to_string(),
                count: 1,
                bound_owner: Some(canonical_aid.clone()),
                stolen_from: None,
                weight_units: 0,
            },
            "你揣好懷裡那張寫著你名字的船東抵宿券，拄著包鐵長杖走下跳板，可先去鏽錨酒館換兩宿安穩住處。",
        ),
        "deck_sailor" => (
            "遠航水手",
            BoundTool {
                slug: "belaying_club".to_string(),
                name: "包銅繫纜短棍".to_string(),
                provenance: "甲板上敲緊纜樁與防身兩用的沉手硬木棍".to_string(),
                wear_stage: 0,
                grip_familiarity: 10,
                rune_mark: None,
            },
            CarryItem {
                slug: "hemp_rope".to_string(),
                name: "粗麻繩".to_string(),
                count: 1,
                bound_owner: None,
                stolen_from: None,
                weight_units: 2,
            },
            "你扛起一卷結實粗麻繩踏上白鷗港碼頭，港邊搬運苦力正吆喝著缺人搭手。",
        ),
        _ => (
            "鍛鐵學徒",
            BoundTool {
                slug: "harbor_hammer".to_string(),
                name: "港務舊鍛錘".to_string(),
                provenance: "白鷗港港務局借予初抵學徒的粗鐵短錘，柄尾烙有港務火印".to_string(),
                wear_stage: 0,
                grip_familiarity: 10,
                rune_mark: None,
            },
            CarryItem {
                slug: "lodging_voucher".to_string(),
                name: "船東抵宿券".to_string(),
                count: 1,
                bound_owner: Some(canonical_aid.clone()),
                stolen_from: None,
                weight_units: 0,
            },
            "你拎著港務舊鍛錘與一張船東抵宿券踏上白鷗港碼頭——西面鐵匠街爐火正旺，鏽錨酒館也能憑券歇腳。",
        ),
    };
    entry.role = role_zh.to_string();
    entry.bound_weapon = tool;
    add_carry_item(&mut entry.backpack, starter_item);
    entry.setup_done = true;
    push_breath(entry, intro);
    Ok(intro.to_string())
}

pub fn shortest_node_path(doc: &WorldStaticDoc, start: &str, goal: &str) -> Option<Vec<String>> {
    if start == goal {
        return Some(vec![start.to_string()]);
    }
    let mut prev: BTreeMap<String, String> = BTreeMap::new();
    let mut vis: BTreeSet<String> = BTreeSet::new();
    let mut q = VecDeque::new();
    vis.insert(start.to_string());
    q.push_back(start.to_string());

    while let Some(cur) = q.pop_front() {
        let Some(node) = doc.nodes.get(&cur) else {
            continue;
        };
        for ex in &node.exits {
            if ex.kind == "door" || ex.kind == "stairs" || ex.kind == "dungeon" {
                continue;
            }
            if !vis.contains(&ex.to) {
                vis.insert(ex.to.clone());
                prev.insert(ex.to.clone(), cur.clone());
                if ex.to == goal {
                    let mut path = vec![goal.to_string()];
                    let mut p = goal.to_string();
                    while let Some(parent) = prev.get(&p) {
                        path.push(parent.clone());
                        if parent == start {
                            break;
                        }
                        p = parent.clone();
                    }
                    path.reverse();
                    return Some(path);
                }
                q.push_back(ex.to.clone());
            }
        }
    }
    None
}

/// 供步行使用之真實街道與門戶尋路（允許出入起終點房門，不穿越他人私宅或未探索地城）
pub fn shortest_walk_path(
    doc: &WorldStaticDoc,
    start: &str,
    goal: &str,
    known_nodes: &BTreeSet<String>,
) -> Option<Vec<String>> {
    if start == goal {
        return Some(vec![start.to_string()]);
    }
    let mut prev: BTreeMap<String, String> = BTreeMap::new();
    let mut vis: BTreeSet<String> = BTreeSet::new();
    let mut q = VecDeque::new();
    vis.insert(start.to_string());
    q.push_back(start.to_string());

    while let Some(cur) = q.pop_front() {
        let Some(node) = doc.nodes.get(&cur) else {
            continue;
        };
        for ex in &node.exits {
            if ex.kind == "dungeon" || ex.kind == "secret_out" {
                continue;
            }
            if vis.contains(&ex.to) {
                continue;
            }
            if ex.to != goal {
                if !known_nodes.is_empty() && !known_nodes.contains(&ex.to) {
                    continue;
                }
                if let Some(dst) = doc.nodes.get(&ex.to)
                    && (dst.dungeon || dst.kind == "house" || dst.kind == "house_vacant")
                {
                    continue;
                }
            }
            vis.insert(ex.to.clone());
            prev.insert(ex.to.clone(), cur.clone());
            if ex.to == goal {
                let mut path = vec![goal.to_string()];
                let mut p = goal.to_string();
                while let Some(parent) = prev.get(&p) {
                    path.push(parent.clone());
                    if parent == start {
                        break;
                    }
                    p = parent.clone();
                }
                path.reverse();
                return Some(path);
            }
            q.push_back(ex.to.clone());
        }
    }
    None
}

pub fn apply_move_step(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    target_node: &str,
    now_ms: u64,
) -> Result<String, String> {
    let Some(aid) = delta.resolve_actor_id(actor_key) else {
        return Err("請先向港務官核對通行身分。".to_string());
    };
    let cur_node_id = {
        let Some(a) = delta.actors.get(&aid) else {
            return Err("請先向港務官核對通行身分。".to_string());
        };
        if !a.setup_done {
            return Err("請先在港務登岸木冊留下你的名號與來歷。".to_string());
        }
        if a.active_skirmish.is_some() {
            return Err("眼前正與敵手短兵對峙，請先出招應對或虛晃退開！".to_string());
        }
        if a.active_transit.is_some() {
            return Err("你正坐在雙馬篷車上趕路，若要改走山徑請先拍板中途下車。".to_string());
        }
        if now_ms > 0 && a.last_step_unix_ms > 0 && now_ms.saturating_sub(a.last_step_unix_ms) < 180
        {
            return Err("腳步太急，先穩住氣息再邁步。".to_string());
        }
        a.node_id.clone()
    };

    let Some(cur_node) = doc.nodes.get(&cur_node_id) else {
        return Err("四周霧氣迷濛，暫時辨不清方位。".to_string());
    };

    if cur_node_id == "oi:dng_pit:lair" && target_node == "oi:dng_pit:altar" {
        let cleared_until = delta
            .cleared_nests_until_day
            .get("oi:dng_pit:lair")
            .copied()
            .unwrap_or(-1);
        if clock.game_days >= cleared_until {
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.active_skirmish = Some(SkirmishState {
                foe_slug: "stone_spider".to_string(),
                foe_name: "背嵌原石的吞石巨蛛".to_string(),
                can_speak_goblin: false,
                foe_stagger: 0,
                player_guard: false,
                round_idx: 1,
            });
            a.mark_mutated();
            let ambush =
                "你剛想繞過蛛網邁向深處石台，盤踞窟頂的吞石巨蛛猛然垂落攔住去路！".to_string();
            push_breath(a, ambush.clone());
            return Ok(ambush);
        }
    }

    let mut valid_exit_dir: Option<String> = cur_node
        .exits
        .iter()
        .find(|e| e.to == target_node)
        .map(|e| e.dir.clone());

    let mut entering_secret = false;
    if valid_exit_dir.is_none()
        && let Some(hid) = doc.hidden_entrances.get(&cur_node_id)
        && hid.target == target_node
    {
        let unlocked = delta
            .actors
            .get(&aid)
            .is_some_and(|a| a.unlocked_secrets.contains(&cur_node_id));
        if unlocked {
            valid_exit_dir = Some(hid.dir.clone());
            entering_secret = true;
        }
    }

    let Some(dst_node) = doc.nodes.get(target_node) else {
        return Err("前方雲霧深鎖，無路可通。".to_string());
    };

    let mut town_shortcut_nodes: Vec<String> = Vec::new();
    if valid_exit_dir.is_none()
        && cur_node.cell_id == dst_node.cell_id
        && !cur_node.dungeon
        && !dst_node.dungeon
        && cur_node.kind != "wild"
        && dst_node.kind != "wild"
    {
        let mut q = VecDeque::new();
        let mut prev: BTreeMap<String, String> = BTreeMap::new();
        q.push_back((cur_node_id.clone(), 0usize));
        prev.insert(cur_node_id.clone(), String::new());
        while let Some((u, depth)) = q.pop_front() {
            if u == target_node {
                let mut cur = u;
                while let Some(p_id) = prev.get(&cur) {
                    if p_id.is_empty() {
                        break;
                    }
                    town_shortcut_nodes.push(cur.clone());
                    cur = p_id.clone();
                }
                town_shortcut_nodes.reverse();
                valid_exit_dir = Some("穿過街坊".to_string());
                break;
            }
            if depth >= 5 {
                continue;
            }
            if let Some(unode) = doc.nodes.get(&u) {
                for ex in &unode.exits {
                    if !prev.contains_key(&ex.to)
                        && let Some(nx) = doc.nodes.get(&ex.to)
                        && nx.cell_id == cur_node.cell_id
                        && !nx.dungeon
                        && nx.kind != "wild"
                    {
                        prev.insert(ex.to.clone(), u.clone());
                        q.push_back((ex.to.clone(), depth + 1));
                    }
                }
            }
        }
    }

    let Some(dir_label) = valid_exit_dir else {
        return Err("那個方向沒有可直接通行的道路或門戶。".to_string());
    };

    if entering_secret {
        // 透過 `node_occupants` $O(1)$ 通知同場全體見證者（不分玩家或居民！）
        if let Some(occ) = delta.node_occupants.get(&cur_node_id).cloned() {
            for wid in occ {
                if wid == aid {
                    continue;
                }
                if let Some(wa) = delta.actors.get_mut(&wid)
                    && wa.alive
                    && !wa.unlocked_secrets.contains(&cur_node_id)
                {
                    wa.unlocked_secrets.insert(cur_node_id.clone());
                    if let Some(hid) = doc.hidden_entrances.get(&cur_node_id) {
                        wa.mind.ltm.known_nodes.insert(hid.target.clone());
                    }
                    push_breath(
                        wa,
                        "你親眼看見旁人搬開斷橋下的鬆動楔石閃身入內，橋下石縫暗門的位置已清晰可見！",
                    );
                    wa.mark_mutated();
                }
            }
        }
        let pname = delta
            .actors
            .get(&aid)
            .map_or("旅人", |a| a.display_name.as_str())
            .to_string();
        broadcast_scene_event(
            doc,
            delta,
            clock,
            &cur_node_id,
            &aid,
            &pname,
            EventPublicity::PublicLoud,
            None,
            68,
            40,
            ValueAxis::CuriosityVenture,
            "open_bridge_secret",
            &format!("{}搬開灰松斷橋下的矮人鬆動楔石鑽入暗門", pname),
        );
    }

    for mid in &town_shortcut_nodes {
        if let Some(mnd) = doc.nodes.get(mid)
            && let Some(a) = delta.actors.get_mut(&aid)
        {
            let knows = mnd.kind == "plaza" || mnd.kind == "gate";
            a.record_step_visit(mid, mnd.cell_id, knows);
        }
    }

    let knows_dst_state = doc
        .cells
        .get(&dst_node.cell_id.to_string())
        .is_some_and(|c| {
            c.kind == "settlement" || dst_node.kind == "gate" || dst_node.kind == "plaza"
        });
    delta.relocate_actor(&aid, target_node, dst_node.cell_id, knows_dst_state);

    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    a.last_step_unix_ms = now_ms;
    a.last_active_unix = clock.real_unix;

    let carry_weight: u32 = a
        .backpack
        .iter()
        .map(|i| u32::from(i.weight_units) * i.count)
        .sum();
    if dst_node.kind == "wild" || dst_node.dungeon {
        let step_cost = if carry_weight >= 8 { 4 } else { 2 };
        a.fatigue = a.fatigue.saturating_add(step_cost).min(95);
    } else if a.fatigue > 0 && dst_node.kind == "plaza" {
        a.fatigue = a.fatigue.saturating_sub(1);
    }

    let clean_dir = dir_label.split('（').next().unwrap_or(&dir_label);
    let msg = if clean_dir == "穿過街坊" {
        format!("你穿過街坊石板路，來到【{}】。", dst_node.name)
    } else {
        format!("你往{}邁步，來到【{}】。", clean_dir, dst_node.name)
    };
    push_breath(a, msg.clone());
    Ok(msg)
}

/// 雙馬篷車乘行與中途下車推進（共用 `delta.transfer_copper` 與 `delta.relocate_actor`）
pub fn apply_carriage_action(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    node: &SpatialNodeDef,
    coach_id: Option<&str>,
    action: &str,
) -> Result<String, String> {
    let aid = delta.resolve_actor_id(actor_key).ok_or("找不到旅人行蹤")?;
    match action {
        "board_carriage_west" | "board_carriage_east" => {
            if !node.name.contains("驛馬") && !node.name.contains("車馬") && node.kind != "plaza"
            {
                return Err("請在驛馬行或市集廣場搭乘雙馬篷車。".to_string());
            }
            let dest = if action == "board_carriage_west" {
                "oi:west_strait_ferry:plaza"
            } else {
                "oi:white_gull_harbor:plaza"
            };
            let start_plaza = if node.indoor {
                node.exits
                    .first()
                    .map(|e| e.to.clone())
                    .unwrap_or_else(|| node.id.clone())
            } else {
                node.id.clone()
            };
            if start_plaza == dest {
                return Err("你已經在這一端的終點站了。".to_string());
            }
            if delta.actors.get(&aid).map_or(0, |a| a.copper) < 3 {
                return Err("搭乘驛馬車專線需要 3 枚銅幣車資。".to_string());
            }
            let Some(route) = shortest_node_path(doc, &start_plaza, dest) else {
                return Err("目前道路受阻，馬車暫停發班。".to_string());
            };
            let payee = coach_id.unwrap_or("municipal_treasury");
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "carriage_fare",
                &aid,
                payee,
                3,
                "搭乘雙馬篷車車資".to_string(),
            ) {
                return Err("車資結算受阻，請稍候再試。".to_string());
            }
            if let Some(cid) = coach_id
                && let Some(cname) = delta.actors.get(cid).map(|c| c.display_name.clone())
                && let Some(a) = delta.actors.get_mut(&aid)
            {
                a.mind.record_pact_or_trade_evidence(
                    cid,
                    &cname,
                    clock.game_days,
                    true,
                    false,
                    "搭乘雙馬篷車車資",
                );
            }
            let start_cell = doc
                .nodes
                .get(&start_plaza)
                .map_or(node.cell_id, |n| n.cell_id);
            delta.relocate_actor(&aid, &start_plaza, start_cell, true);
            let dest_label = if action == "board_carriage_west" {
                "西峽渡口"
            } else {
                "白鷗港"
            };
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.active_transit = Some(TransitState {
                line_name: format!("疾風雙馬篷車（往{}）", dest_label),
                route_nodes: route,
                step_idx: 0,
                started_unix: clock.real_unix,
            });
            a.mark_mutated();
            let msg = format!(
                "你付了 3 枚銅幣登上軸刻穩固紋路的雙馬篷車，馭手一抖長鞭，馬車順著大路穩穩駛向{}！",
                dest_label
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "carriage_next" => {
            let mut tr = {
                let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                a.active_transit
                    .take()
                    .ok_or_else(|| "你目前並未搭乘馬車。".to_string())?
            };
            let advance_to = (tr.step_idx + 5).min(tr.route_nodes.len().saturating_sub(1));
            for idx in (tr.step_idx + 1)..=advance_to {
                let nid = &tr.route_nodes[idx];
                if let Some(nd) = doc.nodes.get(nid) {
                    let knows = nd.kind == "plaza" || nd.kind == "gate";
                    delta.relocate_actor(&aid, nid, nd.cell_id, knows);
                }
            }
            tr.step_idx = advance_to;
            let cur_nid = tr.route_nodes[tr.step_idx].clone();
            let cur_name = doc
                .nodes
                .get(&cur_nid)
                .map_or_else(|| cur_nid.clone(), |n| n.name.clone());

            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            if tr.step_idx + 1 >= tr.route_nodes.len() {
                a.mark_mutated();
                let msg = format!(
                    "雙馬篷車緩緩停靠在終點【{}】，你拎好行囊跳下車轅，沿途大路風光已盡收眼底。",
                    cur_name
                );
                push_breath(a, msg.clone());
                Ok(msg)
            } else {
                let left = tr.route_nodes.len() - 1 - tr.step_idx;
                a.active_transit = Some(tr);
                a.mark_mutated();
                let msg = format!(
                    "車輪滾滾駛過【{}】（距終點尚有 {} 段路程），窗外山林飛掠而過。",
                    cur_name, left
                );
                push_breath(a, msg.clone());
                Ok(msg)
            }
        }
        _ => {
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            if a.active_transit.take().is_none() {
                return Err("你目前並未搭乘馬車。".to_string());
            }
            a.mark_mutated();
            let cur_name = doc
                .nodes
                .get(&a.node_id)
                .map_or_else(|| a.node_id.clone(), |n| n.name.clone());
            let msg = format!(
                "你拍了拍車廂木板示意停靠，在【{}】拎起行囊中途下車。",
                cur_name
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
    }
}

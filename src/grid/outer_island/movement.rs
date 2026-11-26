//! 全服單一 `UnifiedMindBody` 初抵入籍、背包小工具、語言耳濡目染、街道尋路與步履／篷車移動。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::clock_and_lang::{SolarClockSnapshot, language_name_zh};
use super::mind::{CognitiveMind, EventPublicity};
use super::persona::{CharacterDriftState, ValueAxis};
use super::sim_tick::broadcast_scene_event;
use super::types::{
    BoundTool, CarryItem, IslandWorldDelta, SkirmishState, SpatialNodeDef, UnifiedMindBody,
    WorldStaticDoc,
};

pub const MAX_BREATHS: usize = 8;

pub fn push_breath(a: &mut UnifiedMindBody, text: impl Into<String>) {
    let line = super::surface_cues::normalize_breath_line(&text.into());
    if line.is_empty() || a.recent_breaths.back().is_some_and(|prev| prev == &line) {
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
        birth_year: 306,
        household_bond: "外來旅人".to_string(),
        home_node: doc.spawn_node.clone(),
        work_node: doc.spawn_node.clone(),
        node_id: doc.spawn_node.clone(),
        fatigue: 8,
        bound_weapon: BoundTool {
            slug: "harbor_hammer".to_string(),
            name: "港務舊鍛錘".to_string(),
            provenance: "白鷗港港務局借予初抵學徒的粗鐵短錘，柄尾烙有港務火印".to_string(),
            grip_familiarity: 8,
            ..BoundTool::default()
        },
        languages: langs,
        known_state_cells,
        recent_breaths: VecDeque::new(),
        drift: CharacterDriftState::default(),
        mind,
        dirty: true,
        flushed_rev: 0,
        flushed_copper: i64::MIN,
        ..UnifiedMindBody::default()
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
    if item.vessel_meta.is_none()
        && let Some(existing) = list.iter_mut().find(|x| {
            x.vessel_meta.is_none()
                && x.slug == item.slug
                && x.bound_owner == item.bound_owner
                && x.stolen_from == item.stolen_from
        })
    {
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

/// 跨堆疊精確扣減「無記名借領且無贓物標記」之乾淨貨品；數量不足時零副作用回傳 `None`
pub fn take_clean_carry_items(
    list: &mut Vec<CarryItem>,
    slug: &str,
    need: u32,
) -> Option<CarryItem> {
    if need == 0 {
        return None;
    }
    let is_clean = |x: &CarryItem| {
        x.slug == slug && x.count > 0 && x.bound_owner.is_none() && x.stolen_from.is_none()
    };
    let avail: u32 = list.iter().filter(|x| is_clean(x)).map(|x| x.count).sum();
    if avail < need {
        return None;
    }
    let mut template: Option<CarryItem> = None;
    let mut rem = need;
    let mut idx = 0;
    while idx < list.len() && rem > 0 {
        if is_clean(&list[idx]) {
            if template.is_none() {
                let mut t = list[idx].clone();
                t.count = need;
                template = Some(t);
            }
            let take_here = list[idx].count.min(rem);
            rem -= take_here;
            if list[idx].count == take_here {
                list.remove(idx);
                continue;
            }
            list[idx].count -= take_here;
        }
        idx += 1;
    }
    template
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
    if !entry.display_name.is_empty()
        && entry.display_name != clean_name
        && !entry
            .mind
            .ltm
            .mastery
            .past_aliases
            .contains(&entry.display_name)
    {
        if entry.mind.ltm.mastery.past_aliases.len() >= 8 {
            entry.mind.ltm.mastery.past_aliases.remove(0);
        }
        entry
            .mind
            .ltm
            .mastery
            .past_aliases
            .push(entry.display_name.clone());
    }
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
    let (role_zh, tool, starter_item) = match valid_pack {
        "forest_hunter" => (
            "山林獵戶",
            BoundTool {
                slug: "old_hunting_knife".to_string(),
                name: "舊鹿角獵刀".to_string(),
                provenance: "隨身帶下船的厚背短獵刀，刀鞘纏著防滑麻繩".to_string(),
                grip_familiarity: 10,
                ..BoundTool::default()
            },
            CarryItem {
                slug: "rye_bread".to_string(),
                name: "黑麥乾糧".to_string(),
                count: 2,
                weight_units: 1,
                ..CarryItem::default()
            },
        ),
        "fallen_peddler" => (
            "行腳商旅",
            BoundTool {
                slug: "iron_walking_rod".to_string(),
                name: "包鐵行商手杖".to_string(),
                provenance: "走南闖北防身撐路用的硬木包鐵長杖".to_string(),
                grip_familiarity: 6,
                ..BoundTool::default()
            },
            CarryItem {
                slug: "lodging_voucher".to_string(),
                name: "船東抵宿券".to_string(),
                count: 1,
                bound_owner: Some(canonical_aid.clone()),
                ..CarryItem::default()
            },
        ),
        "deck_sailor" => (
            "遠航水手",
            BoundTool {
                slug: "belaying_club".to_string(),
                name: "包銅繫纜短棍".to_string(),
                provenance: "甲板上敲緊纜樁與防身兩用的沉手硬木棍".to_string(),
                grip_familiarity: 10,
                ..BoundTool::default()
            },
            CarryItem {
                slug: "hemp_rope".to_string(),
                name: "粗麻繩".to_string(),
                count: 1,
                weight_units: 2,
                ..CarryItem::default()
            },
        ),
        _ => (
            "鍛鐵學徒",
            BoundTool {
                slug: "harbor_hammer".to_string(),
                name: "港務舊鍛錘".to_string(),
                provenance: "白鷗港港務局借予初抵學徒的粗鐵短錘，柄尾烙有港務火印".to_string(),
                grip_familiarity: 10,
                ..BoundTool::default()
            },
            CarryItem {
                slug: "lodging_voucher".to_string(),
                name: "船東抵宿券".to_string(),
                count: 1,
                bound_owner: Some(canonical_aid.clone()),
                ..CarryItem::default()
            },
        ),
    };
    let (arrival_line, intro) = super::live_tide::compose_arrival_breaths(clock, valid_pack);
    entry.role = role_zh.to_string();
    entry.bound_weapon = tool;
    add_carry_item(&mut entry.backpack, starter_item);
    entry.setup_done = true;
    entry.recent_breaths.clear();
    push_breath(entry, arrival_line);
    push_breath(entry, intro.clone());
    Ok(intro)
}

fn reconstruct_bfs_path(prev: &BTreeMap<String, String>, start: &str, goal: &str) -> Vec<String> {
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
    path
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
                    return Some(reconstruct_bfs_path(&prev, start, goal));
                }
                q.push_back(ex.to.clone());
            }
        }
    }
    None
}

pub fn find_hidden_entrance_for_rumor<'a>(
    doc: &'a WorldStaticDoc,
    rumor_key: &str,
) -> Option<(&'a str, &'a super::types::HiddenEntranceDef)> {
    doc.hidden_entrances
        .iter()
        .find(|(_, h)| h.rumor_key == rumor_key)
        .map(|(k, h)| (k.as_str(), h))
}

/// 供步行使用之真實街道與門戶尋路（允許出入起終點房門，地城內角色夜間可經 `secret_out` 返回地面）
pub fn shortest_walk_path(
    doc: &WorldStaticDoc,
    start: &str,
    goal: &str,
    known_nodes: &BTreeSet<String>,
) -> Option<Vec<String>> {
    if start == goal {
        return Some(vec![start.to_string()]);
    }
    let start_in_dungeon = doc.nodes.get(start).is_some_and(|n| n.dungeon);
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
            if !start_in_dungeon && (ex.kind == "dungeon" || ex.kind == "secret_out") {
                continue;
            }
            if vis.contains(&ex.to) {
                continue;
            }
            if ex.to != goal {
                if !start_in_dungeon && !known_nodes.is_empty() && !known_nodes.contains(&ex.to) {
                    continue;
                }
                if let Some(dst) = doc.nodes.get(&ex.to)
                    && ((!start_in_dungeon && dst.dungeon)
                        || dst.kind == "house"
                        || dst.kind == "house_vacant")
                {
                    continue;
                }
            }
            vis.insert(ex.to.clone());
            prev.insert(ex.to.clone(), cur.clone());
            if ex.to == goal {
                return Some(reconstruct_bfs_path(&prev, start, goal));
            }
            q.push_back(ex.to.clone());
        }
    }
    None
}

/// 供自主長途遠行、就醫、購屋與知情秘境探窟使用之完整門戶＋已解鎖暗門尋路
pub fn shortest_expedition_path(
    doc: &WorldStaticDoc,
    start: &str,
    goal: &str,
    unlocked_secrets: &BTreeSet<String>,
) -> Option<Vec<String>> {
    if start == goal {
        return Some(vec![start.to_string()]);
    }
    if !doc.nodes.contains_key(start) || !doc.nodes.contains_key(goal) {
        return None;
    }
    let goal_is_dng = doc.nodes.get(goal).is_some_and(|n| n.dungeon);
    let start_is_dng = doc.nodes.get(start).is_some_and(|n| n.dungeon);
    let mut prev: BTreeMap<String, String> = BTreeMap::new();
    let mut vis: BTreeSet<String> = BTreeSet::new();
    let mut q = VecDeque::new();
    vis.insert(start.to_string());
    q.push_back(start.to_string());

    while let Some(cur) = q.pop_front() {
        let Some(node) = doc.nodes.get(&cur) else {
            continue;
        };
        let mut nexts: Vec<(&str, &str)> = node
            .exits
            .iter()
            .map(|e| (e.to.as_str(), e.kind.as_str()))
            .collect();
        if unlocked_secrets.contains(&cur)
            && let Some(hid) = doc.hidden_entrances.get(&cur)
        {
            nexts.push((hid.target.as_str(), "secret_in"));
        }
        for (to_id, ekind) in nexts {
            if vis.contains(to_id) {
                continue;
            }
            if (ekind == "dungeon" || ekind == "secret_out") && !goal_is_dng && !start_is_dng {
                continue;
            }
            if to_id != goal
                && let Some(dst) = doc.nodes.get(to_id)
                && (dst.kind == "house" || dst.kind == "house_vacant")
            {
                continue;
            }
            vis.insert(to_id.to_string());
            prev.insert(to_id.to_string(), cur.clone());
            if to_id == goal {
                return Some(reconstruct_bfs_path(&prev, start, goal));
            }
            q.push_back(to_id.to_string());
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
            return Err("你正坐在車船座艙內趕路，若要改走山徑請先拍板中途下車。".to_string());
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
            let primordial_gone = delta.first_spider_slain.is_some();
            let (f_slug, f_name, ambush_msg) = if !primordial_gone {
                (
                    "stone_spider",
                    "背嵌原石的吞石巨蛛",
                    "你剛想繞過蛛網邁向深處石台，盤踞窟頂的吞石巨蛛猛然垂落攔住去路！",
                )
            } else {
                (
                    "cave_lizard",
                    "循荒氣佔巢的嗜石岩蜥",
                    "你剛邁向深處石台，空巢易主遷入舊巢的嗜石岩蜥猛然竄出攔住去路！",
                )
            };
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.active_skirmish = Some(SkirmishState {
                foe_slug: f_slug.to_string(),
                foe_name: f_name.to_string(),
                can_speak_goblin: false,
                foe_stagger: 0,
                player_guard: false,
                round_idx: 1,
            });
            a.mark_mutated();
            let ambush = ambush_msg.to_string();
            push_breath(a, ambush.clone());
            return Ok(ambush);
        }
    }

    let matched_exit: Option<super::types::NodeExitDef> =
        cur_node.exits.iter().find(|e| e.to == target_node).cloned();
    let mut valid_exit_dir: Option<String> = matched_exit.as_ref().map(|e| e.dir.clone());

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

    let is_town_fabric = |k: &str| {
        !matches!(
            k,
            "wild" | "road" | "campfire_nook" | "summit_outlook" | "plague_ruin" | "ruin"
        )
    };
    let mut town_shortcut_nodes: Vec<String> = Vec::new();
    if valid_exit_dir.is_none()
        && cur_node.cell_id == dst_node.cell_id
        && !cur_node.dungeon
        && !dst_node.dungeon
        && is_town_fabric(&cur_node.kind)
        && is_town_fabric(&dst_node.kind)
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
                        && is_town_fabric(&nx.kind)
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

    let mut gate_step_cost_opt: Option<(u8, Option<&'static str>)> = None;
    if let (Some(ex_def), Some(actor_ro)) = (matched_exit.as_ref(), delta.actors.get(&aid)) {
        gate_step_cost_opt = Some(super::mesh_expedition::evaluate_exit_env_gate(
            actor_ro, ex_def, dst_node, clock,
        )?);
    }

    if entering_secret {
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

    let from_nid = delta
        .actors
        .get(&aid)
        .map(|a| a.node_id.clone())
        .unwrap_or_default();
    let src_cell_id = doc
        .nodes
        .get(&from_nid)
        .map_or(dst_node.cell_id, |n| n.cell_id);
    if let (Some(actor_ro), Some(src_nd)) = (delta.actors.get(&aid), doc.nodes.get(&from_nid)) {
        super::transit_and_cargo::check_bulk_overland_move(doc, delta, actor_ro, src_nd, dst_node)?;
    }
    let knows_dst_state = doc
        .cells
        .get(&dst_node.cell_id.to_string())
        .is_some_and(|c| {
            c.kind == "settlement"
                || c.kind == "rural_settlement"
                || dst_node.kind == "gate"
                || dst_node.kind == "plaza"
                || dst_node.kind == "village_gate"
        });
    super::live_tide::relocate_actor_with_local_witness(
        doc,
        delta,
        clock,
        &aid,
        &from_nid,
        target_node,
        dst_node.cell_id,
        knows_dst_state,
    );

    if delta.actors.get(&aid).is_some_and(|a| {
        a.ongoing_activity
            .as_ref()
            .is_some_and(|st| st.kind == "dock_haul" && st.target_node == target_node)
    }) {
        let _ = super::ongoing_activity::settle_dock_haul_delivery(doc, delta, clock, &aid);
    }

    let flood_crossing = src_cell_id != dst_node.cell_id
        && super::live_land::is_river_crossing_impassable_for_wheels(delta, dst_node.cell_id);
    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    a.last_step_unix_ms = now_ms;
    if now_ms > 0 && a.controller_id.is_some() {
        a.last_active_unix = clock.real_unix;
    }

    let mut env_flavor_suffix = String::new();
    if let Some((gate_cost, note_opt)) = gate_step_cost_opt {
        if gate_cost > 0 {
            a.fatigue = a.fatigue.saturating_add(gate_cost).min(95);
        } else if a.fatigue > 0 && dst_node.kind == "plaza" {
            a.fatigue = a.fatigue.saturating_sub(1);
        }
        if let Some(note) = note_opt {
            env_flavor_suffix = format!(" {note}");
        }
    } else if dst_node.kind == "wild" || dst_node.dungeon {
        let carry_weight: u32 = a
            .backpack
            .iter()
            .map(|i| u32::from(i.weight_units) * i.count)
            .sum();
        let step_cost = if carry_weight >= 8 { 4 } else { 2 };
        a.fatigue = a.fatigue.saturating_add(step_cost).min(95);
    } else if a.fatigue > 0 && dst_node.kind == "plaza" {
        a.fatigue = a.fatigue.saturating_sub(1);
    }
    if flood_crossing {
        a.fatigue = a.fatigue.saturating_add(6).min(95);
        env_flavor_suffix.push_str(" 前方木橋受汛沖斷，你踩著齊膝濁水艱難涉過淺灘。");
    }

    let clean_dir = dir_label.split('（').next().unwrap_or(&dir_label);
    let msg = if clean_dir == "穿過街坊" {
        format!(
            "你穿過街坊石板路，來到【{}】。{}",
            dst_node.name, env_flavor_suffix
        )
    } else {
        format!(
            "你往{}邁步，來到【{}】。{}",
            clean_dir, dst_node.name, env_flavor_suffix
        )
    };
    let td = super::live_tide::derive_tide_clock_details(clock);
    push_breath(a, format!("〔{:02}:{:02}〕{msg}", td.hour, td.minute));
    Ok(msg)
}

/// 雙馬篷車與沿岸貨駁乘行推進（薄委託至 `transit_and_cargo` 實體逐拍載具引擎）
pub fn apply_carriage_action(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    node: &SpatialNodeDef,
    coach_id: Option<&str>,
    action: &str,
) -> Result<String, String> {
    super::transit_and_cargo::apply_carriage_action(
        doc, delta, clock, actor_key, node, coach_id, action, "",
    )
}

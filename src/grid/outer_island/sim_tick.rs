//! 全服單一 `UnifiedMindBody` 創世種子灌入、動態移入、$O(1)$ 現場廣播、逐節點邁步與快慢雙腦推進。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::clock_and_lang::SolarClockSnapshot;
use super::mind::{ActiveIntention, CognitiveMind, EventPublicity, EvidenceTier};
use super::movement::shortest_walk_path;
use super::persona::{
    InnatePersona, ResponseTendency, Situation16D, ValueAxis, derive_innate_persona,
};
use super::types::{
    BoundTool, ContractRecord, IslandWorldDelta, NewActorSpawnSpec, UnifiedMindBody, WorldStaticDoc,
};

/// 依營生建立隨身綁定器物（器物即職業，全員同構）
pub fn default_tool_for_role(role: &str, name: &str) -> BoundTool {
    let (slug, label) = if role.contains("鍛鐵") {
        ("smith_hammer", "厚背鍛鐵重錘")
    } else if role.contains("獵") {
        ("hunter_blade", "硬木柄獵刀")
    } else if role.contains("漁") {
        ("harpoon_net", "倒鉤鐵魚叉")
    } else if role.contains("草藥") {
        ("herb_sickle", "銅柄採藥彎鐮")
    } else if role.contains("石工") {
        ("stone_chisel", "開山厚鐵鑿")
    } else if role.contains("木匠") {
        ("carpenter_axe", "短柄手斧")
    } else if role.contains("皮匠") {
        ("tanner_knife", "裁革半月刀")
    } else if role.contains("衛兵") {
        ("guard_spear", "白蠟木鐵矛")
    } else if role.contains("苦力") {
        ("dock_hook", "卸貨鐵抓鉤")
    } else if role.contains("領航") {
        ("star_astrolabe", "黃銅觀星尺")
    } else if role.contains("酒館") {
        ("tavern_ladle", "橡木長柄杓")
    } else if role.contains("車行") {
        ("coach_whip", "牛皮長車鞭")
    } else if role.contains("農") {
        ("farm_hoe", "鐵口寬鋤")
    } else {
        ("daily_knife", "隨身短裁刀")
    };
    BoundTool {
        slug: slug.to_string(),
        name: label.to_string(),
        provenance: format!("{}慣用隨身傢伙", name),
        wear_stage: 0,
        grip_familiarity: 18,
        rune_mark: None,
    }
}

fn collect_cell_public_nodes(doc: &WorldStaticDoc) -> BTreeMap<u32, Vec<String>> {
    let mut cell_public_nodes: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for (nid, nd) in &doc.nodes {
        if !nd.dungeon && (!nd.indoor || nd.kind == "shop") {
            cell_public_nodes
                .entry(nd.cell_id)
                .or_default()
                .push(nid.clone());
        }
    }
    cell_public_nodes
}

/// 僅於世界尚未建世（`!delta.genesis_seeded`）時執行一次：將創世初生名冊轉化為獨立 `UnifiedMindBody`，此後執行期徹底與 `doc.residents` 脫鉤！
pub fn seed_genesis_actors_once(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    is_night: bool,
) {
    if delta.genesis_total_copper <= 0 {
        delta.genesis_total_copper = doc.ledger_genesis.total_conserved_copper;
    }
    if delta.genesis_seeded {
        if delta.node_occupants.is_empty() && !delta.actors.is_empty() {
            delta.rebuild_indexes();
        }
        return;
    }

    delta.rebuild_indexes();
    let cell_public_nodes = collect_cell_public_nodes(doc);

    // 預建立同戶家眷索引，讓同屋家人在創世時自然相識
    let mut household_members: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for r in &doc.residents {
        household_members
            .entry(r.household_id.clone())
            .or_default()
            .push((r.id.clone(), r.name.clone()));
    }

    for seed in &doc.residents {
        if delta.actors.contains_key(&seed.id) {
            continue;
        }
        let work_post = if doc
            .nodes
            .get(&seed.home_node)
            .is_some_and(|n| n.kind == "shop")
        {
            seed.home_node.clone()
        } else {
            seed.work_node.clone()
        };
        let start_node = if is_night {
            seed.home_node.clone()
        } else {
            work_post.clone()
        };
        let home_cell = doc.nodes.get(&seed.home_node).map_or(4616, |n| n.cell_id);
        let locals = cell_public_nodes
            .get(&home_cell)
            .map_or(&[][..], |v| v.as_slice());

        let mut mind = CognitiveMind::default();
        mind.seed_local_map_knowledge(
            &seed.home_node,
            &work_post,
            home_cell,
            locals.iter().map(String::as_str),
        );
        if seed.role.contains("車行") || seed.role.contains("領航") || seed.role.contains("獵")
        {
            for (nid, nd) in &doc.nodes {
                if !nd.dungeon && !nd.indoor {
                    mind.ltm.known_nodes.insert(nid.clone());
                    mind.ltm.known_cells.insert(nd.cell_id);
                }
            }
        }
        if let Some(fams) = household_members.get(&seed.household_id) {
            for (fid, fname) in fams {
                if fid != &seed.id {
                    mind.record_pact_or_trade_evidence(fid, fname, 0, true, false, "同屋家眷師徒");
                }
            }
        }

        let mut known_state = BTreeSet::new();
        known_state.insert(home_cell);

        let actor = UnifiedMindBody {
            actor_id: seed.id.clone(),
            controller_id: None,
            soul_seed: seed.soul_seed,
            rev: 1,
            alive: true,
            setup_done: true,
            display_name: seed.name.clone(),
            race: seed.race.clone(),
            role: seed.role.clone(),
            appearance: format!("常年在島上從事{}營生", seed.role),
            origin_pack: "island_native".to_string(),
            household_id: seed.household_id.clone(),
            home_node: seed.home_node.clone(),
            work_node: work_post,
            node_id: start_node,
            route_plan: VecDeque::new(),
            copper: seed.copper,
            fatigue: 0,
            wound_part: None,
            severe_wrist_warning: false,
            bound_weapon: default_tool_for_role(&seed.role, &seed.name),
            backpack: Vec::new(),
            inn_locker: Vec::new(),
            inn_lockers: BTreeMap::new(),
            inn_nights_left: 0,
            owned_house_node: None,
            rescue_pact_active: false,
            conversing_until_ms: 0,
            languages: seed.languages.clone(),
            lang_daily_gain: BTreeMap::new(),
            lang_gain_day: 0,
            known_state_cells: known_state,
            unlocked_secrets: BTreeSet::new(),
            known_rumors: BTreeSet::new(),
            explored_windows: BTreeSet::new(),
            recent_breaths: VecDeque::new(),
            active_skirmish: None,
            active_transit: None,
            last_step_unix_ms: 0,
            last_talk_unix_ms: 0,
            last_active_unix: 0,
            drift: Default::default(),
            mind,
            dirty: true,
            flushed_rev: 0,
            flushed_copper: i64::MIN,
        };
        delta.upsert_actor(actor);
    }

    delta.genesis_seeded = true;
}

/// 執行期免重啟批量接納新誕生／跨海移入之 `UnifiedMindBody`（隨身銅幣經 `mainland_clearing` 複式撥入，嚴守 `conservation_diff == 0`）
pub fn spawn_dynamic_actors(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    specs: Vec<NewActorSpawnSpec>,
    game_day: i64,
    bell_idx: u8,
) -> Vec<String> {
    let cell_public_nodes = collect_cell_public_nodes(doc);
    let mut created_ids = Vec::with_capacity(specs.len());

    for spec in specs {
        if delta.actors.contains_key(&spec.actor_id) {
            continue;
        }
        let home_cell = doc
            .nodes
            .get(&spec.home_node)
            .or_else(|| doc.nodes.get(&spec.start_node))
            .map_or(4616, |n| n.cell_id);
        let locals = cell_public_nodes
            .get(&home_cell)
            .map_or(&[][..], |v| v.as_slice());

        let mut mind = CognitiveMind::default();
        mind.seed_local_map_knowledge(
            &spec.home_node,
            &spec.work_node,
            home_cell,
            locals.iter().map(String::as_str),
        );
        if let Some(sn) = doc.nodes.get(&spec.start_node) {
            mind.record_footstep(&spec.start_node, sn.cell_id);
        }

        let mut known_state = BTreeSet::new();
        known_state.insert(home_cell);

        let actor = UnifiedMindBody {
            actor_id: spec.actor_id.clone(),
            controller_id: spec.controller_id,
            soul_seed: spec.preferred_soul_seed.unwrap_or(0),
            rev: 1,
            alive: true,
            setup_done: true,
            display_name: spec.display_name.clone(),
            race: spec.race,
            role: spec.role,
            appearance: spec.appearance,
            origin_pack: "mainland_migrant".to_string(),
            household_id: spec.household_id,
            home_node: spec.home_node,
            work_node: spec.work_node,
            node_id: spec.start_node,
            route_plan: VecDeque::new(),
            copper: 0,
            bound_weapon: spec.bound_weapon,
            languages: spec.languages,
            known_state_cells: known_state,
            mind,
            dirty: true,
            ..UnifiedMindBody::default()
        };
        let aid = actor.actor_id.clone();
        delta.upsert_actor(actor);
        if spec.initial_copper_from_mainland > 0 {
            let _ = delta.transfer_copper(
                game_day,
                bell_idx,
                "mainland_arrival",
                "mainland_clearing",
                &aid,
                spec.initial_copper_from_mainland,
                format!("{}攜隨身盤纏登島入籍", spec.display_name),
            );
        }
        created_ids.push(aid);
    }
    created_ids
}

/// 透過 `node_occupants` $O(1)$ 索引向同場全體清醒 `UnifiedMindBody`（不分玩家或居民）廣播感官事件！
#[expect(
    clippy::too_many_arguments,
    reason = "explicit scene witness broadcast context"
)]
pub fn broadcast_scene_event(
    _doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    node_id: &str,
    actor_key: &str,
    actor_name: &str,
    publicity: EventPublicity,
    spoken_lang: Option<&str>,
    salience: u8,
    affect_charge: u8,
    touched_axis: ValueAxis,
    event_key_suffix: &str,
    summary_zh: &str,
) {
    let subject_aid = delta
        .resolve_actor_id(actor_key)
        .unwrap_or_else(|| actor_key.to_string());
    let ev_id = format!(
        "ev:{}:{}:{}:{}",
        clock.game_days, clock.bell_index, subject_aid, event_key_suffix
    );

    let Some(occupant_ids) = delta.node_occupants.get(node_id).cloned() else {
        return;
    };

    for oid in occupant_ids {
        let Some(observer) = delta.actors.get_mut(&oid) else {
            continue;
        };
        if !observer.alive || !observer.setup_done || observer.node_id != node_id {
            continue;
        }
        let is_part = oid == subject_aid;
        let langs = observer.languages.clone();
        if observer.mind.ingest_sensory_event(
            &langs,
            publicity,
            spoken_lang,
            is_part,
            &ev_id,
            clock.game_days,
            clock.bell_index,
            node_id,
            &subject_aid,
            actor_name,
            EvidenceTier::DirectWitness,
            salience,
            affect_charge,
            touched_axis,
            summary_zh,
        ) {
            observer.mark_mutated();
        }
    }
}

fn find_town_tavern_and_keeper(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    cell_id: u32,
) -> (Option<String>, Option<String>) {
    let tav_node = doc
        .nodes
        .values()
        .find(|n| {
            n.cell_id == cell_id
                && !n.dungeon
                && (n.name.contains("酒館") || n.name.contains("客棧") || n.kind == "tavern")
        })
        .or_else(|| {
            doc.nodes
                .values()
                .find(|n| n.cell_id == cell_id && !n.dungeon && n.kind == "plaza")
        })
        .map(|n| n.id.clone());

    // 先由該酒館節點之 `node_occupants` $O(1)$ 找在場掌櫃，其次僅查同聚落節點之 `node_occupants`（零全服 $O(N)$ 掃描）
    let keeper_id = tav_node
        .as_ref()
        .and_then(|tn| delta.node_occupants.get(tn))
        .and_then(|occ| {
            occ.iter().find_map(|aid| {
                let a = delta.actors.get(aid)?;
                if a.alive && (a.role.contains("酒館") || a.role.contains("掌櫃")) {
                    Some(aid.clone())
                } else {
                    None
                }
            })
        })
        .or_else(|| {
            doc.nodes
                .iter()
                .filter(|(_, n)| n.cell_id == cell_id)
                .filter_map(|(nid, _)| delta.node_occupants.get(nid))
                .flat_map(|occ| occ.iter())
                .find_map(|aid| {
                    let a = delta.actors.get(aid)?;
                    if a.alive && (a.role.contains("酒館") || a.role.contains("掌櫃")) {
                        Some(aid.clone())
                    } else {
                        None
                    }
                })
        });

    (tav_node, keeper_id)
}

/// Dave Mark 多候選效用評估器：依個體 `骨底脾氣 × CAPS 條件傾向 × 核心價值 × 飢疲餘額` 動態決定當下意圖
fn evaluate_daytime_utility<'a>(
    a: &'a UnifiedMindBody,
    innate: &InnatePersona,
    tav_opt: Option<&'a str>,
    bell_index: u8,
) -> (&'static str, String, &'static str) {
    let hunger = i32::from(a.mind.stm.affect.hunger);
    let fatigue = i32::from(a.mind.stm.affect.fatigue.max(a.fatigue));
    let sit = Situation16D::new(
        0,
        0,
        15,
        0,
        if a.copper < 4 { -25 } else { 15 },
        hunger - 35,
        fatigue - 35,
        if a.copper < 4 { 40 } else { 0 },
        if bell_index >= 7 { 25 } else { 5 },
        0,
        0,
        0,
        hunger,
        fatigue,
        10,
        10,
    );
    let tend = innate.choose_dominant_tendency(&sit, &a.drift);
    let top_val = innate.top_value_axis(&a.drift);

    let mut work_score = 48
        + i32::from(innate.temperament.persistence) / 2
        + if matches!(top_val, ValueAxis::CraftPride | ValueAxis::TangibleGain) {
            16
        } else {
            0
        }
        + if a.mind.stm.active_goal_tag == "frugal_clear_debt" {
            22
        } else {
            0
        };
    if a.copper < 4 {
        work_score += 18;
    }

    let meal_score = if tav_opt.is_some() {
        hunger * 2 - 42
            + if matches!(tend, ResponseTendency::Bargain | ResponseTendency::SeekAlly) {
                8
            } else {
                0
            }
    } else {
        i32::MIN / 2
    };

    let social_score = if tav_opt.is_some() && a.copper >= 6 {
        22 + i32::from(innate.temperament.approach_avoid) / 2
            + if bell_index >= 7 { 24 } else { -12 }
            + if matches!(
                top_val,
                ValueAxis::KinshipLoyalty | ValueAxis::CuriosityVenture
            ) {
                12
            } else {
                0
            }
    } else {
        i32::MIN / 2
    };

    let rest_score = fatigue * 2 - 90
        + if tend == ResponseTendency::Withdraw {
            14
        } else {
            0
        };

    if meal_score >= work_score && meal_score >= social_score && meal_score >= rest_score {
        if let Some(tn) = tav_opt {
            return ("buy_meal", tn.to_string(), "往酒館吃口熱食");
        }
    }
    if rest_score > work_score && rest_score > social_score {
        let bed = a
            .owned_house_node
            .clone()
            .unwrap_or_else(|| a.home_node.clone());
        return ("go_home_sleep", bed, "回屋歇息養神");
    }
    if social_score > work_score
        && let Some(tn) = tav_opt
    {
        return ("tavern_rest", tn.to_string(), "往酒館歇腳打聽街坊風聲");
    }
    ("walk_work", a.work_node.clone(), "前往當值處幹活")
}

/// 全服單一規則推進迴圈：對全體 `UnifiedMindBody` 一視同仁推進步履、生計、睡眠記憶整合與語言遮罩傳聞
pub fn tick_outer_island_world(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    now_unix_ms: u64,
) {
    seed_genesis_actors_once(doc, delta, clock.is_night);

    let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
    let new_bell_arrived = abs_bell > delta.last_settled_bell;
    if new_bell_arrived {
        delta.last_settled_bell = abs_bell;
    }

    // 1. 推進所有在途 `UnifiedMindBody` 沿真實街道門戶邁出最多 2 步（對話駐足或玩家即時手操期間不搶步）
    let walking_ids: Vec<String> = delta
        .actors
        .iter()
        .filter(|(_, a)| {
            a.alive
                && a.setup_done
                && !a.route_plan.is_empty()
                && !(now_unix_ms > 0 && a.conversing_until_ms > now_unix_ms)
                && !(a.controller_id.is_some() && clock.real_unix - a.last_active_unix < 1800)
        })
        .map(|(id, _)| id.clone())
        .collect();

    for aid in walking_ids {
        for _ in 0..2 {
            let next_opt = delta
                .actors
                .get_mut(&aid)
                .and_then(|a| a.route_plan.pop_front());
            if let Some(next_nid) = next_opt
                && let Some(nd) = doc.nodes.get(&next_nid)
            {
                let knows_state = nd.kind == "plaza" || nd.kind == "gate";
                delta.relocate_actor(&aid, &next_nid, nd.cell_id, knows_state);
            } else {
                break;
            }
        }
    }

    if !new_bell_arrived {
        return;
    }

    // 預快取各聚落酒館與掌櫃（僅數個聚落，避免在萬人迴圈內重複查找）
    let mut cell_tavern_cache: BTreeMap<u32, (Option<String>, Option<String>)> = BTreeMap::new();
    for c in doc.cells.values() {
        if c.kind == "settlement" {
            cell_tavern_cache.insert(
                c.cell_id,
                find_town_tavern_and_keeper(doc, delta, c.cell_id),
            );
        }
    }

    // 2. 新響鐘抵達：全體非在線手操之 `UnifiedMindBody`（含全體居民＋離線安寢之玩家角色）共用同一套生理、生計與睡眠整合規則！
    let all_ids: Vec<String> = delta.actors.keys().cloned().collect();
    for aid in &all_ids {
        let mut pay_innkeeper: Option<(String, i64, String)> = None;
        let mut open_meal_tab: Option<(String, i64, String)> = None;
        let mut municipal_wage: Option<(i64, String)> = None;

        if let Some(a) = delta.actors.get_mut(aid) {
            if !a.alive || !a.setup_done {
                continue;
            }
            let online_human =
                a.controller_id.is_some() && clock.real_unix - a.last_active_unix < 1800;
            if online_human {
                if !clock.is_night {
                    a.mind.stm.affect.hunger = (a.mind.stm.affect.hunger.saturating_add(3)).min(95);
                    a.mark_mutated();
                }
                continue;
            }

            let mut mutated = false;
            let innate = derive_innate_persona(a.soul_seed);
            if a.mind.pending_slow_think_score > 0 {
                a.mind.slow_think_wait_bells = a.mind.slow_think_wait_bells.saturating_add(1);
                mutated = true;
            }

            if !clock.is_night {
                a.mind.stm.affect.hunger = (a.mind.stm.affect.hunger.saturating_add(9)).min(95);
                a.mind.stm.affect.fatigue = (a.mind.stm.affect.fatigue.saturating_add(6)).min(95);
                mutated = true;
            }

            let sleep_home = a
                .owned_house_node
                .clone()
                .unwrap_or_else(|| a.home_node.clone());
            let home_cell = doc.nodes.get(&sleep_home).map_or(4616, |n| n.cell_id);
            let (tav_opt, ik_opt) = cell_tavern_cache
                .get(&home_cell)
                .cloned()
                .unwrap_or((None, None));

            let target_opt: Option<(&str, String, &str)> = if clock.is_night {
                let in_bed = a.node_id == sleep_home
                    || (doc
                        .nodes
                        .get(&a.node_id)
                        .is_some_and(|n| n.kind == "inn_room")
                        && a.inn_nights_left > 0);
                if in_bed {
                    let stagger_ok = (a.soul_seed % 2) == u64::from(clock.bell_index % 2)
                        || clock.bell_index >= 10;
                    if stagger_ok {
                        a.fatigue = 0;
                        let short = a.copper < 4;
                        a.mind.run_sleep_consolidation(
                            &innate,
                            &mut a.drift,
                            clock.game_days,
                            short,
                        );
                        mutated = true;
                    }
                    None
                } else {
                    Some(("go_home_sleep", sleep_home, "收工回屋歇息"))
                }
            } else {
                let (chosen_kind, chosen_dst, chosen_reason) =
                    evaluate_daytime_utility(a, &innate, tav_opt.as_deref(), clock.bell_index);
                if chosen_kind == "buy_meal" && a.node_id == chosen_dst {
                    if let Some(ref ik_id) = ik_opt
                        && ik_id != aid
                    {
                        if a.copper >= 2 {
                            pay_innkeeper = Some((ik_id.clone(), 2, a.display_name.clone()));
                            a.mind.stm.affect.hunger = a.mind.stm.affect.hunger.saturating_sub(48);
                            a.mind.stm.today_goal_progressed = true;
                        } else {
                            let sit = Situation16D::new(
                                5, 0, 25, 0, -20, 45, 35, 30, 15, 0, 0, 0, 50, 15, 10, 10,
                            );
                            let tend = innate.choose_dominant_tendency(&sit, &a.drift);
                            if matches!(
                                tend,
                                ResponseTendency::Bargain
                                    | ResponseTendency::SeekAlly
                                    | ResponseTendency::ComplyOrder
                            ) {
                                open_meal_tab = Some((ik_id.clone(), 2, a.display_name.clone()));
                                a.mind.stm.affect.hunger =
                                    a.mind.stm.affect.hunger.saturating_sub(42);
                            } else {
                                a.mind.stm.affect.hunger =
                                    a.mind.stm.affect.hunger.saturating_sub(18);
                            }
                        }
                    } else {
                        a.mind.stm.affect.hunger = a.mind.stm.affect.hunger.saturating_sub(40);
                    }
                    mutated = true;
                    None
                } else if a.node_id != chosen_dst {
                    Some((chosen_kind, chosen_dst, chosen_reason))
                } else {
                    a.mind.stm.today_goal_progressed = true;
                    mutated = true;
                    if a.controller_id.is_none()
                        && matches!(
                            a.role.as_str(),
                            "石工" | "木匠" | "巡城衛兵" | "碼頭苦力" | "觀星領航員"
                        )
                        && clock.bell_index == 5
                    {
                        municipal_wage = Some((2, a.display_name.clone()));
                    }
                    None
                }
            };

            if let Some((kind, dst_nid, reason)) = target_opt
                && a.route_plan.is_empty()
                && a.node_id != dst_nid
                && a.mind.ltm.known_nodes.contains(&dst_nid)
                && let Some(full_path) =
                    shortest_walk_path(doc, &a.node_id, &dst_nid, &a.mind.ltm.known_nodes)
            {
                a.route_plan = full_path.into_iter().skip(1).collect();
                if a.mind.stm.intentions.len() >= 4 {
                    a.mind.stm.intentions.pop_front();
                }
                a.mind.stm.intentions.push_back(ActiveIntention {
                    kind: kind.to_string(),
                    target_node: dst_nid,
                    reason_zh: reason.to_string(),
                    created_bell: abs_bell,
                });
                mutated = true;
            }

            if mutated {
                a.mark_mutated();
            }
        }

        if let Some((ik_id, amt, dname)) = pay_innkeeper {
            let _ = delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "npc_meal",
                aid,
                &ik_id,
                amt,
                format!("{}向酒館買熱食", dname),
            );
        }

        if let Some((ik_id, amt, dname)) = open_meal_tab {
            let cid = format!("tab:{}:{}:{}", aid, ik_id, clock.game_days);
            delta.dirty_contracts.insert(cid.clone());
            delta
                .contracts
                .entry(cid.clone())
                .and_modify(|c| c.amount_copper += amt)
                .or_insert_with(|| ContractRecord {
                    contract_id: cid,
                    kind: "meal_tab".to_string(),
                    creditor_id: ik_id,
                    debtor_id: aid.clone(),
                    amount_copper: amt,
                    due_day: clock.game_days + 3,
                    status: "open".to_string(),
                    note_zh: format!("{}在酒館記下的熱食飯帳", dname),
                });
        }

        if let Some((wage, dname)) = municipal_wage
            && delta.municipal_treasury_copper > 20_000 + wage
        {
            let _ = delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "municipal_wage",
                "municipal_treasury",
                aid,
                wage,
                format!("港務公庫發放{}修繕當值薪餉", dname),
            );
        }
    }

    settle_due_contracts(doc, delta, clock);
    propagate_gossip_o1(delta, clock);
}

fn settle_due_contracts(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    let due_keys: Vec<String> = delta
        .contracts
        .iter()
        .filter(|(_, c)| {
            (c.status == "open" || c.status == "defaulted") && clock.game_days >= c.due_day
        })
        .map(|(k, _)| k.clone())
        .collect();

    for ckey in due_keys {
        let Some(c) = delta.contracts.get(&ckey).cloned() else {
            continue;
        };
        let d_name = delta
            .actor_by_id_or_controller(&c.debtor_id)
            .map_or("欠債人", |a| a.display_name.as_str())
            .to_string();
        let c_name = delta
            .actor_by_id_or_controller(&c.creditor_id)
            .map_or("債主", |a| a.display_name.as_str())
            .to_string();

        if delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "contract_settle",
            &c.debtor_id,
            &c.creditor_id,
            c.amount_copper,
            format!("{d_name}如期結清欠{c_name}的帳款"),
        ) {
            if let Some(cmut) = delta.contracts.get_mut(&ckey) {
                cmut.status = "settled".to_string();
                delta.dirty_contracts.insert(ckey.clone());
            }
            if let Some(cr) = delta.actor_mut_by_id_or_controller(&c.creditor_id) {
                cr.mind.record_pact_or_trade_evidence(
                    &c.debtor_id,
                    &d_name,
                    clock.game_days,
                    true,
                    false,
                    "準時結清舊帳",
                );
                cr.mark_mutated();
            }
        } else {
            let strategy_note =
                if let Some(cr) = delta.actor_mut_by_id_or_controller(&c.creditor_id) {
                    let inn = derive_innate_persona(cr.soul_seed);
                    let sit = Situation16D::new(
                        15, 20, 20, -35, -30, 25, 40, 35, 45, 20, 0, 10, 20, 0, 25, 10,
                    );
                    let tend = inn.choose_dominant_tendency(&sit, &cr.drift);
                    let action_desc = match tend {
                        ResponseTendency::Confront => {
                            format!("{c_name}當街攔住{d_name}拍桌催討欠帳")
                        }
                        ResponseTendency::SeekAlly => {
                            format!("{c_name}在酒館向街坊公開點名{d_name}拖欠帳款")
                        }
                        ResponseTendency::Bargain | ResponseTendency::ComplyOrder => {
                            format!("{c_name}要求{d_name}以打雜勞力折抵逾期帳款")
                        }
                        ResponseTendency::ShelterWeak => {
                            format!("{c_name}嘆了口氣准許{d_name}寬限兩日再還")
                        }
                        _ => format!("{c_name}冷著臉把{d_name}記上拒賒黑冊"),
                    };
                    cr.mind.record_pact_or_trade_evidence(
                        &c.debtor_id,
                        &d_name,
                        clock.game_days,
                        false,
                        false,
                        &action_desc,
                    );
                    cr.mark_mutated();
                    Some((cr.node_id.clone(), action_desc))
                } else {
                    None
                };
            if let Some(cmut) = delta.contracts.get_mut(&ckey) {
                cmut.due_day = clock.game_days + 2;
                cmut.status = "defaulted".to_string();
                delta.dirty_contracts.insert(ckey.clone());
            }
            if let Some((nid, desc)) = strategy_note {
                broadcast_scene_event(
                    doc,
                    delta,
                    clock,
                    &nid,
                    &c.creditor_id,
                    &c_name,
                    EventPublicity::PublicLoud,
                    Some("harbor_trade"),
                    52,
                    45,
                    ValueAxis::PactIntegrity,
                    "debt_chase",
                    &desc,
                );
            }
        }
    }
}

/// 依 `node_occupants` $O(1)$ 空間分桶執行同場語言遮罩傳聞擴散（玩家與居民一視同仁）
fn propagate_gossip_o1(delta: &mut IslandWorldDelta, clock: &SolarClockSnapshot) {
    let crowded_nodes: Vec<(String, Vec<String>)> = delta
        .node_occupants
        .iter()
        .filter(|(_, set)| set.len() >= 2)
        .map(|(nid, set)| (nid.clone(), set.iter().cloned().collect()))
        .collect();

    for (nid, occupants) in crowded_nodes {
        let pick_idx = ((clock.game_days.unsigned_abs() * 7 + u64::from(clock.bell_index))
            as usize)
            % occupants.len();
        let speaker_id = &occupants[pick_idx];
        let Some(speaker) = delta.actors.get(speaker_id) else {
            continue;
        };
        if !speaker.alive || !speaker.setup_done {
            continue;
        }
        let Some(best_ep) = speaker
            .mind
            .ltm
            .episodic
            .iter()
            .filter(|e| clock.game_days - e.origin_day <= 12 && e.salience >= 48)
            .max_by_key(|e| e.retention_score(clock.game_days))
            .cloned()
        else {
            continue;
        };

        let sp_lang = match speaker.race.as_str() {
            "矮人" if nid.contains("forge") || nid.contains("smithy") => "dwarven",
            "精靈" if nid.contains("herb") || nid.contains("pine") => "elvish",
            "獸人" if nid.contains("dock") && (clock.bell_index % 2 == 1) => "orcish",
            _ => "harbor_trade",
        };
        let sp_name = speaker.display_name.clone();
        let core_text = best_ep
            .subjective_zh
            .split_once('：')
            .map_or(best_ep.subjective_zh.as_str(), |(_, rhs)| rhs);
        let hearsay_text = format!("聽{}提起：{}", sp_name, core_text);

        let start_offset = ((clock.game_days.unsigned_abs() * 3 + u64::from(clock.bell_index))
            as usize)
            % occupants.len();
        for step in 0..occupants.len().min(4) {
            let listener_id = &occupants[(start_offset + step) % occupants.len()];
            if listener_id == speaker_id {
                continue;
            }
            if let Some(listener) = delta.actors.get_mut(listener_id) {
                if !listener.alive || !listener.setup_done {
                    continue;
                }
                let langs = listener.languages.clone();
                if listener.mind.ingest_sensory_event(
                    &langs,
                    EventPublicity::TableWhisper,
                    Some(sp_lang),
                    false,
                    &best_ep.event_id,
                    clock.game_days,
                    clock.bell_index,
                    &nid,
                    &best_ep.subject_actor_id,
                    &best_ep.subject_name,
                    EvidenceTier::TavernHearsay,
                    best_ep.salience.saturating_sub(8),
                    35,
                    ValueAxis::PactIntegrity,
                    &hearsay_text,
                ) {
                    listener.mark_mutated();
                }
            }
        }
    }
}

//! 全服單一 `UnifiedMindBody` 創世種子灌入、動態移入、$O(1)$ 現場廣播、逐節點邁步與快慢雙腦推進。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::appraisal::{ConcernCause, awaken_lived_concerns_and_intentions_once};
use super::clock_and_lang::SolarClockSnapshot;
use super::desire_effects::{
    execute_arrived_desire_effect, propagate_gossip_o1, settle_due_contracts,
};
use super::desire_sim::{BellDecisionIndex, evaluate_personalized_desire};
use super::household_economy::{
    ensure_household_larders_seeded_once, find_town_tavern_and_keeper, restore_tavern_stock_one,
    settle_daily_town_livelihood_and_levy, try_consume_household_meal,
    try_consume_tavern_stock_one,
};
use super::mind::{ActiveIntention, CognitiveMind, EventPublicity, EvidenceTier};
use super::movement::shortest_walk_path;
use super::persona::{ResponseTendency, Situation16D, ValueAxis, derive_innate_persona};
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
        uid: String::new(),
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
        super::live_tide::ensure_porter_indoor_homes(doc, delta);
        if super::live_commodities::ensure_shop_material_stocks_seeded(doc, delta) {
            delta.extras_dirty = true;
        }
        return;
    }

    delta.rebuild_indexes();
    let cell_public_nodes = collect_cell_public_nodes(doc);

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
            birth_year: 300,
            household_bond: String::new(),
            home_node: seed.home_node.clone(),
            work_node: work_post,
            node_id: start_node,
            copper: seed.copper,
            bound_weapon: default_tool_for_role(&seed.role, &seed.name),
            languages: seed.languages.clone(),
            known_state_cells: known_state,
            mind,
            dirty: true,
            flushed_rev: 0,
            flushed_copper: i64::MIN,
            ..UnifiedMindBody::default()
        };
        delta.upsert_actor(actor);
    }

    super::live_tide::ensure_porter_indoor_homes(doc, delta);
    if is_night {
        for pid in [
            "res:white_gull_harbor:dock_porter",
            "res:west_strait_ferry:dock_porter",
            "res:gull_tail_port:dock_porter",
            "res:north_cape_beacon:dock_porter",
        ] {
            if let Some(home) = delta.actors.get(pid).map(|a| a.home_node.clone())
                && let Some(nd) = doc.nodes.get(&home)
            {
                delta.relocate_actor(pid, &home, nd.cell_id, true);
            }
        }
    }
    super::harbor_history::apply_island_history_backfill(doc, delta);
    super::causal_arcs::apply_gazetteer_v3_upgrade(doc, delta);
    ensure_household_larders_seeded_once(delta);
    super::rural_migrate::apply_rural_v5_weave_and_init(doc, delta, 0);
    super::live_land::ensure_live_land_seeded(doc, delta, 0);
    if super::live_commodities::ensure_shop_material_stocks_seeded(doc, delta) {
        delta.extras_dirty = true;
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

        let mut actor = UnifiedMindBody {
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
            birth_year: (super::clock_and_lang::solar_year_and_season_of_day(game_day).0 - 26)
                .max(1),
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
        let _ =
            super::world_weave_bio::refresh_actor_biography_if_needed(doc, &mut actor, game_day);
        let aid = actor.actor_id.clone();
        delta.upsert_actor(actor);
        super::living_chronicler::record_cross_sea_arrival(doc, delta, game_day, bell_idx, &aid);
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
        let eff_salience = if observer.controller_id.is_none() && observer.work_node == node_id {
            salience.saturating_add(18).min(90)
        } else {
            salience
        };
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
            eff_salience,
            affect_charge,
            touched_axis,
            summary_zh,
        ) {
            let obs_inn = derive_innate_persona(observer.soul_seed.max(1));
            let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
            observer.mind.appraise_sensory_episode_auto(
                &obs_inn,
                &oid,
                &subject_aid,
                is_part,
                &ev_id,
                eff_salience,
                affect_charge,
                touched_axis,
                abs_bell,
            );
            if !is_part
                && observer.controller_id.is_some()
                && eff_salience >= 30
                && let Some(ep_txt) = observer
                    .mind
                    .stm
                    .day_episodes
                    .back()
                    .map(|e| e.summary_zh.clone())
            {
                let td = super::live_tide::derive_tide_clock_details(clock);
                super::movement::push_breath(
                    observer,
                    format!("〔{:02}:{:02}〕{ep_txt}", td.hour, td.minute),
                );
            }
            observer.mark_mutated();
        }
    }
}

/// 全服單一規則推進迴圈：對全體 `UnifiedMindBody` 一視同仁推進步履、生計、睡眠記憶整合與語言遮罩傳聞
pub fn tick_outer_island_world(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    now_unix_ms: u64,
) {
    seed_genesis_actors_once(doc, delta, clock.is_night);
    ensure_household_larders_seeded_once(delta);
    if super::live_commodities::ensure_shop_material_stocks_seeded(doc, delta) {
        delta.extras_dirty = true;
    }
    super::rural_migrate::ensure_rural_runtime_initialized(doc, delta, clock.game_days);
    super::transit_and_cargo::tick_embodied_transit_vehicles(doc, delta, clock);
    super::ongoing_activity::tick_ongoing_activities(doc, delta, clock);
    awaken_lived_concerns_and_intentions_once(doc, delta, clock);

    let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
    let start_bell = if delta.last_settled_bell <= 0 {
        abs_bell - 1
    } else {
        delta.last_settled_bell.max(abs_bell - 12)
    };

    // 1. 鐘響之間（`abs_bell <= start_bell`）每遊戲分鐘（2.5s）錯峰發起作息路線；整鐘邊界則交由下方共享動詞核優先決策（防覆寫探窟／護行！）
    if abs_bell <= start_bell {
        super::live_tide::step_staggered_minute_routines(doc, delta, clock, now_unix_ms);
    }
    super::unified_kernel::step_autonomous_skirmishes_and_transits(doc, delta, clock);
    let mut walking_pairs: Vec<(u64, String)> = delta
        .actors
        .iter()
        .filter(|(_, a)| {
            a.alive
                && a.setup_done
                && a.active_skirmish.is_none()
                && a.active_transit.is_none()
                && !a.route_plan.is_empty()
                && !(now_unix_ms > 0 && a.conversing_until_ms > now_unix_ms)
                && !super::live_tide::is_actor_online_protected(a, clock.real_unix)
        })
        .map(|(id, a)| {
            (
                crate::combat::wordplate::splitmix64(a.soul_seed ^ now_unix_ms),
                id.clone(),
            )
        })
        .collect();
    walking_pairs.sort_by_key(|(rk, _)| *rk);
    let walking_ids: Vec<String> = walking_pairs.into_iter().map(|(_, id)| id).collect();

    for aid in walking_ids {
        for _ in 0..2 {
            let (cur_nid, next_opt) = match delta.actors.get_mut(&aid) {
                Some(a)
                    if a.active_skirmish.is_none()
                        && a.active_transit.is_none()
                        && !super::live_tide::is_actor_online_protected(a, clock.real_unix) =>
                {
                    (a.node_id.clone(), a.route_plan.pop_front())
                }
                _ => break,
            };
            if let Some(next_nid) = next_opt
                && let Some(nd) = doc.nodes.get(&next_nid)
            {
                let cur_dng = doc.nodes.get(&cur_nid).is_some_and(|n| n.dungeon);
                if nd.dungeon || cur_dng || doc.hidden_entrances.contains_key(&cur_nid) {
                    let step_err =
                        super::movement::apply_move_step(doc, delta, clock, &aid, &next_nid, 0)
                            .is_err();
                    if step_err
                        || delta
                            .actors
                            .get(&aid)
                            .is_some_and(|a| a.active_skirmish.is_some())
                    {
                        if let Some(a) = delta.actors.get_mut(&aid) {
                            a.route_plan.clear();
                            a.mark_mutated();
                        }
                        break;
                    }
                } else {
                    if let (Some(actor_ro), Some(src_nd)) =
                        (delta.actors.get(&aid), doc.nodes.get(&cur_nid))
                        && super::transit_and_cargo::check_bulk_overland_move(
                            doc, delta, actor_ro, src_nd, nd,
                        )
                        .is_err()
                    {
                        if let Some(a) = delta.actors.get_mut(&aid) {
                            a.route_plan.clear();
                            a.mark_mutated();
                        }
                        break;
                    }
                    let knows_state = matches!(nd.kind.as_str(), "plaza" | "gate" | "village_gate");
                    super::live_tide::relocate_actor_with_local_witness(
                        doc,
                        delta,
                        clock,
                        &aid,
                        &cur_nid,
                        &next_nid,
                        nd.cell_id,
                        knows_state,
                    );
                }
            } else {
                break;
            }
        }
    }

    if abs_bell <= start_bell {
        return;
    }

    // 預快取各聚落酒館與掌櫃（僅數個聚落，避免在萬人迴圈內重複查找）
    let mut cell_tavern_cache: BTreeMap<u32, (Option<String>, Option<String>)> = BTreeMap::new();
    for c in doc.cells.values() {
        if matches!(c.kind.as_str(), "settlement" | "rural_settlement") {
            cell_tavern_cache.insert(
                c.cell_id,
                find_town_tavern_and_keeper(doc, delta, c.cell_id),
            );
        }
    }

    // 2. 若跨響鐘重啟或跳鐘，先逐鐘補齊中間漏失之城鎮生計稅收與到期契約結算（確保第 5/8 響帳本絕不漏結，且不產生幽靈夜間路線覆寫當下決策！）
    for missed_bell in (start_bell + 1)..abs_bell {
        let missed_clk =
            super::clock_and_lang::solar_clock_for_settled_bell(missed_bell, clock.real_unix);
        settle_daily_town_livelihood_and_levy(doc, delta, &missed_clk, &cell_tavern_cache);
        super::live_land::tick_daily_live_land(doc, delta, &missed_clk);
        settle_due_contracts(doc, delta, &missed_clk);
    }
    delta.last_settled_bell = abs_bell;

    let mut ranked_ids: Vec<(u64, String)> = delta
        .actors
        .iter()
        .map(|(id, a)| {
            (
                crate::combat::wordplate::splitmix64(a.soul_seed ^ (abs_bell as u64)),
                id.clone(),
            )
        })
        .collect();
    ranked_ids.sort_by_key(|(rk, _)| *rk);
    let all_ids: Vec<String> = ranked_ids.into_iter().map(|(_, id)| id).collect();
    let parity_claimed =
        super::unified_kernel::execute_player_parity_bell_actions(doc, delta, clock, &all_ids);
    let bell_idx_cache = BellDecisionIndex::build(delta);
    for aid in &all_ids {
        let mut pay_innkeeper: Option<(String, String, i64, String)> = None;
        let mut open_meal_tab: Option<(String, String, i64, String)> = None;
        let mut arrived_effect: Option<(
            &'static str,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = None;
        let mut night_eat_home = false;

        let larder_now = delta
            .actors
            .get(aid)
            .map_or(0, |a| bell_idx_cache.larder_of(a));
        let has_unpaid_debt = bell_idx_cache.debtors_with_unpaid_record.contains(aid);
        let co_rival_opt = delta.actors.get(aid).and_then(|a| {
            if a.controller_id.is_some() {
                return None;
            }
            let sleep_home = a
                .owned_house_node
                .as_deref()
                .unwrap_or(a.home_node.as_str());
            if a.node_id == sleep_home {
                return None;
            }
            let cur_stamp = (clock.game_days + 10_000).max(1);
            let occ = delta.node_occupants.get(&a.node_id)?;
            occ.iter()
                .find(|&oid| {
                    oid != aid
                        && delta.actors.get(oid).is_none_or(|o| {
                            (o.household_id.is_empty() || o.household_id != a.household_id)
                                && !(o.controller_id.is_some()
                                    && !super::live_tide::is_actor_online_protected(
                                        o,
                                        clock.real_unix,
                                    ))
                        })
                        && a.mind.ltm.social.get(oid).is_some_and(|s| {
                            !s.cohabiting
                                && s.warmth <= -22
                                && (s.last_confront_day == 0
                                    || cur_stamp >= s.last_confront_day + 4)
                        })
                })
                .cloned()
        });
        let desired_opt = delta.actors.get(aid).and_then(|a| {
            let online_human = super::live_tide::is_actor_online_protected(a, clock.real_unix);
            if !a.alive
                || !a.setup_done
                || a.active_skirmish.is_some()
                || a.active_transit.is_some()
                || online_human
                || clock.is_night
                || parity_claimed.contains(aid)
            {
                return None;
            }
            let sleep_home = a
                .owned_house_node
                .as_deref()
                .unwrap_or(a.home_node.as_str());
            let home_cell = doc.nodes.get(sleep_home).map_or(4616, |n| n.cell_id);
            let (tav_opt, ik_opt) = cell_tavern_cache
                .get(&home_cell)
                .map_or((None, None), |(t, k)| (t.as_deref(), k.as_deref()));
            if a.controller_id.is_some() {
                return None;
            }
            let innate = derive_innate_persona(a.soul_seed.max(1));
            Some(evaluate_personalized_desire(
                doc,
                delta,
                &bell_idx_cache,
                a,
                &innate,
                tav_opt,
                ik_opt,
                clock.bell_index,
                abs_bell,
            ))
        });

        let home_meal_price = if desired_opt.as_ref().is_some_and(|d| d.kind == "buy_meal") {
            delta
                .actors
                .get(aid)
                .and_then(|a| {
                    let sleep_home = a
                        .owned_house_node
                        .as_deref()
                        .unwrap_or(a.home_node.as_str());
                    let hc = doc.nodes.get(sleep_home).map_or(4616, |n| n.cell_id);
                    cell_tavern_cache.get(&hc).and_then(|(tav_opt, _)| {
                        tav_opt.as_deref().map(|tn| {
                            super::live_commodities::quote_tavern_ration_copper(doc, delta, hc, tn)
                        })
                    })
                })
                .unwrap_or(2)
        } else {
            2
        };

        let mut need_offline_shelter = false;
        if let Some(a) = delta.actors.get_mut(aid) {
            if !a.alive || !a.setup_done {
                continue;
            }
            let online_human = super::live_tide::is_actor_online_protected(a, clock.real_unix);
            if online_human || a.active_skirmish.is_some() || a.active_transit.is_some() {
                if online_human && !clock.is_night {
                    a.mind.stm.affect.hunger = (a.mind.stm.affect.hunger.saturating_add(3)).min(95);
                    a.mark_mutated();
                }
                continue;
            }

            let mut mutated = false;
            let innate = derive_innate_persona(a.soul_seed.max(1));
            if a.mind.pending_slow_think_score > 0 {
                a.mind.slow_think_wait_bells = a.mind.slow_think_wait_bells.saturating_add(1);
                mutated = true;
            }

            if !clock.is_night && a.controller_id.is_none() {
                a.mind.stm.affect.hunger = (a.mind.stm.affect.hunger.saturating_add(6)).min(95);
                a.mind.stm.affect.fatigue = (a.mind.stm.affect.fatigue.saturating_add(5)).min(95);
                a.mind.decay_concerns_and_sync_affect(&innate);
                if let Some(ref rid) = co_rival_opt {
                    let _ = a.mind.appraise_event(
                        &innate,
                        ConcernCause::RivalEncounter,
                        Some(rid),
                        None,
                        42,
                        abs_bell,
                    );
                }
                if a.bound_weapon.wear_stage >= 2 {
                    let _ = a.mind.appraise_event(
                        &innate,
                        ConcernCause::ToolDamaged,
                        None,
                        None,
                        36,
                        abs_bell,
                    );
                }
                a.mind.tick_goal_strategy_expiry(
                    clock.game_days,
                    has_unpaid_debt,
                    larder_now,
                    a.copper,
                    a.bound_weapon.wear_stage,
                    &innate,
                    abs_bell,
                );
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

            let target_opt: Option<(&'static str, String, String)> = if clock.is_night {
                let in_bed = super::live_tide::has_real_indoor_bed(doc, a, &sleep_home);
                if in_bed {
                    let stagger_ok = (a.soul_seed % 2) == u64::from(clock.bell_index % 2)
                        || clock.bell_index >= 10;
                    if stagger_ok {
                        super::live_tide::prune_stale_day_window_keys(
                            &mut a.explored_windows,
                            clock.game_days,
                        );
                        if a.controller_id.is_some() {
                            a.fatigue = a.fatigue.saturating_sub(12);
                            mutated = true;
                        } else {
                            if !a.actor_id.starts_with("pl:") && a.mind.stm.affect.hunger >= 36 {
                                night_eat_home = true;
                            }
                            a.fatigue = 0;
                            let short = a.copper < 4 || larder_now == 0;
                            a.mind.run_sleep_consolidation(
                                &innate,
                                &mut a.drift,
                                clock.game_days,
                                short,
                            );
                            let _ = super::world_weave_bio::refresh_actor_biography_if_needed(
                                doc,
                                a,
                                clock.game_days,
                            );
                            mutated = true;
                        }
                    }
                    None
                } else if a.controller_id.is_some() {
                    a.fatigue = a.fatigue.saturating_sub(6);
                    mutated = true;
                    need_offline_shelter = true;
                    None
                } else {
                    Some(("go_home_sleep", sleep_home, "收工回屋歇息".to_string()))
                }
            } else if let Some(dec) = desired_opt {
                if dec.kind == "buy_meal" && a.node_id == dec.target_node {
                    if let (Some(tn), Some(ik_id)) = (tav_opt.as_ref(), ik_opt.as_ref())
                        && ik_id != aid
                    {
                        if a.copper >= home_meal_price {
                            pay_innkeeper = Some((
                                ik_id.clone(),
                                tn.clone(),
                                home_meal_price,
                                a.display_name.clone(),
                            ));
                        } else if !has_unpaid_debt {
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
                                open_meal_tab = Some((
                                    ik_id.clone(),
                                    tn.clone(),
                                    home_meal_price,
                                    a.display_name.clone(),
                                ));
                            } else {
                                let _ = a.mind.appraise_event(
                                    &innate,
                                    ConcernCause::LivelihoodShortfall,
                                    None,
                                    None,
                                    48,
                                    abs_bell,
                                );
                            }
                        } else {
                            let _ = a.mind.appraise_event(
                                &innate,
                                ConcernCause::LivelihoodShortfall,
                                None,
                                None,
                                48,
                                abs_bell,
                            );
                        }
                    } else if a.controller_id.is_none() {
                        night_eat_home = true;
                    }
                    mutated = true;
                    None
                } else if a.node_id != dec.target_node {
                    if dec.kind == "forage_wild" {
                        a.mind.ltm.known_nodes.insert(dec.target_node.clone());
                    }
                    Some((dec.kind, dec.target_node, dec.reason_zh))
                } else {
                    if a.mind
                        .stm
                        .intentions
                        .back()
                        .is_none_or(|last| last.kind != dec.kind)
                    {
                        if a.mind.stm.intentions.len() >= 4 {
                            a.mind.stm.intentions.pop_front();
                        }
                        a.mind.stm.intentions.push_back(ActiveIntention {
                            kind: dec.kind.to_string(),
                            target_node: dec.target_node.clone(),
                            reason_zh: dec.reason_zh.clone(),
                            created_bell: abs_bell,
                        });
                    }
                    if dec.kind == "walk_work" {
                        a.mind.stm.today_goal_progressed = true;
                    } else {
                        arrived_effect =
                            Some((dec.kind, dec.focus_actor, tav_opt.clone(), ik_opt.clone()));
                    }
                    mutated = true;
                    None
                }
            } else {
                None
            };

            if let Some((kind, dst_nid, reason)) = target_opt
                && a.route_plan.is_empty()
                && a.node_id != dst_nid
                && a.mind.ltm.known_nodes.contains(&dst_nid)
                && let Some(full_path) =
                    shortest_walk_path(doc, &a.node_id, &dst_nid, &a.mind.ltm.known_nodes)
            {
                if kind == "detour_avoid" {
                    a.mind.stm.last_detour_bell = (abs_bell + 100_000).max(1);
                }
                a.route_plan = full_path.into_iter().skip(1).collect();
                if a.mind.stm.intentions.len() >= 4 {
                    a.mind.stm.intentions.pop_front();
                }
                a.mind.stm.intentions.push_back(ActiveIntention {
                    kind: kind.to_string(),
                    target_node: dst_nid,
                    reason_zh: reason,
                    created_bell: abs_bell,
                });
                mutated = true;
            }

            if mutated {
                a.mark_mutated();
            }
        }

        if need_offline_shelter {
            super::live_tide::assign_offline_shelter_route_if_needed(doc, delta, aid, abs_bell);
        }

        if night_eat_home {
            let _ = try_consume_household_meal(delta, aid, abs_bell);
        }

        if let Some((ekind, f_actor, t_opt, k_opt)) = arrived_effect {
            let _ = execute_arrived_desire_effect(
                doc,
                delta,
                clock,
                aid,
                ekind,
                f_actor.as_deref(),
                t_opt.as_deref(),
                k_opt.as_deref(),
                abs_bell,
            );
        }

        if let Some((ik_id, tn, amt, dname)) = pay_innkeeper
            && try_consume_tavern_stock_one(delta, &ik_id, &tn)
        {
            if delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "npc_meal",
                aid,
                &ik_id,
                amt,
                format!("{}向酒館買熱食", dname),
            ) {
                if let Some(a) = delta.actors.get_mut(aid) {
                    a.mind.stm.affect.hunger = a.mind.stm.affect.hunger.saturating_sub(48);
                    a.mind
                        .resolve_concerns_matching(ConcernCause::LivelihoodShortfall, None, 28);
                    a.mind.stm.today_goal_progressed = true;
                    a.mark_mutated();
                }
                broadcast_scene_event(
                    doc,
                    delta,
                    clock,
                    &tn,
                    aid,
                    &dname,
                    EventPublicity::PublicLoud,
                    Some("harbor_trade"),
                    32,
                    10,
                    ValueAxis::SurvivalSafety,
                    "npc_meal",
                    &format!("{dname}在酒館櫃前點了一份熱騰騰的魚麥熱食"),
                );
            } else {
                restore_tavern_stock_one(delta, &ik_id, &tn);
            }
        }

        if let Some((ik_id, tn, amt, dname)) = open_meal_tab
            && try_consume_tavern_stock_one(delta, &ik_id, &tn)
        {
            if let Some(a) = delta.actors.get_mut(aid) {
                a.mind.stm.affect.hunger = a.mind.stm.affect.hunger.saturating_sub(42);
                a.mind
                    .resolve_concerns_matching(ConcernCause::LivelihoodShortfall, None, 22);
                a.mark_mutated();
            }
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
    }

    settle_daily_town_livelihood_and_levy(doc, delta, clock, &cell_tavern_cache);
    super::live_land::tick_daily_live_land(doc, delta, clock);
    settle_due_contracts(doc, delta, clock);
    propagate_gossip_o1(delta, clock);
}

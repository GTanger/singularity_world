//! 世界居民迴圈：場所、路線與實體位置皆使用同一份 PlaceGraph 主資料。

mod activity;
mod decision;
mod journey;
mod occupancy;

use std::collections::HashMap;

use crate::db::set_entity_activity;
use crate::npc::IntentType;
use crate::store::get_store;
use crate::world::journey::EntityJourney;
use crate::world::place::PlaceGraph;

use self::activity::{
    action_delay_seconds, place_name, staying_activity, synthesize_moving_activity,
    synthesize_staying_activity,
};
use self::decision::{PlaceIntent, choose_place_intent};
use self::journey::{persist_continuity, persist_journey, restore_states};
use self::occupancy::{occupants_by_place, relocate_occupant};

const NPCS_PER_TICK: usize = 24;

#[derive(Debug, Clone, Default)]
pub struct NpcState {
    pub path: Vec<String>,
    pub destination_place_id: Option<String>,
    pub last_intent: Option<IntentType>,
    pub intent_reason: String,
    pub intent_started_at: i64,
    pub next_wake_at: i64,
    pub blocked_by: Option<String>,
    pub revision: i64,
}

#[derive(Debug, Clone)]
struct NpcSnapshot {
    place_id: String,
    current_activity: String,
}

struct AdvanceContext<'a> {
    graph: &'a PlaceGraph,
    now_unix: i64,
    hour: i32,
    game_day: i32,
    occupants: &'a mut HashMap<String, Vec<String>>,
}

pub struct GridNpcManager {
    /// 路徑是可重算快取；位置、當前活動與抵達結果由 PostgreSQL 保存。
    pub states: HashMap<String, NpcState>,
    cursor: usize,
}

impl Default for GridNpcManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GridNpcManager {
    pub fn new() -> Self {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        if let Some(pool) = crate::pg::pool() {
            match crate::pg::living_world::reconcile_existing_characters(&pool, now_unix) {
                Ok(report) => {
                    if report.corrected_all_gather_rows > 0
                        && let Some(store) = get_store()
                        && !store.write().load_entities_from_pg()
                    {
                        tracing::warn!(
                            "all-gather correction committed but entity cache reload failed"
                        );
                    }
                    tracing::info!(
                        "[living-world] characters={} autonomous={} continuity={} obligations={} relationships={} corrected_all_gather={} source_records={}",
                        report.character_count,
                        report.autonomous_count,
                        report.continuity_count,
                        report.obligations_from_assignments,
                        report.relationships_from_encounters,
                        report.corrected_all_gather_rows,
                        report.source_record_count
                    );
                }
                Err(error) => tracing::warn!("living-world reconciliation failed: {error}"),
            }
        }
        Self {
            states: restore_states(now_unix),
            cursor: 0,
        }
    }

    pub fn tick(&mut self, now_unix: i64, hour: i32, game_day: i32) -> Vec<GridMoveEvent> {
        let Some(store) = get_store() else {
            return Vec::new();
        };
        let (snapshots, graph) = {
            let store = store.read();
            (snapshot_npcs(&store), store.place_graph_snapshot())
        };
        if snapshots.is_empty() {
            self.states.clear();
            self.cursor = 0;
            return Vec::new();
        }
        self.states.retain(|id, _| snapshots.contains_key(id));
        let mut ids: Vec<String> = snapshots.keys().cloned().collect();
        ids.sort();
        for id in &ids {
            self.states.entry(id.clone()).or_insert_with(|| NpcState {
                next_wake_at: now_unix,
                intent_started_at: now_unix,
                ..NpcState::default()
            });
        }
        let batch = take_fair_due_batch(
            &ids,
            &self.states,
            &mut self.cursor,
            now_unix,
            NPCS_PER_TICK,
        );
        let mut occupants = occupants_by_place(&snapshots);
        batch
            .into_iter()
            .filter_map(|entity_id| {
                let snapshot = snapshots.get(&entity_id)?;
                self.advance_one(
                    &entity_id,
                    snapshot,
                    AdvanceContext {
                        graph: &graph,
                        now_unix,
                        hour,
                        game_day,
                        occupants: &mut occupants,
                    },
                )
            })
            .collect()
    }

    fn advance_one(
        &mut self,
        entity_id: &str,
        snapshot: &NpcSnapshot,
        context: AdvanceContext<'_>,
    ) -> Option<GridMoveEvent> {
        if crate::store::is_npc_paused_for_interaction(entity_id) {
            return None;
        }
        let AdvanceContext {
            graph,
            now_unix,
            hour,
            game_day,
            occupants,
        } = context;
        let state = self.states.entry(entity_id.to_string()).or_default();
        if state.next_wake_at > now_unix {
            return None;
        }
        if state.path.is_empty() {
            let intent = choose_place_intent(entity_id, &snapshot.place_id, graph, hour, now_unix);
            begin_intent(entity_id, snapshot, graph, state, intent, now_unix);
            if state.path.is_empty() {
                finish_intent(
                    entity_id,
                    &snapshot.place_id,
                    graph,
                    now_unix,
                    game_day,
                    state,
                    occupants,
                );
                return None;
            }
        }
        let next = state.path[0].clone();
        let Some(route) = graph.route_to(&snapshot.place_id, &next).cloned() else {
            state.path.clear();
            state.destination_place_id = None;
            state.blocked_by = Some("route_invalidated".to_string());
            state.next_wake_at = now_unix + 30;
            persist_journey(entity_id, state, "原路線已失效，等待重新決定", now_unix);
            persist_continuity(entity_id, state, now_unix);
            return None;
        };

        // 路線受阻檢查：高成本路線（travel_cost >= 4）在區域惡劣天候下可能受阻
        let is_weather_blocked = if route.travel_cost >= 4
            && matches!(
                state.last_intent,
                Some(IntentType::Trade | IntentType::Work | IntentType::Gather)
            ) {
            let target_region = graph
                .place(&next)
                .or_else(|| graph.place(&snapshot.place_id))
                .map(|p| p.zone.as_str())
                .unwrap_or("");
            let wet_weather = if !target_region.is_empty() {
                crate::db::get_region_climate(target_region)
                    .ok()
                    .flatten()
                    .map(|c| matches!(c.event_type.as_str(), "rain" | "plum_rain" | "snow"))
                    .unwrap_or(false)
            } else {
                false
            };
            let hash = (now_unix / 120) ^ stable_jitter(entity_id, 100);
            wet_weather && (hash % 100 < 5)
        } else {
            false
        };
        if is_weather_blocked {
            state.blocked_by = Some("route_weather_blocked".to_string());
            state.next_wake_at = now_unix + 60;
            let current_place_title = place_name(graph, &snapshot.place_id);
            if let Some(pool) = crate::pg::pool() {
                let _ = crate::pg::living_world::open_obligation(
                    &pool,
                    entity_id,
                    "stranded_cargo",
                    &format!("受阻於{current_place_title}的隨行貨擔"),
                    Some(now_unix + 7200),
                    &format!("route_weather_blocked:{current_place_title}"),
                    now_unix,
                );
            }
            let dest_name = state.destination_place_id.as_deref().map(|id| place_name(graph, id)).unwrap_or_default();
            let intent = state.last_intent.unwrap_or(IntentType::Wander);
            let activity = synthesize_moving_activity(intent, &dest_name, state.blocked_by.as_deref(), Some("stranded_cargo"));
            set_activity_if_changed(entity_id, &snapshot.current_activity, &activity);
            persist_journey(entity_id, state, "因風雨路滑受阻，暫停前進", now_unix);
            persist_continuity(entity_id, state, now_unix);
            return None;
        }

        let moved = {
            let journey = journey_after_committed_step(entity_id, state, &next, now_unix);
            let store = get_store()?;
            store
                .write()
                .move_entity_to_adjacent_place_with_journey(entity_id, &next, &journey)
                .ok()??
        };
        state.path.remove(0);
        state.next_wake_at = now_unix + travel_delay_seconds(moved.route.travel_cost);
        state.blocked_by = None;
        relocate_occupant(occupants, entity_id, &snapshot.place_id, &next);
        let intent = state.last_intent.unwrap_or(IntentType::Wander);
        let arrived = state.path.is_empty();
        let consequence = if arrived {
            finish_intent(entity_id, &next, graph, now_unix, game_day, state, occupants)
        } else {
            let dest_name = state.destination_place_id.as_deref().map(|id| place_name(graph, id)).unwrap_or_default();
            let activity = synthesize_moving_activity(intent, &dest_name, None, None);
            set_activity_if_changed(entity_id, &snapshot.current_activity, &activity);
            format!("沿{}路線繼續前往目的地", route.direction)
        };
        if !arrived {
            persist_journey(entity_id, state, &consequence, now_unix);
            persist_continuity(entity_id, state, now_unix);
        }

        let target_zone = graph.place(&next).map(|p| p.zone.as_str()).unwrap_or("wt-c-wenyao");
        let stepped_coords = get_store().and_then(|s| {
            s.read().step_npc_grid_coord(entity_id, &route.direction, target_zone, now_unix as u64)
        });
        let (from_room_id, to_room_id) = if let Some(((ox, oy), (nx, ny))) = stepped_coords {
            (
                crate::grid::grid_room_id_from_coord(ox, oy),
                crate::grid::grid_room_id_from_coord(nx, ny),
            )
        } else {
            (
                compatibility_room(graph, &snapshot.place_id),
                compatibility_room(graph, &next),
            )
        };

        Some(GridMoveEvent {
            entity_id: entity_id.to_string(),
            from_place_id: snapshot.place_id.clone(),
            to_place_id: next.clone(),
            from_room_id,
            to_room_id,
            intent,
            travel_cost: route.travel_cost,
            consequence,
            description: format!("移動至{}", place_name(graph, &next)),
            action_id: moved.action_id,
        })
    }
}

fn journey_after_committed_step(
    entity_id: &str,
    state: &NpcState,
    next_place_id: &str,
    now_unix: i64,
) -> EntityJourney {
    let planned_path = state.path.iter().skip(1).cloned().collect::<Vec<_>>();
    let arrived = planned_path.is_empty();
    EntityJourney {
        entity_id: entity_id.to_string(),
        reason: state
            .last_intent
            .unwrap_or(IntentType::Idle)
            .as_key()
            .to_string(),
        destination_place_id: if arrived {
            Some(next_place_id.to_string())
        } else {
            state.destination_place_id.clone()
        },
        planned_path,
        consequence: if arrived {
            "抵達目的地".to_string()
        } else {
            "沿路繼續前往目的地".to_string()
        },
        updated_at: now_unix,
    }
}

#[derive(Debug, Clone)]
pub struct GridMoveEvent {
    pub entity_id: String,
    pub from_place_id: String,
    pub to_place_id: String,
    pub from_room_id: String,
    pub to_room_id: String,
    pub intent: IntentType,
    pub travel_cost: i32,
    pub consequence: String,
    pub description: String,
    pub action_id: String,
}

fn snapshot_npcs(store: &crate::store::Store) -> HashMap<String, NpcSnapshot> {
    store
        .entities
        .values()
        .filter(|entity| entity.kind == "npc" && entity.vit > 0 && entity.id != "npc_m3_029_pack_runner")
        .filter_map(|entity| {
            let place_id = entity.place_id.clone()?;
            store
                .get_place(&place_id)
                .filter(|place| place.traversable)?;
            Some((
                entity.id.clone(),
                NpcSnapshot {
                    place_id,
                    current_activity: entity.current_activity.clone(),
                },
            ))
        })
        .collect()
}

fn take_fair_due_batch(
    ids: &[String],
    states: &HashMap<String, NpcState>,
    cursor: &mut usize,
    now_unix: i64,
    limit: usize,
) -> Vec<String> {
    if ids.is_empty() || limit == 0 {
        *cursor = 0;
        return Vec::new();
    }
    let start = *cursor % ids.len();
    let mut batch = Vec::with_capacity(limit.min(ids.len()));
    let mut scanned = 0;
    while scanned < ids.len() {
        let index = (start + scanned) % ids.len();
        let id = &ids[index];
        scanned += 1;
        if states
            .get(id)
            .is_some_and(|state| state.next_wake_at > now_unix)
        {
            continue;
        }
        batch.push(id.clone());
        if batch.len() == limit {
            *cursor = (index + 1) % ids.len();
            return batch;
        }
    }
    *cursor = start;
    batch
}

fn begin_intent(
    entity_id: &str,
    snapshot: &NpcSnapshot,
    graph: &PlaceGraph,
    state: &mut NpcState,
    intent: PlaceIntent,
    now_unix: i64,
) {
    state.last_intent = Some(intent.ty);
    state.intent_reason = intent.reason;
    state.intent_started_at = now_unix;
    state.blocked_by = None;
    state.destination_place_id = intent.target.clone();
    state.path = intent
        .target
        .filter(|target| target != &snapshot.place_id)
        .and_then(|target| graph.find_path(&snapshot.place_id, &target))
        .unwrap_or_default()
        .into_iter()
        .skip(1)
        .collect();
    if !state.path.is_empty() {
        let dest_name = state
            .destination_place_id
            .as_deref()
            .map(|id| place_name(graph, id))
            .unwrap_or_default();
        let active_ob = state
            .intent_reason
            .strip_prefix("obligation:")
            .map(str::to_string);
        let activity = crate::pg::pool()
            .and_then(|pool| {
                crate::pg::world_logistics::active_delivery_target(&pool, entity_id)
                    .ok()
                    .flatten()
            })
            .filter(|delivery| {
                state.destination_place_id.as_deref() == Some(&delivery.destination_place_id)
            })
            .map(|delivery| {
                let raw_dname = graph
                    .place(&delivery.destination_place_id)
                    .map(|p| p.player_title())
                    .filter(|n| !n.is_empty())
                    .unwrap_or("鄰近市集");
                let base_dname = crate::pg::living_world::peninsula_market_settlement_name(raw_dname);
                let dname = if base_dname != "鄰近市集" && !base_dname.ends_with("市集") {
                    format!("{base_dname}市集")
                } else {
                    base_dname.to_string()
                };
                let display_name = crate::pg::living_world::peninsula_good_display_name(&delivery.display_name);
                format!(
                    "正運送{}份{}前往{}",
                    delivery.quantity, display_name, dname
                )
            })
            .unwrap_or_else(|| {
                synthesize_moving_activity(
                    intent.ty,
                    &dest_name,
                    state.blocked_by.as_deref(),
                    active_ob.as_deref(),
                )
            });
        set_activity_if_changed(entity_id, &snapshot.current_activity, &activity);
    }
    let consequence = if state.path.is_empty() {
        "決定留在目前場所"
    } else {
        "決定前往新的目的地"
    };
    persist_journey(entity_id, state, consequence, now_unix);
    persist_continuity(entity_id, state, now_unix);
}

fn finish_intent(
    entity_id: &str,
    place_id: &str,
    graph: &PlaceGraph,
    now_unix: i64,
    game_day: i32,
    state: &mut NpcState,
    occupants: &HashMap<String, Vec<String>>,
) -> String {
    state.blocked_by = None;
    let intent = state.last_intent.unwrap_or(IntentType::Idle);

    // 1. 檢查 entity 的 soul_seed 並展開為 (amp, freq, phase)
    let soul_axes = crate::store::get_store()
        .and_then(|s| s.read().entities.get(entity_id).and_then(|e| e.soul_seed))
        .map(crate::db::expand_soul_seed_to_combat_axes);
    let amp = soul_axes.map(|(a, _, _)| a).unwrap_or(1.0);

    // 2. 當完成 Work 或 Gather 時，若決定性擲骰（受 amp 調變）觸發工具磨損 (~20% 機率)，開立 repair_tool 義務
    let mut current_obligation_kind: Option<String> = None;
    if matches!(intent, IntentType::Work | IntentType::Gather) {
        let wear_hash = (now_unix / 60) ^ stable_jitter(entity_id, 1000);
        // amp 越高（活動能量大），磨損機率略微提高，基礎約 3%
        let threshold = (30.0 * (amp / 1.0).clamp(0.5, 2.0)) as i64;
        if wear_hash % 1000 < threshold {
            if let Some(pool) = crate::pg::pool() {
                let current_place_title = place_name(graph, place_id);
                let _ = crate::pg::living_world::open_obligation(
                    &pool,
                    entity_id,
                    "repair_tool",
                    "勞作磨損待修整的隨身工具",
                    Some(now_unix + 3600),
                    &format!("work_wear:{current_place_title}"),
                    now_unix,
                );
            }
            current_obligation_kind = Some("repair_tool".to_string());
        }
    }

    // 3. 當在目的場所完成對應行動時，結清相應義務（含養傷完成結清 treat_injury）
    if let Some(pool) = crate::pg::pool() {
        if matches!(intent, IntentType::Trade | IntentType::Work)
            && (state.intent_reason.starts_with("obligation:")
                || state.intent_reason.contains("義務")
                || current_obligation_kind.is_none())
        {
            if let Ok(open_obs) = crate::pg::living_world::list_open_obligations_for_entity(&pool, entity_id) {
                let ent_mg = crate::db::get_entity(entity_id)
                    .ok()
                    .flatten()
                    .map(|e| e.magnesium)
                    .unwrap_or(0);
                for ob in open_obs {
                    if ob.obligation_kind == "repay_debt" {
                        continue;
                    }
                    if matches!(
                        ob.obligation_kind.as_str(),
                        "repair_tool" | "stranded_cargo" | "repay_debt" | "restock_goods" | "supply_shortage" | "craft_material" | "fuel_shortage" | "treat_injury"
                    ) {
                        match ob.obligation_kind.as_str() {
                            "restock_goods" => {
                                crate::server::action::trade::restock_shop_items_for_seller(entity_id);
                                if ent_mg >= 2 {
                                    let _ = crate::db::add_magnesium(entity_id, -2);
                                }
                            }
                            "repair_tool" => {
                                if ent_mg >= 2 {
                                    let _ = crate::db::add_magnesium(entity_id, -2);
                                }
                                if let Some(peer_id) = occupants.get(place_id).and_then(|ids| ids.iter().find(|p| p.as_str() != entity_id)) {
                                    let _ = crate::db::add_magnesium(peer_id, 2);
                                    let _ = crate::pg::living_world::record_relationship_delta(
                                        &pool,
                                        entity_id,
                                        peer_id,
                                        "traded",
                                        1,
                                        "repair_tool_settled",
                                        now_unix,
                                    );
                                }
                            }
                            "repay_debt" => {
                                if ent_mg >= 4 {
                                    let _ = crate::db::add_magnesium(entity_id, -4);
                                    if !ob.subject_ref.is_empty() {
                                        let _ = crate::db::add_magnesium(&ob.subject_ref, 4);
                                    }
                                }
                                if !ob.subject_ref.is_empty() {
                                    let _ = crate::pg::living_world::record_relationship_delta(
                                        &pool,
                                        entity_id,
                                        &ob.subject_ref,
                                        "helped",
                                        1,
                                        "repay_debt_settled",
                                        now_unix,
                                    );
                                }
                            }
                            "stranded_cargo" => {
                                let _ = crate::db::add_magnesium(entity_id, 4);
                            }
                            _ => {}
                        }
                        let ob_room_name = place_name(graph, place_id);
                        let _ = settle_obligation_venue_event_pack_note(&ob_room_name, &ob.obligation_kind, &ob.subject_ref);
                        crate::store::clear_active_npc_obligation(entity_id, &ob.obligation_kind);
                    }
                }
            }
        }
        if intent == IntentType::Idle {
            let _ = settle_obligation_venue_event_pack_note(&place_name(graph, place_id), "treat_injury", "");
            crate::store::clear_active_npc_obligation(entity_id, "treat_injury");
        }
    }
    let peer = occupants.get(place_id).and_then(|ids| {
        ids.iter().find(|peer| {
            let p = peer.as_str();
            p != entity_id
                && !p.starts_with("ambient_")
                && !p.starts_with("shanhai_")
                && (p.starts_with("npc_") || p.starts_with("wt_npc_"))
        })
    });
    let _ = intent;
    let arrival_activity = if intent != IntentType::Socialize || peer.is_some() {
        super::brain_arrival::apply_brain_arrival_effects(
            now_unix, game_day, entity_id, place_id, intent,
        )
    } else {
        None
    };
    let active_delivery = if intent == IntentType::Work {
        crate::pg::pool().and_then(|pool| {
            crate::pg::world_logistics::active_delivery_target(&pool, entity_id)
                .ok()
                .flatten()
        })
    } else {
        None
    };

    let place_display_name = crate::store::get_store()
        .and_then(|s| s.read().get_place(place_id).map(|p| crate::pg::living_world::peninsula_market_settlement_name(p.player_title()).to_string()))
        .unwrap_or_else(|| place_id.to_string());

    let synthesized_staying = synthesize_staying_activity(
        intent,
        &place_display_name,
        state.blocked_by.as_deref(),
        current_obligation_kind.as_deref(),
    );

    let (_raw_activity, mut consequence) = arrival_result(intent, peer, &place_display_name);
    let delivery_activity = active_delivery.as_ref().map(|delivery| {
        let raw_dname = crate::db::get_room_name(&delivery.destination_place_id)
            .ok()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "鄰近市集".to_string());
        let base_dname = crate::pg::living_world::peninsula_market_settlement_name(&raw_dname);
        let dname = if base_dname != "鄰近市集" && !base_dname.ends_with("市集") {
            format!("{base_dname}市集")
        } else {
            base_dname.to_string()
        };
        let display_name = crate::pg::living_world::peninsula_good_display_name(&delivery.display_name);
        format!(
            "正運送{}份{}前往{}",
            delivery.quantity, display_name, dname
        )
    });
    set_activity_if_changed(
        entity_id,
        "",
        arrival_activity
            .as_deref()
            .or(delivery_activity.as_deref())
            .unwrap_or(&synthesized_staying),
    );
    if let Some(delivery) = active_delivery {
        let raw_dname = crate::db::get_room_name(&delivery.destination_place_id)
            .ok()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "鄰近市集".to_string());
        let base_dname = crate::pg::living_world::peninsula_market_settlement_name(&raw_dname);
        let dname = if base_dname != "鄰近市集" && !base_dname.ends_with("市集") {
            format!("{base_dname}市集")
        } else {
            base_dname.to_string()
        };
        let display_name = crate::pg::living_world::peninsula_good_display_name(&delivery.display_name);
        state.destination_place_id = Some(delivery.destination_place_id.clone());
        state.intent_reason = format!(
            "已在供應地裝好{}份{}，正送往{}",
            delivery.quantity, display_name, dname
        );
        state.next_wake_at = now_unix;
        consequence = format!("完成來源差事並開始承運貨物前往{dname}");
    } else {
        state.next_wake_at = now_unix + action_delay_seconds(intent) + stable_jitter(entity_id, 17);
    }
    state.blocked_by = None;
    persist_journey(entity_id, state, &consequence, now_unix);
    persist_continuity(entity_id, state, now_unix);
    consequence
}

fn travel_delay_seconds(travel_cost: i32) -> i64 {
    i64::from(travel_cost.max(1)).saturating_mul(3)
}

fn stable_jitter(entity_id: &str, span: i64) -> i64 {
    if span <= 0 {
        return 0;
    }
    let hash = entity_id
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    (hash % span as u64) as i64
}

fn arrival_result(intent: IntentType, peer: Option<&String>, room_name: &str) -> (&'static str, String) {
    if intent == IntentType::Socialize && peer.is_none() {
        return ("在公共場所停留", "抵達，但現場沒有可交往的人".to_string());
    }
    let consequence = peer
        .map(|peer_id| format_venue_arrival_peer_consequence(room_name, peer_id))
        .unwrap_or_else(|| {
            let act = staying_activity(intent);
            let clean_act = act.strip_prefix("正在").unwrap_or(act);
            format!("抵達並開始{clean_act}")
        });
    (staying_activity(intent), consequence)
}

pub fn settle_obligation_venue_event_pack_note(
    room_name: &str,
    obligation_kind: &str,
    subject_ref: &str,
) -> Option<String> {
    let clean_room = room_name.trim();
    if clean_room.is_empty() {
        return None;
    }
    let hint = match obligation_kind.trim() {
        "repair_tool" => "工具 鐵砧 鍛鎚",
        "restock_goods" => "貨架 櫃檯 帳冊",
        "repay_debt" => "帳冊 櫃檯 銀錢",
        "stranded_cargo" => "貨擔 麻袋 推車",
        "treat_injury" => "藥草 繃帶 藥臼",
        _ => {
            if !subject_ref.trim().is_empty() {
                subject_ref.trim()
            } else {
                obligation_kind.trim()
            }
        }
    };
    let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(clean_room, hint);
    if let Some(venue) = crate::grid::resolve_any_room_venue_object(clean_room, "") {
        if let Some((_, tool_name, _)) = venue.lendable_tool {
            let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(clean_room, tool_name);
        }
        let zone_tag = if venue.has_counter_restricted_zone {
            "當值櫃檯內側"
        } else {
            "當值作業區"
        };
        return Some(format!(
            "於{}{}由{}辦妥「{}」",
            venue.venue_kind,
            zone_tag,
            venue.default_host_title,
            obligation_kind.trim()
        ));
    }
    None
}

pub fn format_venue_arrival_peer_consequence(room_name: &str, peer_id: &str) -> String {
    let peer_name = crate::store::resolve_npc_person_name(peer_id, "", None);
    let (is_staff, has_uniform, in_counter_zone) =
        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            room_name.trim(),
            peer_id,
            "",
            "",
            "",
        );
    if let Some(venue) = crate::grid::resolve_any_room_venue_object(room_name.trim(), "") {
        if is_staff && (has_uniform || in_counter_zone) {
            let zone_tag = if in_counter_zone {
                "櫃檯內場的"
            } else {
                "身著工作服的"
            };
            if !peer_name.is_empty() && !peer_name.starts_with("npc_") && peer_name != "陌生人" {
                return format!("抵達並遇見{}{}（{}）", zone_tag, venue.default_host_title, peer_name);
            } else {
                return format!("抵達並遇見{}{}", zone_tag, venue.default_host_title);
            }
        }
    }
    let clean_peer = if peer_name.is_empty() || peer_name.starts_with("npc_") {
        "同坊街坊".to_string()
    } else {
        peer_name
    };
    format!("抵達並遇見{clean_peer}")
}

fn compatibility_room(graph: &PlaceGraph, place_id: &str) -> String {
    graph
        .place(place_id)
        .map(|place| place.compatibility_room_id())
        .unwrap_or_else(|| place_id.to_string())
}

fn set_activity_if_changed(entity_id: &str, current: &str, next: &str) {
    let target_next = if entity_id.starts_with("npc_") {
        if let Some(st) = crate::store::LIVE_NPC_GRID_STATES.read().get(entity_id) {
            st.current_activity()
        } else {
            let slot = crate::store::wenyao_household_slot_for(entity_id);
            let home_settlement = slot.home_name.split('・').next().unwrap_or("文曜");
            const ALL_SETTLEMENTS: &[&str] = &[
                "文曜", "風舶角", "大圳頭", "半嶺鎮", "南哨", "老堰頭", "北嶺腳", "霜林口",
                "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口", "水尾寮", "下營",
            ];
            let mismatches = ALL_SETTLEMENTS.iter().any(|&settlement| {
                !home_settlement.starts_with(settlement) && next.contains(settlement)
            });
            if mismatches {
                return;
            }
            next.to_string()
        }
    } else {
        next.to_string()
    };

    if current != target_next {
        let _ = set_entity_activity(entity_id, &target_next);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod wave14_tests {
    use super::*;

    #[test]
    fn test_settle_obligation_venue_event_pack_note_known_venue() {
        let note = settle_obligation_venue_event_pack_note(
            "老堰頭・水輪石磨工坊",
            "repair_tool",
            "",
        );
        assert!(note.is_some());
        let text = note.unwrap();
        assert!(text.contains("水輪石磨工坊"));
        assert!(text.contains("當值石磨匠師"));
        assert!(text.contains("辦妥「repair_tool」"));
    }

    #[test]
    fn test_settle_obligation_venue_event_pack_note_empty_or_no_venue() {
        assert!(settle_obligation_venue_event_pack_note("", "repair_tool", "").is_none());
        assert!(settle_obligation_venue_event_pack_note("   ", "repair_tool", "").is_none());
        assert!(settle_obligation_venue_event_pack_note("荒蕪礫石空地", "repair_tool", "").is_none());
    }

    #[test]
    fn test_format_venue_arrival_peer_consequence_staff_and_non_venue() {
        let arrival_venue = format_venue_arrival_peer_consequence("老堰頭・水輪石磨工坊", "秦水生");
        assert!(arrival_venue.contains("櫃檯內場的當值石磨匠師"));

        let arrival_wild = format_venue_arrival_peer_consequence("荒蕪礫石空地", "npc_random_nobody_999");
        assert!(arrival_wild.starts_with("抵達並遇見") && !arrival_wild.contains("櫃檯內場"));
    }
}

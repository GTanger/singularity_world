//! 居民場所決策：目標與路徑只讀正式 PlaceGraph，不再猜座標鄰接。

use std::collections::{HashSet, VecDeque};

use crate::db::{
    Personality, get_assignments_for_entity, get_room_ids_for_venue, get_schedule_target,
};
use crate::grid::{SquareCoord, parse_grid_room_id};
use crate::store::get_store;
use crate::world::materials;
use crate::world::place::{PlaceGraph, WorldPlace, legacy_place_id};

use super::super::decision::{DecisionState, IntentType, build_decision_state};

const SEARCH_RADIUS: usize = 8;

#[derive(Debug, Clone)]
pub(super) struct PlaceIntent {
    pub(super) ty: IntentType,
    pub(super) target: Option<String>,
    pub(super) reason: String,
}

#[derive(Debug, Clone)]
struct WeightedIntent {
    intent: PlaceIntent,
    weight: u32,
}

pub(super) fn choose_place_intent(
    entity_id: &str,
    place_id: &str,
    graph: &PlaceGraph,
    hour: i32,
    now_unix: i64,
) -> PlaceIntent {
    if let Some(pool) = crate::pg::pool()
        && let Ok(Some(delivery)) =
            crate::pg::world_logistics::active_delivery_target(&pool, entity_id)
        && let Some(dest_place) = graph.place(&delivery.destination_place_id)
    {
        let raw_title = dest_place.player_title().trim();
        let base_dname = crate::pg::living_world::peninsula_market_settlement_name(raw_title);
        let dname = if base_dname.is_empty() {
            "鄰近市集".to_string()
        } else if base_dname != "鄰近市集" && !base_dname.ends_with("市集") {
            format!("{base_dname}市集")
        } else {
            base_dname.to_string()
        };
        let display_name = crate::pg::living_world::peninsula_good_display_name(&delivery.display_name);
        return PlaceIntent {
            ty: IntentType::Work,
            target: Some(delivery.destination_place_id),
            reason: format!(
                "正在承運{}份{}，準備送往{}",
                delivery.quantity, display_name, dname
            ),
        };
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(obligations) =
            crate::pg::living_world::list_open_obligations_for_entity(&pool, entity_id)
        && let Some(ob) = obligations.into_iter().find(|o| {
            matches!(
                o.obligation_kind.as_str(),
                "repair_tool" | "stranded_cargo" | "repay_debt" | "treat_injury" | "restock_goods" | "supply_shortage" | "craft_material" | "fuel_shortage"
            )
        })
    {
        let found_target = match ob.obligation_kind.as_str() {
            "repair_tool" | "craft_material" => nearest_forge_or_workshop_place(graph, place_id)
                .or_else(|| nearest_work_place(graph, place_id))
                .or_else(|| nearest_trade_place(graph, place_id)),
            "treat_injury" => nearest_clinic_or_rest_place(graph, place_id)
                .or_else(|| nearest_rest_place(graph, place_id)),
            "stranded_cargo" => {
                if ob.subject_ref != place_id && graph.place(&ob.subject_ref).is_some() {
                    Some(ob.subject_ref.clone())
                } else {
                    nearest_trade_place(graph, place_id)
                        .or_else(|| nearest_work_place(graph, place_id))
                }
            }
            "repay_debt" => crate::db::get_entity_room(&ob.subject_ref)
                .ok()
                .filter(|r| r != place_id && graph.place(r).is_some())
                .or_else(|| nearest_trade_place(graph, place_id))
                .or_else(|| nearest_work_place(graph, place_id)),
            _ => nearest_trade_place(graph, place_id)
                .or_else(|| nearest_work_place(graph, place_id)),
        };
        if let Some(target) = found_target.or_else(|| graph.place(place_id).map(|_| place_id.to_string())) {
            let ty = match ob.obligation_kind.as_str() {
                "treat_injury" => IntentType::Idle,
                "repair_tool" | "craft_material" => IntentType::Work,
                _ => IntentType::Trade,
            };
            return PlaceIntent {
                ty,
                target: Some(target),
                reason: format!("obligation:{}", ob.obligation_kind),
            };
        }
    }
    let state = build_decision_state(entity_id).ok();
    if let Some(intent) = economic_recovery_intent(state.as_ref(), place_id, graph) {
        return intent;
    }
    if let Some(target) = scheduled_intent(entity_id, hour, graph) {
        return target;
    }
    let candidates = build_candidates(entity_id, state.as_ref(), place_id, graph, hour, now_unix);
    choose_weighted(entity_id, now_unix, candidates)
}

fn scale_candidate_weight(base: u32, scale: f64) -> u32 {
    let scaled = (base as f64 * scale).round() as i64;
    scaled.max(1) as u32
}

fn nearest_trade_place(graph: &PlaceGraph, origin: &str) -> Option<String> {
    find_reachable(graph, origin, SEARCH_RADIUS, is_trade_place)
}

fn nearest_work_place(graph: &PlaceGraph, origin: &str) -> Option<String> {
    find_reachable(graph, origin, SEARCH_RADIUS, is_work_place)
}

fn nearest_rest_place(graph: &PlaceGraph, origin: &str) -> Option<String> {
    find_reachable(graph, origin, SEARCH_RADIUS, is_rest_place)
}

fn nearest_forge_or_workshop_place(graph: &PlaceGraph, origin: &str) -> Option<String> {
    find_reachable(graph, origin, SEARCH_RADIUS, is_forge_or_workshop_place)
}

fn nearest_clinic_or_rest_place(graph: &PlaceGraph, origin: &str) -> Option<String> {
    find_reachable(graph, origin, SEARCH_RADIUS, is_clinic_or_rest_place)
}
fn build_candidates(
    entity_id: &str,
    state: Option<&DecisionState>,
    place_id: &str,
    graph: &PlaceGraph,
    hour: i32,
    now_unix: i64,
) -> Vec<WeightedIntent> {
    let personality = state
        .map(|s| s.personality.clone())
        .or_else(|| crate::db::get_personality_for_entity(entity_id))
        .unwrap_or(Personality {
            boldness: 0.5,
            sensitivity: 0.5,
            orderliness: 0.5,
        });

    let mut candidates = Vec::new();
    if let Some(intent) = economic_recovery_intent(state, place_id, graph) {
        let weight = scale_candidate_weight(9, 0.75 + 0.5 * personality.sensitivity);
        candidates.push(WeightedIntent { intent, weight });
    }
    if let Some(intent) = night_rest_intent(hour, place_id, graph) {
        let weight = scale_candidate_weight(
            8,
            0.6 + 0.7 * personality.orderliness + 0.2 * personality.sensitivity,
        );
        candidates.push(WeightedIntent { intent, weight });
    }
    if let Some(intent) = assigned_work_intent(entity_id, state, hour, place_id, graph) {
        let weight = scale_candidate_weight(10, 0.6 + 0.8 * personality.orderliness);
        candidates.push(WeightedIntent { intent, weight });
    }
    if (6..=20).contains(&hour)
        && let Some(intent) = event_pack_work_intent(entity_id, state, hour, place_id, graph)
    {
        let weight = scale_candidate_weight(
            6,
            0.5 + 0.5 * personality.boldness + 0.4 * personality.orderliness,
        );
        candidates.push(WeightedIntent { intent, weight });
    }
    if let Some(intent) = unassigned_job_intent(state, hour, place_id, graph) {
        let weight = scale_candidate_weight(
            7,
            0.6 + 0.5 * personality.orderliness + 0.3 * personality.boldness,
        );
        candidates.push(WeightedIntent { intent, weight });
    }
    if let Some(intent) = social_intent(state, place_id, graph) {
        let base_weight = social_weight(state, now_unix);
        let weight = scale_candidate_weight(
            base_weight,
            0.6 + 0.8 * personality.sensitivity + 0.3 * (1.0 - personality.orderliness),
        );
        candidates.push(WeightedIntent { intent, weight });
    }
    let idle_weight = scale_candidate_weight(2, 0.6 + 0.8 * (1.0 - personality.boldness));
    candidates.push(WeightedIntent {
        intent: PlaceIntent {
            ty: IntentType::Idle,
            target: find_reachable(graph, place_id, SEARCH_RADIUS, is_rest_place),
            reason: "留出時間休息與重新判斷".to_string(),
        },
        weight: idle_weight,
    });
    let wander_weight = scale_candidate_weight(
        3,
        0.6 + 1.0 * personality.boldness + 0.4 * (1.0 - personality.orderliness),
    );
    candidates.push(WeightedIntent {
        intent: PlaceIntent {
            ty: IntentType::Wander,
            target: deterministic_reachable(entity_id, now_unix, graph, place_id, SEARCH_RADIUS),
            reason: "沒有更強的既有義務，查看可達的附近場所".to_string(),
        },
        weight: wander_weight,
    });
    candidates
}

fn economic_recovery_intent(
    state: Option<&DecisionState>,
    place_id: &str,
    graph: &PlaceGraph,
) -> Option<PlaceIntent> {
    let state = state?;
    if state.magnesium >= super::super::decision::SURVIVAL_LINE_MG {
        return None;
    }
    if state.inventory_total_qty > 0 {
        return Some(PlaceIntent {
            ty: IntentType::Trade,
            target: Some(find_runtime_market(graph, place_id)?),
            reason: "現有鎂不足，但持物可先進入交換，不再無視背包重複採集".to_string(),
        });
    }
    if graph
        .place(place_id)
        .is_some_and(|place| place.source_kind == "azgaar_runtime")
    {
        // 白塔舊 P0 遍地採集已退役；沒有持物的人改由下方求職／生活候選處理。
        return None;
    }
    Some(PlaceIntent {
        ty: IntentType::Gather,
        target: Some(find_reachable(
            graph,
            place_id,
            SEARCH_RADIUS,
            is_material_source,
        )?),
        reason: "現有鎂與可交換持物皆不足，尋找可取得的世界物質".to_string(),
    })
}

fn night_rest_intent(hour: i32, place_id: &str, graph: &PlaceGraph) -> Option<PlaceIntent> {
    if (6..22).contains(&hour) {
        return None;
    }
    Some(PlaceIntent {
        ty: IntentType::Idle,
        target: find_reachable(graph, place_id, SEARCH_RADIUS, is_rest_place),
        reason: "夜間且沒有更明確的排程，尋找可休息場所".to_string(),
    })
}

fn assigned_work_intent(
    entity_id: &str,
    state: Option<&DecisionState>,
    hour: i32,
    place_id: &str,
    graph: &PlaceGraph,
) -> Option<PlaceIntent> {
    if state.is_none_or(|state| !state.has_assignment) || !(6..20).contains(&hour) {
        return None;
    }
    let target = assigned_venue_place(entity_id, graph)
        .or_else(|| find_reachable(graph, place_id, SEARCH_RADIUS, is_work_place))?;
    Some(PlaceIntent {
        ty: IntentType::Work,
        target: Some(target),
        reason: "已有正式指派且目前在工作時段".to_string(),
    })
}

fn event_pack_work_intent(
    entity_id: &str,
    _state: Option<&DecisionState>,
    _hour: i32,
    place_id: &str,
    graph: &PlaceGraph,
) -> Option<PlaceIntent> {
    if entity_id.trim().is_empty() {
        return None;
    }
    let now_unix = crate::game::now_unix();
    for cand_id in reachable_places(graph, place_id, SEARCH_RADIUS) {
        if let Some(place) = graph.place(&cand_id) {
            let title = place.player_title().trim();
            if title.is_empty() {
                continue;
            }
            if let Some((ep, sp)) =
                crate::game::peninsula_scripts::active_event_and_script_pack_for_room(title, None, now_unix)
            {
                if ep.status != "completed" {
                    return Some(PlaceIntent {
                        ty: IntentType::Work,
                        target: Some(cand_id),
                        reason: format!("響應事件包【{}】與劇本包【{}】現場協作需求", ep.pack_id, sp.title),
                    });
                }
            }
        }
    }
    None
}

fn unassigned_job_intent(
    state: Option<&DecisionState>,
    hour: i32,
    place_id: &str,
    graph: &PlaceGraph,
) -> Option<PlaceIntent> {
    if state.is_none_or(|state| state.has_assignment) || !(6..20).contains(&hour) {
        return None;
    }
    Some(PlaceIntent {
        ty: IntentType::SeekJob,
        target: Some(find_runtime_market(graph, place_id)?),
        reason: "尚無正式指派，前往有真實市場運務的場所詢問工作".to_string(),
    })
}

/// 交易與求職必須前往同一份 Azgaar 市場主資料所建立的場所，不能再以
/// `market` / `social` 等外觀標籤猜測一個看似合理、實際卻無法結算的地點。
fn find_runtime_market(graph: &PlaceGraph, origin: &str) -> Option<String> {
    let runtime_markets = get_store()
        .map(|store| {
            let store = store.read();
            store
                .venues
                .iter()
                .filter(|(venue_id, _)| venue_id.starts_with("wt-market-"))
                .flat_map(|(_, venue)| venue.room_ids.iter().cloned())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    if runtime_markets.is_empty() {
        // 單元測試與尚未採用白塔 runtime 的相容路徑。
        return find_reachable(graph, origin, SEARCH_RADIUS, is_trade_place);
    }
    find_reachable_in_set(graph, origin, &runtime_markets)
        .or_else(|| find_reachable(graph, origin, SEARCH_RADIUS, is_trade_place))
}

fn find_reachable_in_set(
    graph: &PlaceGraph,
    origin: &str,
    target_ids: &HashSet<String>,
) -> Option<String> {
    for max_steps in [SEARCH_RADIUS, SEARCH_RADIUS * 3, usize::MAX] {
        if let Some(place_id) = reachable_places(graph, origin, max_steps)
            .into_iter()
            .find(|place_id| target_ids.contains(place_id))
        {
            return Some(place_id);
        }
    }
    None
}

fn assigned_venue_place(entity_id: &str, graph: &PlaceGraph) -> Option<String> {
    for assignment in get_assignments_for_entity(entity_id).ok()? {
        if let Ok(Some(room_ids)) = get_room_ids_for_venue(&assignment.venue_id) {
            for room_id in room_ids {
                if let Some(place_id) = parse_target_place(&room_id, graph) {
                    return Some(place_id);
                }
            }
        }
    }
    None
}

fn social_intent(
    _state: Option<&DecisionState>,
    place_id: &str,
    graph: &PlaceGraph,
) -> Option<PlaceIntent> {
    Some(PlaceIntent {
        ty: IntentType::Socialize,
        target: Some(find_reachable(
            graph,
            place_id,
            SEARCH_RADIUS,
            is_social_place,
        )?),
        reason: "可達範圍內有人際往來場所".to_string(),
    })
}

fn social_weight(state: Option<&DecisionState>, now_unix: i64) -> u32 {
    let Some(state) = state else {
        return 3;
    };
    let elapsed = if state.last_talk_at <= 0 {
        i64::MAX
    } else {
        now_unix.saturating_sub(state.last_talk_at)
    };
    let time_weight = if elapsed >= 7_200 { 4 } else { 2 };
    let disposition_bonus = u32::from(state.disposition > 25);
    time_weight + disposition_bonus
}

fn choose_weighted(entity_id: &str, now_unix: i64, candidates: Vec<WeightedIntent>) -> PlaceIntent {
    let total: u64 = candidates
        .iter()
        .map(|candidate| u64::from(candidate.weight))
        .sum();
    let mut cursor = stable_hash(&format!("{entity_id}:{}", now_unix / 30)) % total.max(1);
    for candidate in candidates {
        let weight = u64::from(candidate.weight);
        if cursor < weight {
            return candidate.intent;
        }
        cursor -= weight;
    }
    unreachable!("至少有休息與走動候選")
}

fn scheduled_intent(entity_id: &str, hour: i32, graph: &PlaceGraph) -> Option<PlaceIntent> {
    let target = get_schedule_target(entity_id, hour)?;
    let place_id = parse_target_place(&target.room, graph)?;
    Some(PlaceIntent {
        ty: if target.is_work {
            IntentType::Work
        } else {
            IntentType::Idle
        },
        target: Some(place_id),
        reason: "已有明確排程".to_string(),
    })
}

fn parse_target_place(raw: &str, graph: &PlaceGraph) -> Option<String> {
    let trimmed = raw.trim();
    if graph.place(trimmed).is_some() {
        return Some(trimmed.to_string());
    }
    let coord = parse_grid_room_id(trimmed).or_else(|| parse_coord_pair(trimmed))?;
    let place_id = legacy_place_id(coord);
    graph.place(&place_id).map(|_| place_id)
}

fn parse_coord_pair(raw: &str) -> Option<SquareCoord> {
    let (x, y) = raw.split_once(',')?;
    Some(SquareCoord::new(
        x.trim().parse().ok()?,
        y.trim().parse().ok()?,
    ))
}

fn find_reachable(
    graph: &PlaceGraph,
    origin: &str,
    max_steps: usize,
    predicate: fn(&WorldPlace) -> bool,
) -> Option<String> {
    reachable_places(graph, origin, max_steps)
        .into_iter()
        .find(|place_id| graph.place(place_id).is_some_and(predicate))
}

fn deterministic_reachable(
    entity_id: &str,
    now_unix: i64,
    graph: &PlaceGraph,
    origin: &str,
    max_steps: usize,
) -> Option<String> {
    let candidates: Vec<_> = reachable_places(graph, origin, max_steps)
        .into_iter()
        .filter(|place_id| place_id != origin)
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let index =
        stable_hash(&format!("{entity_id}:{origin}:{}", now_unix / 30)) % candidates.len() as u64;
    Some(candidates[index as usize].clone())
}

fn stable_hash(value: &str) -> u64 {
    value.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn reachable_places(graph: &PlaceGraph, origin: &str, max_steps: usize) -> Vec<String> {
    if graph.place(origin).is_none() {
        return Vec::new();
    }
    let mut visited = HashSet::from([origin.to_string()]);
    let mut queue = VecDeque::from([(origin.to_string(), 0usize)]);
    let mut result = Vec::new();
    while let Some((place_id, depth)) = queue.pop_front() {
        result.push(place_id.clone());
        if depth >= max_steps {
            continue;
        }
        for route in graph.routes_from(&place_id) {
            if route.status == "active" && visited.insert(route.to_place_id.clone()) {
                queue.push_back((route.to_place_id.clone(), depth + 1));
            }
        }
    }
    result
}

fn is_material_source(place: &WorldPlace) -> bool {
    place.tags.iter().any(|tag| tag.starts_with("resource:"))
        || materials::material_for_room_id(&place.compatibility_room_id()).is_some()
}

fn is_forge_or_workshop_place(place: &WorldPlace) -> bool {
    has_any_tag(
        place,
        &["鐵匠鋪", "工坊", "磨石", "礦場", "林場", "織坊"],
    ) || {
        let title = place.player_title();
        title.contains('鍛')
            || title.contains('鐵')
            || title.contains("工坊")
            || title.contains("巧木")
            || title.contains("織造")
            || title.contains('窯')
    }
}

fn is_clinic_or_rest_place(place: &WorldPlace) -> bool {
    has_any_tag(
        place,
        &["藥圃", "藥材", "浴場", "旅社", "歇宿", "民居"],
    ) || {
        let title = place.player_title();
        title.contains("診所")
            || title.contains("脈絡")
            || title.contains("百草")
            || title.contains("浴場")
            || title.contains("旅社")
            || title.contains("茶屋")
    }
}

fn is_social_place(place: &WorldPlace) -> bool {
    has_any_tag(
        place,
        &[
            "social", "market", "public_square", "rest_spot",
            "食堂", "熱食", "現煮飲品", "歇腳",
            "茶屋", "廣場", "浴場", "石橋",
        ],
    ) || matches!(place.kind.as_str(), "tavern" | "inn" | "market" | "urban")
}

fn is_trade_place(place: &WorldPlace) -> bool {
    has_any_tag(
        place,
        &[
            "trade", "market", "public_square", "social",
            "商鋪", "市集", concat!("零", "售"), "雜貨", "收購", "櫃檯",
            "聯運", "碼頭", "茶屋",
        ],
    ) || matches!(place.kind.as_str(), "market" | "urban" | "dock")
}

fn is_rest_place(place: &WorldPlace) -> bool {
    has_any_tag(
        place,
        &[
            "rest", "rest_spot", "home", "lodging",
            "民居", "歇宿", "歇腳",
            "旅社", "浴場", "茶屋", "水井",
        ],
    ) || matches!(place.kind.as_str(), "inn" | "tavern" | "farmhouse")
}

fn is_work_place(place: &WorldPlace) -> bool {
    has_any_tag(
        place,
        &[
            "work", "trade", "warehouse", "dock", "farm",
            "工坊", "鐵匠鋪", "糧倉", "礦場", "林場", "藥圃", "漁港", "織坊",
            "磨石", "藥材", "聯運", "碼頭", "水井",
        ],
    ) || matches!(
        place.kind.as_str(),
        "urban" | "workshop" | "warehouse" | "dock"
    )
}

fn has_any_tag(place: &WorldPlace, tags: &[&str]) -> bool {
    place
        .tags
        .iter()
        .any(|place_tag| tags.iter().any(|wanted| place_tag == wanted))
}

#[cfg(test)]
pub(super) fn test_parse_target_place(raw: &str, graph: &PlaceGraph) -> Option<String> {
    parse_target_place(raw, graph)
}

#[cfg(test)]
pub(super) fn test_reachable_places(
    graph: &PlaceGraph,
    origin: &str,
    max_steps: usize,
) -> Vec<String> {
    reachable_places(graph, origin, max_steps)
}

#[cfg(test)]
pub(super) fn test_find_reachable_in_set(
    graph: &PlaceGraph,
    origin: &str,
    target_ids: &HashSet<String>,
) -> Option<String> {
    find_reachable_in_set(graph, origin, target_ids)
}

#[cfg(test)]
pub(super) fn test_economic_recovery_intent(
    magnesium: i32,
    inventory_total_qty: i32,
    origin: &str,
    graph: &PlaceGraph,
) -> Option<IntentType> {
    let state = DecisionState {
        magnesium,
        has_assignment: false,
        personality: crate::db::Personality {
            boldness: 0.5,
            sensitivity: 0.5,
            orderliness: 0.5,
        },
        has_personality: true,
        last_talk_at: 0,
        disposition: 0,
        inventory_total_qty,
    };
    economic_recovery_intent(Some(&state), origin, graph).map(|intent| intent.ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_place(id: &str, tags: &[&str], title: &str) -> WorldPlace {
        let mut place_tags: Vec<String> = tags.iter().map(|s| s.to_string()).collect();
        if !title.is_empty() {
            place_tags.push(format!("player_title:{}", title));
        }
        WorldPlace {
            id: id.to_string(),
            world_id: crate::world::place::LEGACY_WORLD_ID.to_string(),
            topology_node_ref: None,
            parent_place_id: None,
            name: id.to_string(),
            kind: "urban".to_string(),
            scale: "place".to_string(),
            canon: false,
            status: "material".to_string(),
            traversable: true,
            source_kind: "legacy_grid".to_string(),
            zone: String::new(),
            tags: place_tags,
            description: String::new(),
            legacy_grid_x: Some(0),
            legacy_grid_y: Some(0),
        }
    }

    #[test]
    fn test_is_forge_or_workshop_place() {
        let p_tag = dummy_place("p1", &["工坊"], "");
        assert!(is_forge_or_workshop_place(&p_tag));

        let p_smith = dummy_place("p2", &["鐵匠鋪"], "");
        assert!(is_forge_or_workshop_place(&p_smith));

        let p_title = dummy_place("p3", &[], "老堰頭巧木工坊");
        assert!(is_forge_or_workshop_place(&p_title));

        let p_unrelated = dummy_place("p4", &["rest"], "尋常民宅");
        assert!(!is_forge_or_workshop_place(&p_unrelated));
    }

    #[test]
    fn test_is_clinic_or_rest_place() {
        let p_tag = dummy_place("p1", &["藥圃"], "");
        assert!(is_clinic_or_rest_place(&p_tag));

        let p_clinic_title = dummy_place("p2", &[], "文曜草藥診所");
        assert!(is_clinic_or_rest_place(&p_clinic_title));

        let p_bath = dummy_place("p3", &["浴場"], "");
        assert!(is_clinic_or_rest_place(&p_bath));

        let p_unrelated = dummy_place("p4", &["mine"], "赤嶺礦坑");
        assert!(!is_clinic_or_rest_place(&p_unrelated));
    }

    #[test]
    fn test_expanded_tag_predicates() {
        let p_bath = dummy_place("b", &["浴場"], "");
        assert!(is_social_place(&p_bath));
        assert!(is_rest_place(&p_bath));

        let p_tea = dummy_place("t", &["茶屋"], "");
        assert!(is_social_place(&p_tea));
        assert!(is_trade_place(&p_tea));
        assert!(is_rest_place(&p_tea));

        let p_inn = dummy_place("i", &["旅社"], "");
        assert!(is_rest_place(&p_inn));

        let p_logistics = dummy_place("l", &["聯運"], "");
        assert!(is_trade_place(&p_logistics));
        assert!(is_work_place(&p_logistics));

        let p_square = dummy_place("s", &["廣場"], "");
        assert!(is_social_place(&p_square));
    }

    #[test]
    fn test_event_pack_work_intent() {
        let mut p = dummy_place("wt_mill", &[], "老堰頭水輪石磨工坊");
        p.traversable = true;
        let graph = PlaceGraph::new(vec![p], Vec::new()).unwrap();

        let intent_empty = event_pack_work_intent("", None, 10, "wt_mill", &graph);
        assert!(intent_empty.is_none());

        let intent = event_pack_work_intent("npc_worker_1", None, 10, "wt_mill", &graph);
        assert!(intent.is_some());
        let intent = intent.unwrap();
        assert_eq!(intent.ty, IntentType::Work);
        assert_eq!(intent.target.as_deref(), Some("wt_mill"));
        assert!(intent.reason.contains("響應事件包"));
        assert!(intent.reason.contains("劇本包"));
    }
}

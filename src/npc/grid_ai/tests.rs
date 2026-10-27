use std::collections::HashSet;

use super::decision::{
    test_economic_recovery_intent, test_find_reachable_in_set, test_parse_target_place,
    test_reachable_places,
};
use super::*;
use crate::world::place::{LEGACY_WORLD_ID, WorldPlace, WorldRoute, legacy_place_id};

fn place(id: &str, x: i32, y: i32) -> WorldPlace {
    WorldPlace {
        id: id.to_string(),
        world_id: LEGACY_WORLD_ID.to_string(),
        topology_node_ref: None,
        parent_place_id: None,
        name: id.to_string(),
        kind: "road".to_string(),
        scale: "place".to_string(),
        canon: false,
        status: "material".to_string(),
        traversable: true,
        source_kind: "legacy_grid".to_string(),
        zone: String::new(),
        tags: Vec::new(),
        description: String::new(),
        legacy_grid_x: Some(x),
        legacy_grid_y: Some(y),
    }
}

fn route(id: &str, from: &str, to: &str, direction: &str) -> WorldRoute {
    WorldRoute {
        id: id.to_string(),
        from_place_id: from.to_string(),
        direction: direction.to_string(),
        to_place_id: to.to_string(),
        mode: "walk".to_string(),
        travel_cost: 1,
        canon: false,
        status: "active".to_string(),
        source_kind: "legacy_grid".to_string(),
    }
}

fn test_graph() -> PlaceGraph {
    let a = legacy_place_id(crate::grid::SquareCoord::new(0, 0));
    let b = legacy_place_id(crate::grid::SquareCoord::new(1, 0));
    let c = legacy_place_id(crate::grid::SquareCoord::new(0, 1));
    PlaceGraph::new(
        vec![place(&a, 0, 0), place(&b, 1, 0), place(&c, 0, 1)],
        vec![route("ab", &a, &b, "東")],
    )
    .unwrap()
}

fn economic_graph() -> (PlaceGraph, String) {
    let origin = legacy_place_id(crate::grid::SquareCoord::new(0, 0));
    let mut source_and_market = place(&origin, 0, 0);
    source_and_market.tags = vec!["trade".to_string(), "resource:test".to_string()];
    (
        PlaceGraph::new(vec![source_and_market], Vec::new()).unwrap(),
        origin,
    )
}

#[test]
fn fair_batch_wraps_without_starving_ids() {
    let ids = (0..5).map(|i| format!("npc-{i}")).collect::<Vec<_>>();
    let states = ids
        .iter()
        .map(|id| (id.clone(), NpcState::default()))
        .collect();
    let mut cursor = 0;
    assert_eq!(
        take_fair_due_batch(&ids, &states, &mut cursor, 0, 3),
        ids[0..3]
    );
    assert_eq!(
        take_fair_due_batch(&ids, &states, &mut cursor, 0, 3),
        vec![ids[3].clone(), ids[4].clone(), ids[0].clone()]
    );
    assert_eq!(cursor, 1);
}

#[test]
fn fair_batch_skips_sleeping_residents_without_losing_cursor_order() {
    let ids = (0..5).map(|i| format!("npc-{i}")).collect::<Vec<_>>();
    let states = ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            (
                id.clone(),
                NpcState {
                    next_wake_at: if index % 2 == 0 { 0 } else { 100 },
                    ..NpcState::default()
                },
            )
        })
        .collect();
    let mut cursor = 0;
    assert_eq!(
        take_fair_due_batch(&ids, &states, &mut cursor, 50, 2),
        vec![ids[0].clone(), ids[2].clone()]
    );
    assert_eq!(cursor, 3);
    assert_eq!(
        take_fair_due_batch(&ids, &states, &mut cursor, 50, 2),
        vec![ids[4].clone(), ids[0].clone()]
    );
}

#[test]
fn schedule_target_accepts_place_id_room_id_and_legacy_pair() {
    let graph = test_graph();
    let expected = legacy_place_id(crate::grid::SquareCoord::new(0, 0));
    assert_eq!(
        test_parse_target_place(&expected, &graph),
        Some(expected.clone())
    );
    assert_eq!(
        test_parse_target_place("grid:0:0", &graph),
        Some(expected.clone())
    );
    assert_eq!(test_parse_target_place(" 0, 0 ", &graph), Some(expected));
    assert_eq!(test_parse_target_place("old-room", &graph), None);
}

#[test]
fn reachability_obeys_routes_instead_of_geometric_guessing() {
    let graph = test_graph();
    let origin = legacy_place_id(crate::grid::SquareCoord::new(0, 0));
    let east = legacy_place_id(crate::grid::SquareCoord::new(1, 0));
    let north = legacy_place_id(crate::grid::SquareCoord::new(0, 1));
    let reachable = test_reachable_places(&graph, &origin, 1);
    assert!(reachable.contains(&east));
    assert!(!reachable.contains(&north));
}

#[test]
fn runtime_market_id_wins_over_a_market_like_tag() {
    let graph = test_graph();
    let origin = legacy_place_id(crate::grid::SquareCoord::new(0, 0));
    let actual_market = legacy_place_id(crate::grid::SquareCoord::new(1, 0));
    let targets = HashSet::from([actual_market.clone()]);
    assert_eq!(
        test_find_reachable_in_set(&graph, &origin, &targets),
        Some(actual_market)
    );
}

#[test]
fn shared_place_is_not_treated_as_a_movement_blocker() {
    let snapshots = HashMap::from([
        (
            "npc-a".to_string(),
            NpcSnapshot {
                place_id: "place-a".to_string(),
                current_activity: String::new(),
            },
        ),
        (
            "npc-b".to_string(),
            NpcSnapshot {
                place_id: "place-a".to_string(),
                current_activity: String::new(),
            },
        ),
    ]);
    let occupants = occupants_by_place(&snapshots);
    assert_eq!(occupants["place-a"].len(), 2);
}

#[test]
fn moving_one_resident_updates_the_batch_occupant_snapshot() {
    let mut occupants = HashMap::from([
        (
            "from".to_string(),
            vec!["npc-a".to_string(), "npc-b".to_string()],
        ),
        ("to".to_string(), vec!["npc-c".to_string()]),
    ]);
    relocate_occupant(&mut occupants, "npc-a", "from", "to");
    assert_eq!(occupants["from"], vec!["npc-b"]);
    assert_eq!(occupants["to"], vec!["npc-a", "npc-c"]);
}

#[test]
fn low_funds_use_existing_stock_before_gathering_more() {
    let (graph, origin) = economic_graph();
    assert_eq!(
        test_economic_recovery_intent(0, 12, &origin, &graph),
        Some(IntentType::Trade)
    );
    assert_eq!(
        test_economic_recovery_intent(0, 0, &origin, &graph),
        Some(IntentType::Gather)
    );
    assert_eq!(test_economic_recovery_intent(50, 0, &origin, &graph), None);
}

#[test]
fn white_tower_low_funds_do_not_resurrect_retired_p0_gathering() {
    let origin = "wt-physical-test".to_string();
    let mut runtime_place = place(&origin, 0, 0);
    runtime_place.source_kind = "azgaar_runtime".to_string();
    runtime_place.tags = vec!["resource:retired-p0".to_string()];
    let graph = PlaceGraph::new(vec![runtime_place], Vec::new()).unwrap();
    assert_eq!(test_economic_recovery_intent(0, 0, &origin, &graph), None);
}

#[test]
fn committed_move_journey_starts_after_the_new_current_place() {
    let state = NpcState {
        path: vec![
            "next".to_string(),
            "later".to_string(),
            "target".to_string(),
        ],
        destination_place_id: Some("target".to_string()),
        last_intent: Some(IntentType::SeekJob),
        ..NpcState::default()
    };
    let journey = journey_after_committed_step("npc", &state, "next", 42);
    assert_eq!(journey.destination_place_id.as_deref(), Some("target"));
    assert_eq!(journey.planned_path, vec!["later", "target"]);
    assert_ne!(
        journey.planned_path.first().map(String::as_str),
        Some("next")
    );
}

#[test]
fn committed_arrival_journey_ends_at_the_new_current_place() {
    let state = NpcState {
        path: vec!["target".to_string()],
        destination_place_id: Some("target".to_string()),
        last_intent: Some(IntentType::Work),
        ..NpcState::default()
    };
    let journey = journey_after_committed_step("npc", &state, "target", 42);
    assert_eq!(journey.destination_place_id.as_deref(), Some("target"));
    assert!(journey.planned_path.is_empty());
}

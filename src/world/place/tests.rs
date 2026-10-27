use super::*;

fn place(id: &str, traversable: bool) -> WorldPlace {
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
        traversable,
        source_kind: "legacy_grid".to_string(),
        zone: String::new(),
        tags: Vec::new(),
        description: String::new(),
        legacy_grid_x: None,
        legacy_grid_y: None,
    }
}

fn route(id: &str, from: &str, to: &str, direction: &str, cost: i32) -> WorldRoute {
    WorldRoute {
        id: id.to_string(),
        from_place_id: from.to_string(),
        direction: direction.to_string(),
        to_place_id: to.to_string(),
        mode: "walk".to_string(),
        travel_cost: cost,
        canon: false,
        status: "active".to_string(),
        source_kind: "legacy_grid".to_string(),
    }
}

#[test]
fn weighted_path_uses_routes_instead_of_coordinate_guessing() {
    let graph = PlaceGraph::new(
        vec![place("a", true), place("b", true), place("c", true)],
        vec![
            route("ab", "a", "b", "東", 9),
            route("ac", "a", "c", "南", 1),
            route("cb", "c", "b", "北", 1),
        ],
    )
    .unwrap();
    assert_eq!(graph.find_path("a", "b").unwrap(), ["a", "c", "b"]);
}

#[test]
fn route_rejects_non_traversable_topology_container() {
    let result = PlaceGraph::new(
        vec![place("a", true), place("region", false)],
        vec![route("bad", "a", "region", "北", 1)],
    );
    assert!(result.is_err());
}

#[test]
fn legacy_place_id_round_trips_negative_coordinates() {
    let coord = SquareCoord::new(7, -12);
    assert_eq!(parse_legacy_place_id(&legacy_place_id(coord)), Some(coord));
}

#[test]
fn player_place_identity_separates_title_from_locator() {
    let mut p = place("wt-physical-0088", true);
    p.name = "文曜（東側 · 水澤林）".to_string();
    p.tags = vec![
        "player_title:文曜".to_string(),
        "player_locator:東側 · 水澤林".to_string(),
    ];

    assert_eq!(p.player_title(), "文曜");
    assert_eq!(p.player_locator(), Some("東側 · 水澤林"));
}

#[test]
fn legacy_place_identity_falls_back_without_display_tags() {
    let p = place("舊路口", true);

    assert_eq!(p.player_title(), "舊路口");
    assert_eq!(p.player_locator(), None);
}

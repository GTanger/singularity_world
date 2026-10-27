//! 世界場所與可通行路線；座標只是舊格資料的相容屬性。

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};

use crate::grid::SquareCoord;

pub const LEGACY_WORLD_ID: &str = "legacy_runtime_material";
pub const WHITE_TOWER_WORLD_ID: &str = "white_tower_peninsula";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldPlace {
    pub id: String,
    pub world_id: String,
    pub topology_node_ref: Option<String>,
    pub parent_place_id: Option<String>,
    pub name: String,
    pub kind: String,
    pub scale: String,
    pub canon: bool,
    pub status: String,
    pub traversable: bool,
    pub source_kind: String,
    pub zone: String,
    pub tags: Vec<String>,
    pub description: String,
    pub legacy_grid_x: Option<i32>,
    pub legacy_grid_y: Option<i32>,
}

impl WorldPlace {
    /// 玩家畫面的正式大地名。低層 runtime 的方位與地貌只能放在 locator。
    pub fn player_title(&self) -> &str {
        self.tags
            .iter()
            .find_map(|tag| tag.strip_prefix("player_title:"))
            .filter(|title| !title.trim().is_empty())
            .unwrap_or(&self.name)
    }

    /// 玩家畫面的局部定位，例如「東側 · 水澤林」；不是新造的正典專名。
    pub fn player_locator(&self) -> Option<&str> {
        self.tags
            .iter()
            .find_map(|tag| tag.strip_prefix("player_locator:"))
            .filter(|locator| !locator.trim().is_empty())
    }

    pub fn legacy_coord(&self) -> Option<SquareCoord> {
        Some(SquareCoord::new(self.legacy_grid_x?, self.legacy_grid_y?))
    }

    pub fn compatibility_room_id(&self) -> String {
        self.legacy_coord()
            .map(|coord| crate::grid::grid_room_id_from_coord(coord.x, coord.y))
            .unwrap_or_else(|| self.id.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldRoute {
    pub id: String,
    pub from_place_id: String,
    pub direction: String,
    pub to_place_id: String,
    pub mode: String,
    pub travel_cost: i32,
    pub canon: bool,
    pub status: String,
    pub source_kind: String,
}

#[derive(Debug, Clone, Default)]
pub struct PlaceGraph {
    places: HashMap<String, WorldPlace>,
    outgoing: HashMap<String, Vec<WorldRoute>>,
}

impl PlaceGraph {
    pub fn new(places: Vec<WorldPlace>, routes: Vec<WorldRoute>) -> anyhow::Result<Self> {
        let places: HashMap<_, _> = places.into_iter().map(|p| (p.id.clone(), p)).collect();
        let mut outgoing: HashMap<String, Vec<WorldRoute>> = HashMap::new();
        for route in routes {
            validate_route(&route, &places)?;
            outgoing
                .entry(route.from_place_id.clone())
                .or_default()
                .push(route);
        }
        for routes in outgoing.values_mut() {
            routes.sort_by(|a, b| {
                (&a.direction, &a.to_place_id).cmp(&(&b.direction, &b.to_place_id))
            });
        }
        Ok(Self { places, outgoing })
    }

    pub fn place(&self, place_id: &str) -> Option<&WorldPlace> {
        self.places.get(place_id)
    }

    pub fn places(&self) -> impl Iterator<Item = &WorldPlace> {
        self.places.values()
    }

    pub fn routes_from(&self, place_id: &str) -> &[WorldRoute] {
        self.outgoing
            .get(place_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn route_in_direction(&self, place_id: &str, direction: &str) -> Option<&WorldRoute> {
        self.routes_from(place_id)
            .iter()
            .find(|route| route.direction == direction && route.status == "active")
    }

    pub fn route_to(&self, from: &str, to: &str) -> Option<&WorldRoute> {
        self.routes_from(from)
            .iter()
            .find(|route| route.to_place_id == to && route.status == "active")
    }

    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        if from == to && self.places.contains_key(from) {
            return Some(vec![from.to_string()]);
        }
        if !self.places.contains_key(from) || !self.places.contains_key(to) {
            return None;
        }
        let parents = self.shortest_path_parents(from, to)?;
        rebuild_path(from, to, &parents)
    }

    fn shortest_path_parents(&self, from: &str, to: &str) -> Option<HashMap<String, String>> {
        let mut distance = HashMap::from([(from.to_string(), 0_i32)]);
        let mut parents = HashMap::new();
        let mut queue = BinaryHeap::from([Reverse((0_i32, from.to_string()))]);
        while let Some(Reverse((cost, current))) = queue.pop() {
            if current == to {
                return Some(parents);
            }
            if cost > *distance.get(&current).unwrap_or(&i32::MAX) {
                continue;
            }
            for route in self.routes_from(&current) {
                relax_route(route, cost, &mut distance, &mut parents, &mut queue);
            }
        }
        None
    }
}

fn validate_route(route: &WorldRoute, places: &HashMap<String, WorldPlace>) -> anyhow::Result<()> {
    let valid_direction = matches!(
        route.direction.as_str(),
        "東" | "西" | "南" | "北" | "上" | "下" | "進" | "出"
    );
    anyhow::ensure!(valid_direction, "無效路線方向：{}", route.direction);
    anyhow::ensure!(route.travel_cost > 0, "路線成本必須大於零");
    anyhow::ensure!(places.contains_key(&route.from_place_id), "路線起點不存在");
    let target = places
        .get(&route.to_place_id)
        .ok_or_else(|| anyhow::anyhow!("路線終點不存在"))?;
    anyhow::ensure!(target.traversable, "路線終點不可通行");
    Ok(())
}

fn relax_route(
    route: &WorldRoute,
    base_cost: i32,
    distance: &mut HashMap<String, i32>,
    parents: &mut HashMap<String, String>,
    queue: &mut BinaryHeap<Reverse<(i32, String)>>,
) {
    if route.status != "active" {
        return;
    }
    let next_cost = base_cost.saturating_add(route.travel_cost);
    if next_cost >= *distance.get(&route.to_place_id).unwrap_or(&i32::MAX) {
        return;
    }
    distance.insert(route.to_place_id.clone(), next_cost);
    parents.insert(route.to_place_id.clone(), route.from_place_id.clone());
    queue.push(Reverse((next_cost, route.to_place_id.clone())));
}

fn rebuild_path(from: &str, to: &str, parents: &HashMap<String, String>) -> Option<Vec<String>> {
    let mut path = vec![to.to_string()];
    let mut current = to;
    while current != from {
        current = parents.get(current)?;
        path.push(current.to_string());
    }
    path.reverse();
    Some(path)
}

pub fn legacy_place_id(coord: SquareCoord) -> String {
    format!("legacy:grid:{}:{}", coord.x, coord.y)
}

pub fn parse_legacy_place_id(place_id: &str) -> Option<SquareCoord> {
    let raw = place_id.strip_prefix("legacy:grid:")?;
    let (x, y) = raw.split_once(':')?;
    Some(SquareCoord::new(x.parse().ok()?, y.parse().ok()?))
}

#[cfg(test)]
#[path = "place/tests.rs"]
mod tests;

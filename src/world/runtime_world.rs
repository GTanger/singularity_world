//! 白塔半島正式執行種子。
//!
//! Azgaar 原始包仍是可回載的作者證據；本模組只讀由該包編譯出的精簡種子，
//! 並以嚴格契約確認哪些資料成為執行世界、哪些只保留為可回查候選。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

use super::physical_geography::NormalizedPoint;
use super::place::{PlaceGraph, WHITE_TOWER_WORLD_ID, WorldPlace, WorldRoute};
use super::topology::builtin_white_tower_topology;

const WHITE_TOWER_RUNTIME_JSON: &str = include_str!("../../data/world/white_tower_runtime_v1.json");

pub const WHITE_TOWER_RUNTIME_ID: &str = "white_tower_runtime_v1";
pub const WHITE_TOWER_RUNTIME_SOURCE_KIND: &str = "azgaar_runtime";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WhiteTowerRuntimeWorld {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub status: String,
    pub canon: bool,
    pub runtime_authority: bool,
    pub player_facing_map: bool,
    pub source: RuntimeSource,
    pub spawn_place_id: String,
    pub places: Vec<WorldPlace>,
    pub routes: Vec<WorldRoute>,
    pub area_profiles: Vec<RuntimeAreaProfile>,
    pub resources: Vec<RuntimeResourceSource>,
    pub source_layers: Vec<SourceLayerDisposition>,
    pub coverage: RuntimeCoverage,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeSource {
    pub package_id: String,
    pub package_manifest_path: String,
    pub package_manifest_sha256: String,
    pub full_json_path: String,
    pub full_json_sha256: String,
    pub physical_geography_path: String,
    pub physical_geography_sha256: String,
    pub azgaar_commit: String,
    pub seed: String,
    pub compiler_version: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeAreaProfile {
    pub place_id: String,
    pub centroid: NormalizedPoint,
    pub terrain_class: String,
    pub elevation_band: String,
    pub biome_names: Vec<String>,
    pub source_cell_count: usize,
    pub river_cell_count: usize,
    pub candidate_burg_count: usize,
    pub candidate_population: f64,
    pub candidate_capital_count: usize,
    pub candidate_market_refs: Vec<u32>,
    pub candidate_state_refs: Vec<u32>,
    pub candidate_culture_refs: Vec<u32>,
    pub candidate_religion_refs: Vec<u32>,
    pub candidate_good_refs: Vec<u32>,
    pub native_route_segment_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RuntimeResourceSource {
    pub place_id: String,
    pub item_id: String,
    pub name: String,
    pub origin_label: String,
    pub capacity: i32,
    pub regeneration_per_day: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SourceLayerDisposition {
    pub layer: String,
    pub source_count: usize,
    pub disposition: String,
    pub runtime_use: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RuntimeCoverage {
    pub physical_areas_activated: usize,
    pub physical_routes_activated: usize,
    pub native_routes_accounted: usize,
    pub native_routes_crossing_areas: usize,
    pub native_routes_within_one_area: usize,
    pub burgs_accounted: usize,
    pub burgs_associated_to_areas: usize,
    pub burgs_using_nearest_feature_fallback: usize,
    pub markets_accounted: usize,
    pub goods_accounted: usize,
    pub deals_accounted: usize,
}

impl WhiteTowerRuntimeWorld {
    pub fn graph(&self) -> Result<PlaceGraph> {
        PlaceGraph::new(self.places.clone(), self.routes.clone())
    }

    pub fn profile(&self, place_id: &str) -> Option<&RuntimeAreaProfile> {
        self.area_profiles
            .iter()
            .find(|profile| profile.place_id == place_id)
    }

    pub fn resource(&self, place_id: &str) -> Option<&RuntimeResourceSource> {
        self.resources
            .iter()
            .find(|source| source.place_id == place_id)
    }

    pub fn nearest_place(&self, point: &NormalizedPoint) -> Option<&WorldPlace> {
        let profile = self.area_profiles.iter().min_by(|left, right| {
            squared_distance(&left.centroid, point)
                .total_cmp(&squared_distance(&right.centroid, point))
                .then_with(|| left.place_id.cmp(&right.place_id))
        })?;
        self.places
            .iter()
            .find(|place| place.id == profile.place_id)
    }
}

pub fn builtin_white_tower_runtime_world() -> Result<WhiteTowerRuntimeWorld> {
    let world: WhiteTowerRuntimeWorld = serde_json::from_str(WHITE_TOWER_RUNTIME_JSON)?;
    validate_white_tower_runtime_world(&world)?;
    Ok(world)
}

pub fn validate_white_tower_runtime_world(world: &WhiteTowerRuntimeWorld) -> Result<()> {
    validate_header(world)?;
    validate_source(world)?;
    let place_ids = validate_places(world)?;
    validate_routes(world, &place_ids)?;
    validate_profiles_and_resources(world, &place_ids)?;
    validate_source_layers(world)?;
    validate_coverage(world)?;
    validate_connectivity(world)
}

fn validate_header(world: &WhiteTowerRuntimeWorld) -> Result<()> {
    ensure!(world.schema_version == 1, "runtime schema_version 必須為 1");
    ensure!(world.id == WHITE_TOWER_RUNTIME_ID, "runtime id 不符");
    ensure!(world.status == "accepted_runtime", "runtime 狀態必須已接受");
    ensure!(!world.canon, "低層執行底稿不可冒充正典");
    ensure!(
        world.runtime_authority,
        "執行種子必須明示 runtime_authority"
    );
    ensure!(!world.player_facing_map, "Azgaar renderer 不可成為玩家表面");
    ensure!(!world.name.trim().is_empty(), "runtime name 不可空白");
    Ok(())
}

fn validate_source(world: &WhiteTowerRuntimeWorld) -> Result<()> {
    let source = &world.source;
    ensure!(
        source.package_id == "white_tower_peninsula_azgaar_package_v1",
        "作者包 id 不符"
    );
    ensure!(
        source.azgaar_commit == "73e43d2b44b9b85b42c5394a0476d90bb22d688e",
        "Azgaar commit 漂移"
    );
    ensure!(source.seed == "white-tower-peninsula-v1", "seed 漂移");
    ensure!(source.compiler_version == 4, "runtime compiler 版本不符");
    for (label, value) in [
        ("manifest hash", &source.package_manifest_sha256),
        ("full JSON hash", &source.full_json_sha256),
        ("geography hash", &source.physical_geography_sha256),
    ] {
        ensure!(value.len() == 64, "{label} 不是 SHA-256");
        ensure!(
            value.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "{label} 無效"
        );
    }
    Ok(())
}

fn validate_places(world: &WhiteTowerRuntimeWorld) -> Result<BTreeSet<&str>> {
    ensure!(world.places.len() == 305, "執行場所必須完整採用 305 區");
    let topology = builtin_white_tower_topology()?;
    let topology_ids = topology
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    for place in &world.places {
        ensure!(
            ids.insert(place.id.as_str()),
            "重複 runtime place：{}",
            place.id
        );
        ensure!(
            place.world_id == WHITE_TOWER_WORLD_ID,
            "{} world_id 不符",
            place.id
        );
        ensure!(place.status == "active", "{} 未啟用", place.id);
        ensure!(place.traversable, "{} 不可通行", place.id);
        ensure!(!place.canon, "{} 冒充正典", place.id);
        ensure!(
            place.source_kind == WHITE_TOWER_RUNTIME_SOURCE_KIND,
            "{} source_kind 不符",
            place.id
        );
        let topology_ref = place
            .topology_node_ref
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("{} 缺 topology_node_ref", place.id))?;
        ensure!(
            topology_ids.contains(topology_ref),
            "{} 拓撲引用不存在",
            place.id
        );
        ensure!(
            place.parent_place_id.as_deref() == Some(&format!("topology:{topology_ref}")),
            "{} parent_place_id 不符",
            place.id
        );
        ensure!(
            place.legacy_coord().is_none(),
            "{} 不可用舊格座標作身分",
            place.id
        );
        ensure!(!place.name.trim().is_empty(), "{} 名稱空白", place.id);
        ensure!(
            !place.name.contains("側的") && !place.name.contains("腹地的"),
            "{} 把工程定位句冒充玩家地名：{}",
            place.id,
            place.name
        );
        let player_title = place
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix("player_title:"));
        let player_locator = place
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix("player_locator:"));
        ensure!(
            player_title.is_some_and(|title| !title.trim().is_empty()),
            "{} 缺玩家地名主層",
            place.id
        );
        ensure!(
            player_locator.is_some_and(|locator| !locator.trim().is_empty()),
            "{} 缺玩家局部定位層",
            place.id
        );
        ensure!(
            !place.name.contains('・')
                && !place
                    .name
                    .chars()
                    .last()
                    .is_some_and(|character| character.is_ascii_digit()),
            "{} 仍洩漏作者工程編號：{}",
            place.id,
            place.name
        );
        ensure!(
            !place.description.trim().is_empty(),
            "{} 描述空白",
            place.id
        );
        ensure!(
            !place.description.contains("候選") && !place.description.contains("runtime"),
            "{} 玩家描述洩漏作者後台語彙",
            place.id
        );
    }
    ensure!(
        ids.contains(world.spawn_place_id.as_str()),
        "出生場所不存在"
    );
    Ok(ids)
}

fn validate_routes(world: &WhiteTowerRuntimeWorld, place_ids: &BTreeSet<&str>) -> Result<()> {
    ensure!(world.routes.len() == 1_644, "執行路線必須完整採用 1,644 條");
    let mut ids = BTreeSet::new();
    let mut ferry_count = 0;
    let mut native_road_count = 0;
    for route in &world.routes {
        ensure!(
            ids.insert(route.id.as_str()),
            "重複 runtime route：{}",
            route.id
        );
        ensure!(
            place_ids.contains(route.from_place_id.as_str()),
            "路線起點不存在"
        );
        ensure!(
            place_ids.contains(route.to_place_id.as_str()),
            "路線終點不存在"
        );
        ensure!(route.status == "active", "{} 未啟用", route.id);
        ensure!(!route.canon, "{} 冒充正典", route.id);
        ensure!(
            route.source_kind == WHITE_TOWER_RUNTIME_SOURCE_KIND,
            "{} source_kind 不符",
            route.id
        );
        ensure!(route.travel_cost > 0, "{} 成本無效", route.id);
        ensure!(
            matches!(
                route.direction.as_str(),
                "東" | "西" | "南" | "北" | "上" | "下" | "進" | "出"
            ),
            "{} 方向無效",
            route.id
        );
        ferry_count += usize::from(route.mode == "ferry");
        native_road_count += usize::from(route.mode == "road");
    }
    ensure!(ferry_count == 2, "風舶嶼渡船雙向路線不完整");
    ensure!(native_road_count > 0, "Azgaar 原生道路沒有實際影響 runtime");
    world.graph()?;
    Ok(())
}

fn validate_profiles_and_resources(
    world: &WhiteTowerRuntimeWorld,
    place_ids: &BTreeSet<&str>,
) -> Result<()> {
    ensure!(
        world.area_profiles.len() == world.places.len(),
        "area profile 數不符"
    );
    ensure!(
        world.resources.len() == world.places.len(),
        "每個執行區都必須有物質來源"
    );
    let mut profile_ids = BTreeSet::new();
    for profile in &world.area_profiles {
        ensure!(
            place_ids.contains(profile.place_id.as_str()),
            "profile 場所不存在"
        );
        ensure!(
            profile_ids.insert(profile.place_id.as_str()),
            "重複 area profile"
        );
        ensure!(
            profile.source_cell_count > 0,
            "{} 沒有來源 cell",
            profile.place_id
        );
        ensure!((0.0..=1.0).contains(&profile.centroid.x), "profile x 越界");
        ensure!((0.0..=1.0).contains(&profile.centroid.y), "profile y 越界");
    }
    let mut resource_ids = BTreeSet::new();
    for source in &world.resources {
        ensure!(
            place_ids.contains(source.place_id.as_str()),
            "resource 場所不存在"
        );
        ensure!(
            resource_ids.insert(source.place_id.as_str()),
            "場所有重複資源"
        );
        ensure!(!source.item_id.trim().is_empty(), "resource item_id 空白");
        ensure!(!source.name.trim().is_empty(), "resource name 空白");
        ensure!(source.capacity > 0, "resource capacity 無效");
        ensure!(
            (1..=source.capacity).contains(&source.regeneration_per_day),
            "resource regeneration 無效"
        );
    }
    ensure!(
        profile_ids == resource_ids,
        "profile 與 resource 場所集合不一致"
    );
    Ok(())
}

fn validate_source_layers(world: &WhiteTowerRuntimeWorld) -> Result<()> {
    let required = BTreeMap::from([
        ("grid_cells", 100_000usize),
        ("packed_cells", 57_623),
        ("physical_areas", 305),
        ("physical_routes", 1_644),
        ("rivers", 1_129),
        ("burgs", 1_666),
        ("native_routes", 1_244),
        ("states", 23),
        ("provinces", 487),
        ("cultures", 9),
        ("religions", 14),
        ("markers", 456),
        ("zones", 15),
        ("goods", 71),
        ("markets", 54),
        ("deals", 25_808),
        ("renderer", 1),
    ]);
    let mut found = BTreeMap::new();
    for layer in &world.source_layers {
        ensure!(!layer.layer.trim().is_empty(), "source layer 名稱空白");
        ensure!(
            matches!(
                layer.disposition.as_str(),
                "runtime" | "runtime_evidence" | "retained_candidate" | "rejected_surface"
            ),
            "{} disposition 無效",
            layer.layer
        );
        ensure!(
            !layer.runtime_use.trim().is_empty(),
            "{} runtime_use 空白",
            layer.layer
        );
        ensure!(
            found
                .insert(layer.layer.as_str(), layer.source_count)
                .is_none(),
            "重複 source layer：{}",
            layer.layer
        );
    }
    ensure!(found == required, "Azgaar 來源層帳冊不完整或計數漂移");
    Ok(())
}

fn validate_coverage(world: &WhiteTowerRuntimeWorld) -> Result<()> {
    let coverage = &world.coverage;
    ensure!(coverage.physical_areas_activated == 305, "場所採用覆蓋不足");
    ensure!(
        coverage.physical_routes_activated == 1_644,
        "路線採用覆蓋不足"
    );
    ensure!(
        coverage.native_routes_accounted == 1_244,
        "原生道路沒有全數交代"
    );
    ensure!(
        coverage.native_routes_crossing_areas + coverage.native_routes_within_one_area
            == coverage.native_routes_accounted,
        "原生道路分類不守恆"
    );
    ensure!(coverage.burgs_accounted == 1_666, "聚落沒有全數交代");
    ensure!(
        coverage.burgs_associated_to_areas == coverage.burgs_accounted,
        "聚落沒有全數關聯至執行區"
    );
    ensure!(coverage.markets_accounted == 54, "市場沒有全數交代");
    ensure!(coverage.goods_accounted == 71, "商品沒有全數交代");
    ensure!(coverage.deals_accounted == 25_808, "交易沒有全數交代");
    Ok(())
}

fn validate_connectivity(world: &WhiteTowerRuntimeWorld) -> Result<()> {
    let graph = world.graph()?;
    let start = world
        .places
        .first()
        .ok_or_else(|| anyhow::anyhow!("runtime 沒有場所"))?;
    let mut visited = BTreeSet::from([start.id.as_str()]);
    let mut queue = VecDeque::from([start.id.as_str()]);
    while let Some(place_id) = queue.pop_front() {
        for route in graph.routes_from(place_id) {
            if route.status == "active" && visited.insert(route.to_place_id.as_str()) {
                queue.push_back(route.to_place_id.as_str());
            }
        }
    }
    ensure!(
        visited.len() == world.places.len(),
        "執行場所圖斷裂：只可達 {}/{}",
        visited.len(),
        world.places.len()
    );
    Ok(())
}

fn squared_distance(left: &NormalizedPoint, right: &NormalizedPoint) -> f64 {
    (left.x - right.x).powi(2) + (left.y - right.y).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_runtime_accounts_for_full_azgaar_package() {
        let world = builtin_white_tower_runtime_world().expect("runtime seed");
        assert_eq!(world.places.len(), 305);
        assert_eq!(world.routes.len(), 1_644);
        assert_eq!(world.resources.len(), 305);
        assert_eq!(world.coverage.burgs_accounted, 1_666);
        assert_eq!(world.coverage.deals_accounted, 25_808);
    }

    #[test]
    fn every_runtime_place_is_reachable_from_spawn() {
        let world = builtin_white_tower_runtime_world().expect("runtime seed");
        let graph = world.graph().expect("graph");
        for place in &world.places {
            assert!(
                graph.find_path(&world.spawn_place_id, &place.id).is_some(),
                "unreachable {}",
                place.id
            );
        }
    }
}

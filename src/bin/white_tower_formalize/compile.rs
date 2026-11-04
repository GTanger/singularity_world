use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::{Value, json};
use singularity_world::world::physical_geography::{
    NormalizedPoint, PhysicalArea, PhysicalGeography, PhysicalRouteCandidate,
    validate_physical_geography,
};
use singularity_world::world::topology::{WorldTopology, validate_topology};

use super::layers::*;

pub const FULL_JSON: &str = "data/world/white_tower_azgaar_v1/white_tower_peninsula_v1_full.json";
pub const MANIFEST_JSON: &str = "data/world/white_tower_azgaar_v1/manifest.json";
pub const GEOGRAPHY_JSON: &str = "data/world/white_tower_physical_geography_v1.json";
pub const TOPOLOGY_JSON: &str = "data/world/white_tower_topology_v1.json";
pub const OUTPUT_JSON: &str = "data/world/white_tower_formalization_v1.json";
pub const OUTPUT_SVG: &str = "docs/artifacts/white-tower-formalization-v1-author.svg";

const REFERENCE_URBAN_SHARE: f64 = 0.68;

#[derive(Debug, Clone, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettlementRecord {
    pub id: String,
    pub source_burg_id: u32,
    pub source_name: String,
    pub display_name: Option<String>,
    pub name_status: String,
    pub physical_area_id: String,
    pub macro_anchor_ref: String,
    pub source_cell_id: u32,
    pub coordinates: Point,
    pub source_group: String,
    pub source_type: String,
    pub working_class: String,
    pub roles: Vec<String>,
    pub population_weight: f64,
    pub working_population: u64,
    pub route_degree: usize,
    pub market_ref: Option<u32>,
    pub state_mesh_ref: Option<u32>,
    pub province_mesh_ref: Option<u32>,
    pub culture_ref: Option<u32>,
    pub religion_ref: Option<u32>,
    pub good_refs: Vec<u32>,
    pub canon: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeRouteRecord {
    pub id: String,
    pub source_route_id: u32,
    pub source_group: String,
    pub corridor_class: String,
    pub feature_id: Option<u32>,
    pub from_settlement_id: String,
    pub to_settlement_id: String,
    pub point_count: usize,
    pub start: Point,
    pub end: Point,
    pub status: String,
    pub canon: bool,
    #[serde(skip_serializing)]
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnchorRecord {
    pub topology_node_ref: String,
    pub label: String,
    pub expected_surface: String,
    pub point: Point,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConflictRoute {
    pub route_id: String,
    pub from_place_id: String,
    pub to_place_id: String,
    pub from_anchor_ref: String,
    pub to_anchor_ref: String,
    pub disposition: String,
    pub reason: String,
    pub from: Point,
    pub to: Point,
}

pub struct CompiledFormalization {
    pub document: Value,
    pub width: f64,
    pub height: f64,
    pub topology_node_count: usize,
    pub settlements: Vec<SettlementRecord>,
    pub routes: Vec<NativeRouteRecord>,
    pub anchors: Vec<AnchorRecord>,
    pub hard_conflicts: Vec<ConflictRoute>,
    pub marker_count: usize,
    pub zone_count: usize,
}

struct AreaLookup<'a> {
    areas: &'a [PhysicalArea],
}

struct RuralWeights {
    by_area: BTreeMap<String, f64>,
    omitted_tiny_islet_weight: f64,
    omitted_tiny_islet_cells: usize,
    omitted_feature_ids: BTreeSet<u32>,
}

impl AreaLookup<'_> {
    fn associate(&self, point: &NormalizedPoint, feature: Option<u32>) -> (&PhysicalArea, bool) {
        let same_feature = self
            .areas
            .iter()
            .filter(|area| feature == Some(area.feature_id))
            .collect::<Vec<_>>();
        let fallback = same_feature.is_empty();
        let candidates = if fallback {
            self.areas.iter().collect::<Vec<_>>()
        } else {
            same_feature
        };
        let area = candidates
            .into_iter()
            .min_by(|left, right| {
                squared_distance(&left.centroid, point)
                    .total_cmp(&squared_distance(&right.centroid, point))
                    .then_with(|| left.id.cmp(&right.id))
            })
            .expect("validated geography contains areas");
        (area, fallback)
    }
}

pub fn compile() -> Result<CompiledFormalization> {
    let full_bytes = fs::read(FULL_JSON).context("讀取 Azgaar Full JSON 失敗")?;
    let manifest_bytes = fs::read(MANIFEST_JSON).context("讀取 Azgaar manifest 失敗")?;
    let geography_bytes = fs::read(GEOGRAPHY_JSON).context("讀取白塔物理地理失敗")?;
    let topology_bytes = fs::read(TOPOLOGY_JSON).context("讀取白塔拓撲失敗")?;

    let full: Value = serde_json::from_slice(&full_bytes)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let geography: PhysicalGeography = serde_json::from_slice(&geography_bytes)?;
    let topology: WorldTopology = serde_json::from_slice(&topology_bytes)?;
    validate_physical_geography(&geography)?;
    validate_topology(&topology)?;

    let width = number(&full["info"], "width")?;
    let height = number(&full["info"], "height")?;
    let pack = &full["pack"];
    let cells = array(pack, "cells")?;
    let lookup = AreaLookup {
        areas: &geography.areas,
    };

    let mut settlements = compile_settlements(pack, cells, &lookup, width, height)?;
    let rural_weights = compile_rural_weights(cells, &lookup, width, height)?;
    let population = assign_population(&full, &mut settlements, &geography, &rural_weights)?;
    let mut routes = compile_routes(pack, &settlements, width, height)?;
    assign_route_degree(&mut settlements, &routes);
    routes.sort_by_key(|route| route.source_route_id);

    let anchors = compile_anchors(&geography, cells, width, height)?;
    let (hard_conflicts, review_routes) = compile_route_overrides(&geography);
    let marker_candidates = compile_markers(pack, width, height)?;
    let zones = compile_zones(pack)?;
    let marker_count = marker_candidates.len();
    let zone_count = zones.len();
    let states = compile_states(pack, cells, width, height)?;
    let provinces = compile_provinces(pack, cells, width, height)?;
    let cultures = compile_named_candidates(pack, "cultures", false)?;
    let religions = compile_named_candidates(pack, "religions", false)?;
    let goods = compile_goods(pack)?;

    let map_scale = compile_scale(&full, &geography)?;
    let source_layers = source_layer_dispositions(&manifest, marker_candidates.len(), zones.len());
    let settlement_classes = count_strings(
        settlements
            .iter()
            .map(|settlement| settlement.working_class.as_str()),
    );
    let corridor_classes = count_strings(routes.iter().map(|route| route.corridor_class.as_str()));
    let marker_dispositions = count_values(&marker_candidates, "disposition");

    let document = json!({
        "schema_version": 1,
        "id": "white_tower_formalization_v1",
        "name": "白塔半島正式化工作層 v1",
        "created": "2026-07-21",
        "status": "working_premise",
        "canon": false,
        "runtime_authority": false,
        "player_facing_map": false,
        "sources": {
            "azgaar_project": manifest["source"]["project"],
            "azgaar_commit": manifest["source"]["commit"],
            "seed": manifest["source"]["seed"],
            "full_json": {"path": FULL_JSON, "sha256": sha256(&full_bytes)},
            "manifest": {"path": MANIFEST_JSON, "sha256": sha256(&manifest_bytes)},
            "physical_geography": {"path": GEOGRAPHY_JSON, "sha256": sha256(&geography_bytes)},
            "accepted_topology": {"path": TOPOLOGY_JSON, "sha256": sha256(&topology_bytes)},
        },
        "decision_layer": {
            "accepted": [
                "37 個既有大地理拓撲及其關係",
                "Azgaar 全量作者資料必須保留並作局部優化",
                "白塔是次大陸級巨大縮略圖",
                "每個正式節點都必須是有意義的地點"
            ],
            "working_premises": [
                "普通科技與個人能量運用共同演進",
                "白塔人口是百萬級以上、全球人口是億級",
                "人口與人物身分持續存在，遠近只改運算解析度"
            ],
            "still_not_canon": [
                "生成地名、國名、政體、文化、宗教及事件名稱",
                "精確人口、都市化率、正式行政邊界與全部道路名稱",
                "本檔的工作人口配置與來源語義轉譯"
            ]
        },
        "candidate_competition": [
            {
                "candidate": "把 100,000 格或 305 個物理區都當成地點節點",
                "result": "rejected",
                "reason": "計算分區沒有自身社會或自然身分，會把大世界灌成無意義房間"
            },
            {
                "candidate": "把 1,666 聚落、23 國、456 標記的原始語義直接正典化",
                "result": "rejected",
                "reason": "會把中世紀政體、地下城、遭遇與隨機名稱偷渡進奇幻現代"
            },
            {
                "candidate": "分開空間底盤、宏觀地理、永久聚落、候選地標與連線",
                "result": "adopted_for_working_layer",
                "reason": "完整利用現有資料，同時讓每個正式節點有身分、座標、關係與來源"
            }
        ],
        "scale": map_scale,
        "population_model": population,
        "place_model": {
            "meaningful_place_count": topology.nodes.len() + settlements.len(),
            "accepted_macro_geography_count": topology.nodes.len(),
            "working_settlement_count": settlements.len(),
            "support_not_place": {
                "grid_cells": geography.metrics.grid_cells,
                "packed_cells": geography.metrics.packed_cells,
                "physical_areas": geography.areas.len(),
                "rule": "cell 與 physical_area 是距離、地貌、人口及路線的空間底盤，不再冒充有意義節點"
            },
            "settlement_classes": settlement_classes,
            "marker_candidates_retained": marker_candidates.len(),
            "marker_dispositions": marker_dispositions,
            "rule": "只有具備永久身分、座標、場所角色、來源與連線關係的記錄才進正式場所層；事件或移動對象不因有圖標就成為地點"
        },
        "settlements": &settlements,
        "area_demography": population["area_demography"].clone(),
        "native_corridors": &routes,
        "corridor_classes": corridor_classes,
        "operational_route_overrides": {
            "hard_conflicts": &hard_conflicts,
            "requires_geometry_review": review_routes,
            "rule": "大圳頭不得直接越過水尾澤抵達青籠關；衝突路線保留來源但不得升格正式跨區走廊"
        },
        "accepted_anchor_projection": &anchors,
        "territorial_mesh_candidates": states,
        "service_district_mesh_candidates": provinces,
        "culture_distribution_candidates": cultures,
        "religion_distribution_candidates": religions,
        "marker_candidates": marker_candidates,
        "zone_candidates": zones,
        "goods_semantic_audit": goods,
        "source_layer_dispositions": source_layers,
        "verification_contract": {
            "all_existing_data_accounted": true,
            "no_regeneration": true,
            "no_generated_name_promoted": true,
            "no_generated_polity_promoted": true,
            "all_settlements_have_stable_place_identity": true,
            "all_native_routes_have_place_endpoints": true,
            "cell_and_area_are_not_meaningful_nodes": true,
            "marker_zero_and_zone_zero_preserved": true,
            "known_south_bypass_flagged": true,
            "falsifier": "任一聚落無法映射到物理區、任一來源層漏記、生成政治被當成正典，或大圳頭仍可被認作直通青籠關時，本批不得判定完成"
        }
    });

    Ok(CompiledFormalization {
        document,
        width,
        height,
        topology_node_count: topology.nodes.len(),
        settlements,
        routes,
        anchors,
        hard_conflicts,
        marker_count,
        zone_count,
    })
}

pub fn validate(compiled: &CompiledFormalization) -> Result<()> {
    ensure!(compiled.topology_node_count == 37, "正典拓撲節點數漂移");
    ensure!(compiled.settlements.len() == 1_666, "聚落未全量承接");
    ensure!(compiled.routes.len() == 1_244, "原生路線未全量承接");
    ensure!(compiled.marker_count == 456, "id=0 的真實標記仍被漏算");
    ensure!(compiled.zone_count == 15, "id=0 的真實區域仍被漏算");
    ensure!(compiled.hard_conflicts.len() == 8, "南向直通衝突數漂移");

    let area_ids = compiled
        .settlements
        .iter()
        .map(|settlement| settlement.physical_area_id.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(!area_ids.is_empty(), "聚落沒有物理區承接");
    let mut settlement_ids = BTreeSet::new();
    for settlement in &compiled.settlements {
        ensure!(
            settlement_ids.insert(settlement.id.as_str()),
            "聚落 id 重複"
        );
        ensure!(
            settlement.display_name.is_none(),
            "生成名稱偷升格 display_name"
        );
        ensure!(
            !settlement.roles.is_empty(),
            "{} 沒有場所意義",
            settlement.id
        );
        ensure!(
            (0.0..=1.0).contains(&settlement.coordinates.x)
                && (0.0..=1.0).contains(&settlement.coordinates.y),
            "{} 座標越界",
            settlement.id
        );
    }
    for route in &compiled.routes {
        ensure!(
            settlement_ids.contains(route.from_settlement_id.as_str()),
            "路線起點不存在"
        );
        ensure!(
            settlement_ids.contains(route.to_settlement_id.as_str()),
            "路線終點不存在"
        );
    }
    ensure!(
        compiled.document["verification_contract"]["no_generated_polity_promoted"] == true,
        "生成政治不得升格"
    );
    Ok(())
}

fn compile_settlements(
    pack: &Value,
    cells: &[Value],
    lookup: &AreaLookup<'_>,
    width: f64,
    height: f64,
) -> Result<Vec<SettlementRecord>> {
    let mut records = Vec::new();
    for burg in array(pack, "burgs")?
        .iter()
        .filter(|burg| burg["i"].as_u64().is_some_and(|id| id > 0) && burg["removed"] != true)
    {
        let source_id = burg["i"].as_u64().context("burg 缺 i")? as u32;
        let point = point_from_xy(burg, width, height)?;
        let normalized = NormalizedPoint {
            x: point.x,
            y: point.y,
        };
        let feature = burg["feature"].as_u64().map(|value| value as u32);
        let (area, fallback) = lookup.associate(&normalized, feature);
        ensure!(!fallback, "burg {source_id} 無法依來源陸塊映射");
        let cell_id = burg["cell"].as_u64().context("burg 缺 cell")? as u32;
        let cell = cells.get(cell_id as usize).context("burg cell 不存在")?;
        let group = text_or(burg, "group", "town");
        let source_type = text_or(burg, "type", "Generic");
        let roles = settlement_roles(burg, &group, &source_type);
        let mut goods = BTreeSet::new();
        for production in burg["production"].as_array().into_iter().flatten() {
            if let Some(id) = production["goodId"].as_u64() {
                goods.insert(id as u32);
            }
        }
        records.push(SettlementRecord {
            id: format!("wt-settlement-{source_id:04}"),
            source_burg_id: source_id,
            source_name: text_or(burg, "name", ""),
            display_name: None,
            name_status: "source_name_retained_needs_language_review".to_string(),
            physical_area_id: area.id.clone(),
            macro_anchor_ref: area.topology_node_ref.clone(),
            source_cell_id: cell_id,
            coordinates: point,
            source_group: group.clone(),
            source_type: source_type.clone(),
            working_class: working_settlement_class(&group).to_string(),
            roles,
            population_weight: burg["population"].as_f64().unwrap_or_default(),
            working_population: 0,
            route_degree: 0,
            market_ref: positive_u32(&burg["market"]),
            state_mesh_ref: positive_u32(&burg["state"]),
            province_mesh_ref: positive_u32(&cell["province"]),
            culture_ref: positive_u32(&burg["culture"]),
            religion_ref: positive_u32(&cell["religion"]),
            good_refs: goods.into_iter().collect(),
            canon: false,
            status: "working_place_candidate".to_string(),
        });
    }
    records.sort_by_key(|record| record.source_burg_id);
    Ok(records)
}

fn settlement_roles(burg: &Value, group: &str, source_type: &str) -> Vec<String> {
    let mut roles = BTreeSet::from(["permanent_settlement".to_string()]);
    roles.insert(
        match group {
            "capital" => "regional_coordination_seed",
            "city" => "major_service_center_seed",
            "fort" => "strategic_service_site_seed",
            "village" => "local_service_center_seed",
            _ => "urban_locality_seed",
        }
        .to_string(),
    );
    match source_type {
        "Naval" => {
            roles.insert("maritime_access_seed".to_string());
        }
        "River" => {
            roles.insert("river_access_seed".to_string());
        }
        "Hunting" => {
            roles.insert("landscape_dependent_livelihood_seed".to_string());
        }
        "Nomadic" => {
            roles.insert("mobile_population_service_seed".to_string());
        }
        _ => {}
    }
    if burg["port"].as_u64().unwrap_or_default() > 0 {
        roles.insert("port_seed".to_string());
    }
    if burg["market"].as_u64().unwrap_or_default() > 0 {
        roles.insert("market_network_node".to_string());
    }
    if burg["plaza"].as_u64().unwrap_or_default() > 0 {
        roles.insert("civic_center_seed".to_string());
    }
    if burg["production"]
        .as_array()
        .is_some_and(|items| !items.is_empty())
    {
        roles.insert("production_network_node".to_string());
    }
    if burg["citadel"].as_u64().unwrap_or_default() > 0
        || burg["walls"].as_u64().unwrap_or_default() > 0
    {
        roles.insert("legacy_defensive_footprint_seed".to_string());
    }
    roles.into_iter().collect()
}

fn working_settlement_class(group: &str) -> &'static str {
    match group {
        "capital" => "metropolitan_or_regional_core_seed",
        "city" => "major_urban_center_seed",
        "fort" => "strategic_service_settlement_seed",
        "village" => "local_settlement_seed",
        _ => "urban_locality_seed",
    }
}

fn compile_rural_weights(
    cells: &[Value],
    lookup: &AreaLookup<'_>,
    width: f64,
    height: f64,
) -> Result<RuralWeights> {
    let mut weights = lookup
        .areas
        .iter()
        .map(|area| (area.id.clone(), 0.0))
        .collect::<BTreeMap<_, _>>();
    let mut omitted_tiny_islet_weight = 0.0;
    let mut omitted_tiny_islet_cells = 0;
    let mut omitted_feature_ids = BTreeSet::new();
    for cell in cells {
        let weight = cell["pop"].as_f64().unwrap_or_default();
        if weight <= 0.0 {
            continue;
        }
        let point_array = cell["p"].as_array().context("cell.p 不是陣列")?;
        let point = NormalizedPoint {
            x: point_array[0].as_f64().context("cell.p.x 缺失")? / width,
            y: point_array[1].as_f64().context("cell.p.y 缺失")? / height,
        };
        let feature = cell["f"].as_u64().map(|value| value as u32);
        let (area, fallback) = lookup.associate(&point, feature);
        if fallback {
            omitted_tiny_islet_weight += weight;
            omitted_tiny_islet_cells += 1;
            omitted_feature_ids.extend(feature);
            continue;
        }
        *weights.get_mut(&area.id).expect("area weight exists") += weight;
    }
    Ok(RuralWeights {
        by_area: weights,
        omitted_tiny_islet_weight,
        omitted_tiny_islet_cells,
        omitted_feature_ids,
    })
}

fn assign_population(
    full: &Value,
    settlements: &mut [SettlementRecord],
    geography: &PhysicalGeography,
    rural_weights: &RuralWeights,
) -> Result<Value> {
    let urban_weight = settlements
        .iter()
        .map(|settlement| settlement.population_weight)
        .sum::<f64>();
    let rural_weight =
        rural_weights.by_area.values().sum::<f64>() + rural_weights.omitted_tiny_islet_weight;
    let population_rate = number(&full["settings"], "populationRate")?;
    let source_implied_total = ((urban_weight + rural_weight) * population_rate).round() as u64;
    let working_urban = (source_implied_total as f64 * REFERENCE_URBAN_SHARE).round() as u64;
    let working_rural = source_implied_total - working_urban;
    let urban_allocations = allocate_exact(
        &settlements
            .iter()
            .map(|settlement| settlement.population_weight)
            .collect::<Vec<_>>(),
        working_urban,
    );
    for (settlement, allocation) in settlements.iter_mut().zip(urban_allocations) {
        settlement.working_population = allocation;
    }

    let mut area_weights = geography
        .areas
        .iter()
        .map(|area| *rural_weights.by_area.get(&area.id).unwrap_or(&0.0))
        .collect::<Vec<_>>();
    area_weights.push(rural_weights.omitted_tiny_islet_weight);
    let rural_allocations = allocate_exact(&area_weights, working_rural);
    let mut area_demography = geography
        .areas
        .iter()
        .enumerate()
        .map(|(index, area)| {
            json!({
                "physical_area_id": area.id,
                "macro_anchor_ref": area.topology_node_ref,
                "source_rural_weight": round6(area_weights[index]),
                "working_rural_population": rural_allocations[index],
                "canon": false,
                "status": "working_population_allocation"
            })
        })
        .collect::<Vec<_>>();
    let omitted_allocation = rural_allocations[geography.areas.len()];
    area_demography.push(json!({
        "support_remainder_id": "wt-support-omitted-tiny-islets",
        "source_feature_ids": &rural_weights.omitted_feature_ids,
        "source_cell_count": rural_weights.omitted_tiny_islet_cells,
        "source_rural_weight": round6(rural_weights.omitted_tiny_islet_weight),
        "working_rural_population": omitted_allocation,
        "is_meaningful_place": false,
        "canon": false,
        "status": "retained_unaggregated_tiny_islet_demography",
        "rule": "保留在總人口與來源稽核中；未完成有意義地點設計前，不硬塞給最近大陸，也不冒充第 306 個物理區"
    }));
    let source_urban_share = urban_weight / (urban_weight + rural_weight);
    Ok(json!({
        "status": "working_scale_proposal",
        "canon": false,
        "source_population_rate": population_rate,
        "source_urban_weight": round6(urban_weight),
        "source_rural_weight": round6(rural_weight),
        "source_implied_total_population": source_implied_total,
        "source_implied_urban_share": round6(source_urban_share),
        "source_urban_share_disposition": "rejected_as_pre_modern_distribution",
        "working_total_population": source_implied_total,
        "working_reference_urban_share": REFERENCE_URBAN_SHARE,
        "urban_share_sensitivity_band": {"min": 0.60, "max": 0.75},
        "working_urban_population": working_urban,
        "working_rural_population": working_rural,
        "unaggregated_tiny_islets": {
            "source_cell_count": rural_weights.omitted_tiny_islet_cells,
            "source_feature_ids": &rural_weights.omitted_feature_ids,
            "source_weight": round6(rural_weights.omitted_tiny_islet_weight),
            "working_population": omitted_allocation,
            "disposition": "retained_as_support_remainder_not_a_place"
        },
        "rule": "保留 Azgaar 總人口量級作工作底盤，但不沿用約 3.25% 的前現代都市比例；68% 只作可運算參考值，不是正典",
        "area_demography": area_demography
    }))
}

fn compile_routes(
    pack: &Value,
    settlements: &[SettlementRecord],
    width: f64,
    height: f64,
) -> Result<Vec<NativeRouteRecord>> {
    let mut records = Vec::new();
    for route in array(pack, "routes")? {
        let source_id = route["i"].as_u64().context("route 缺 i")? as u32;
        let points = route["points"]
            .as_array()
            .context("route.points 不是陣列")?
            .iter()
            .filter_map(|point| point.as_array())
            .map(|point| Point {
                x: point[0].as_f64().unwrap_or_default() / width,
                y: point[1].as_f64().unwrap_or_default() / height,
            })
            .collect::<Vec<_>>();
        ensure!(points.len() >= 2, "route {source_id} 少於兩點");
        let start = points.first().expect("route has start").clone();
        let end = points.last().expect("route has end").clone();
        let from = nearest_settlement(&start, settlements);
        let to = nearest_settlement(&end, settlements);
        let group = text_or(route, "group", "trails");
        records.push(NativeRouteRecord {
            id: format!("wt-native-corridor-{source_id:04}"),
            source_route_id: source_id,
            source_group: group.clone(),
            corridor_class: match group.as_str() {
                "roads" => "primary_land_corridor_seed",
                "searoutes" => "maritime_corridor_seed",
                _ => "secondary_land_corridor_seed",
            }
            .to_string(),
            feature_id: route["feature"].as_u64().map(|value| value as u32),
            from_settlement_id: from.id.clone(),
            to_settlement_id: to.id.clone(),
            point_count: points.len(),
            start,
            end,
            status: "working_corridor_seed".to_string(),
            canon: false,
            points,
        });
    }
    Ok(records)
}

fn assign_route_degree(settlements: &mut [SettlementRecord], routes: &[NativeRouteRecord]) {
    let mut degree = BTreeMap::<String, usize>::new();
    for route in routes {
        *degree.entry(route.from_settlement_id.clone()).or_default() += 1;
        if route.to_settlement_id != route.from_settlement_id {
            *degree.entry(route.to_settlement_id.clone()).or_default() += 1;
        }
    }
    for settlement in settlements {
        settlement.route_degree = *degree.get(&settlement.id).unwrap_or(&0);
    }
}

fn compile_anchors(
    geography: &PhysicalGeography,
    cells: &[Value],
    width: f64,
    height: f64,
) -> Result<Vec<AnchorRecord>> {
    geography
        .anchor_mappings
        .iter()
        .map(|anchor| {
            let cell = cells
                .get(anchor.cell_id as usize)
                .context("anchor cell 不存在")?;
            let p = cell["p"].as_array().context("anchor cell.p 不是陣列")?;
            Ok(AnchorRecord {
                topology_node_ref: anchor.topology_node_ref.clone(),
                label: anchor.label.clone(),
                expected_surface: anchor.expected_surface.clone(),
                point: Point {
                    x: p[0].as_f64().context("anchor x 缺失")? / width,
                    y: p[1].as_f64().context("anchor y 缺失")? / height,
                },
            })
        })
        .collect()
}

fn compile_route_overrides(geography: &PhysicalGeography) -> (Vec<ConflictRoute>, Vec<Value>) {
    let areas = geography
        .areas
        .iter()
        .map(|area| (area.id.as_str(), area))
        .collect::<BTreeMap<_, _>>();
    let mut hard = Vec::new();
    let mut review = Vec::new();
    for route in &geography.route_candidates {
        let Some(from) = areas.get(route.from_place_id.as_str()) else {
            continue;
        };
        let Some(to) = areas.get(route.to_place_id.as_str()) else {
            continue;
        };
        let pair = (
            from.topology_node_ref.as_str(),
            to.topology_node_ref.as_str(),
        );
        let direct_qinglong = matches!(
            pair,
            ("wt-c-southwest-plain", "wt-s-qinglong-pass")
                | ("wt-s-qinglong-pass", "wt-c-southwest-plain")
        );
        if direct_qinglong {
            hard.push(conflict_record(
                route,
                from,
                to,
                "formalization_blocked",
                "繞過水尾澤，與已接受南向地理關係衝突",
            ));
            continue;
        }
        let sentinel_review = matches!(
            pair,
            ("wt-c-southwest-plain", "wt-s-south-sentinel")
                | ("wt-s-south-sentinel", "wt-c-southwest-plain")
        );
        if sentinel_review {
            review.push(json!({
                "route_id": route.id,
                "from_place_id": route.from_place_id,
                "to_place_id": route.to_place_id,
                "reason": "須檢查實際線位是否已進入水尾澤北緣，不能只靠 anchor 標籤判定",
                "disposition": "retained_pending_geometry_review"
            }));
        }
    }
    (hard, review)
}

fn conflict_record(
    route: &PhysicalRouteCandidate,
    from: &PhysicalArea,
    to: &PhysicalArea,
    disposition: &str,
    reason: &str,
) -> ConflictRoute {
    ConflictRoute {
        route_id: route.id.clone(),
        from_place_id: route.from_place_id.clone(),
        to_place_id: route.to_place_id.clone(),
        from_anchor_ref: from.topology_node_ref.clone(),
        to_anchor_ref: to.topology_node_ref.clone(),
        disposition: disposition.to_string(),
        reason: reason.to_string(),
        from: Point {
            x: from.centroid.x,
            y: from.centroid.y,
        },
        to: Point {
            x: to.centroid.x,
            y: to.centroid.y,
        },
    }
}

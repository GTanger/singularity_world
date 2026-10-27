//! Azgaar 白塔物理地理 JSON 的純資料模型；不含驗證、轉換或副作用。

mod hydrology;

use serde::{Deserialize, Serialize};

pub use hydrology::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalGeography {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub status: String,
    pub canon: bool,
    pub runtime_authority: bool,
    pub player_facing_map: bool,
    pub source: GeographySource,
    pub coordinate_space: CoordinateSpace,
    pub metrics: GeographyMetrics,
    pub land_features: Vec<LandFeature>,
    pub biomes: Vec<Biome>,
    pub anchor_mappings: Vec<AnchorMapping>,
    pub areas: Vec<PhysicalArea>,
    pub route_candidates: Vec<PhysicalRouteCandidate>,
    pub rivers: Vec<PhysicalRiver>,
    pub major_hydrology: MajorHydrology,
    pub invariants: GeographyInvariants,
    pub source_dispositions: SourceDispositions,
    pub generation_evidence: GenerationEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeographySource {
    pub project: String,
    pub commit: String,
    pub seed: String,
    pub heightmap_profile: String,
    pub canon_authority: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinateSpace {
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub physical_aggregate_size: f64,
    pub formal_scale_decided: bool,
    pub coordinate_system_decided: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeographyMetrics {
    pub grid_cells: usize,
    pub packed_cells: usize,
    pub generated_land_cells: usize,
    pub aggregated_land_cells: usize,
    pub omitted_tiny_islet_cells: usize,
    pub physical_areas: usize,
    pub route_candidates: usize,
    pub rivers: usize,
    pub lakes: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LandFeature {
    pub id: u32,
    #[serde(rename = "type")]
    pub feature_type: String,
    pub group: String,
    pub cells: usize,
    pub area: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Biome {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorMapping {
    pub topology_node_ref: String,
    pub label: String,
    pub expected_surface: String,
    pub cell_id: u32,
    pub normalized_distance: f64,
    pub height: u8,
    pub biome: u32,
    pub biome_name: String,
    pub river: u32,
    pub area_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalArea {
    pub id: String,
    pub world_id: String,
    pub topology_node_ref: String,
    pub parent_place_id: String,
    pub name: String,
    pub kind: String,
    pub scale: String,
    pub canon: bool,
    pub status: String,
    pub traversable: bool,
    pub source_kind: String,
    pub terrain_class: String,
    pub elevation_band: String,
    pub biome_names: Vec<String>,
    pub feature_id: u32,
    pub source_cell_count: usize,
    pub centroid: NormalizedPoint,
    pub bounds: NormalizedBounds,
    pub height: HeightRange,
    pub river_cell_count: usize,
    pub major_river_ids: Vec<u32>,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NormalizedPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeightRange {
    pub min: u8,
    pub mean: f64,
    pub max: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalRouteCandidate {
    pub id: String,
    pub from_place_id: String,
    pub direction: String,
    pub to_place_id: String,
    pub mode: String,
    pub relative_cost: i32,
    pub shared_border_cells: usize,
    pub via_topology_node_ref: Option<String>,
    pub canon: bool,
    pub status: String,
    pub source_kind: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalRiver {
    pub source_river_id: u32,
    pub parent_river_id: u32,
    pub basin_id: u32,
    pub source_cell_id: Option<u32>,
    pub mouth_cell_id: Option<u32>,
    pub length: Option<f64>,
    pub discharge: Option<f64>,
    pub width: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeographyInvariants {
    pub one_mainland_and_windship_island: bool,
    pub windship_channel_remains_water: bool,
    pub white_tower_heads_green_ridge: bool,
    pub green_ridge_meets_east_sea: bool,
    pub three_source_valleys_meet_near_wenyao: bool,
    pub lower_river_reaches_black_marsh: bool,
    pub black_marsh_is_wetland_not_lake: bool,
    pub central_forest_is_not_black_marsh: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDispositions {
    pub kept: Vec<String>,
    pub candidate_only: Vec<String>,
    pub rejected: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationEvidence {
    pub console_errors: Vec<String>,
    pub console_warnings: Vec<String>,
    pub rejected_surface_errors: Vec<String>,
    pub page_errors: Vec<String>,
}

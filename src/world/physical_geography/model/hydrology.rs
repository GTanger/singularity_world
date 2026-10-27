//! 白塔半島主要水系映射的純資料模型；精確河道仍是作者候選，不是正典。

use serde::{Deserialize, Serialize};

use super::NormalizedPoint;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MajorHydrology {
    pub status: String,
    pub canon: bool,
    pub confluence_cell_id: u32,
    pub confluence_point: NormalizedPoint,
    pub watercourses: Vec<MajorWatercourse>,
    pub invariants: MajorHydrologyInvariants,
    pub selection_diagnostics: Vec<HydrologySelectionDiagnostic>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MajorWatercourse {
    pub topology_node_ref: String,
    pub structural_label: String,
    pub role: String,
    pub status: String,
    pub canon: bool,
    pub start_river_id: u32,
    pub basin_id: u32,
    pub start_cell_id: u32,
    pub end_cell_id: u32,
    pub source_fit_distance: f64,
    pub confluence_fit_distance: Option<f64>,
    pub black_marsh_fit_distance: Option<f64>,
    pub generated_river_ids: Vec<u32>,
    pub source_cell_count: usize,
    pub start_point: NormalizedPoint,
    pub end_point: NormalizedPoint,
    pub path: Vec<NormalizedPoint>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MajorHydrologyInvariants {
    pub distinct_headwaters: bool,
    pub shared_basin: bool,
    pub confluence_near_wenyao: bool,
    pub lower_river_reaches_black_marsh: bool,
    pub lower_river_continues_south: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HydrologySelectionDiagnostic {
    pub topology_node_ref: String,
    pub score: f64,
    pub checkpoint_distances: Vec<f64>,
}

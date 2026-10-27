use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use singularity_world::world::physical_geography::{NormalizedPoint, PhysicalGeography};

use super::compile::{Point, SettlementRecord};

pub(super) fn compile_markers(pack: &Value, width: f64, height: f64) -> Result<Vec<Value>> {
    array(pack, "markers")?
        .iter()
        .filter(|marker| marker["i"].as_u64().is_some() && marker["removed"] != true)
        .map(|marker| {
            let marker_type = text_or(marker, "type", "unknown");
            Ok(json!({
                "source_marker_id": marker["i"],
                "source_type": marker_type,
                "coordinates": point_from_xy(marker, width, height)?,
                "source_cell_id": marker["cell"],
                "disposition": marker_disposition(&marker_type),
                "canon": false,
                "status": "retained_candidate"
            }))
        })
        .collect()
}

fn marker_disposition(marker_type: &str) -> &'static str {
    match marker_type {
        "volcanoes" | "water-sources" | "caves" | "hot-springs" => {
            "geographic_site_seed_requires_grounding"
        }
        "bridges" | "mines" | "libraries" | "inns" | "circuses" | "fairs" => {
            "infrastructure_or_civic_site_seed_requires_modernization"
        }
        "ruins" | "battlefields" | "migration" | "necropolises" | "statues" | "sacred-pineries"
        | "sacred-forests" => "historical_or_cultural_claim_seed",
        "dungeons" | "encounters" | "brigands" | "pirates" | "hill-monsters" | "sea-monsters"
        | "canoes" | "dances" | "jousts" => "event_actor_or_genre_seed_not_a_place",
        _ => "retained_unclassified_candidate",
    }
}

pub(super) fn compile_zones(pack: &Value) -> Result<Vec<Value>> {
    Ok(array(pack, "zones")?
        .iter()
        .filter(|zone| zone["i"].as_u64().is_some() && zone["removed"] != true)
        .map(|zone| {
            json!({
                "source_zone_id": zone["i"],
                "source_name": zone["name"],
                "source_type": zone["type"],
                "source_cell_count": zone["cells"].as_array().map_or(0, Vec::len),
                "disposition": "event_or_historical_overlay_seed_not_a_geographic_region",
                "canon": false,
                "status": "retained_candidate"
            })
        })
        .collect())
}

pub(super) fn compile_states(
    pack: &Value,
    cells: &[Value],
    width: f64,
    height: f64,
) -> Result<Vec<Value>> {
    array(pack, "states")?
        .iter()
        .filter(|state| state["i"].as_u64().is_some_and(|id| id > 0) && state["removed"] != true)
        .map(|state| {
            let center = point_from_cell(cells, state["center"].as_u64(), width, height)?;
            Ok(json!({
                "source_state_id": state["i"],
                "source_name": state["name"],
                "source_form": state["form"],
                "center": center,
                "source_neighbor_refs": state["neighbors"],
                "source_province_refs": state["provinces"],
                "disposition": "boundary_mesh_candidate_not_sovereignty",
                "canon": false,
                "status": "retained_candidate"
            }))
        })
        .collect()
}

pub(super) fn compile_provinces(
    pack: &Value,
    cells: &[Value],
    width: f64,
    height: f64,
) -> Result<Vec<Value>> {
    array(pack, "provinces")?
        .iter()
        .filter(|province| {
            province["i"].as_u64().is_some_and(|id| id > 0) && province["removed"] != true
        })
        .map(|province| {
            let center = point_from_cell(cells, province["center"].as_u64(), width, height)?;
            Ok(json!({
                "source_province_id": province["i"],
                "source_name": province["name"],
                "source_state_ref": province["state"],
                "source_burg_ref": province["burg"],
                "center": center,
                "disposition": "local_service_mesh_candidate_not_administration",
                "canon": false,
                "status": "retained_candidate"
            }))
        })
        .collect()
}

pub(super) fn compile_named_candidates(
    pack: &Value,
    layer: &str,
    zero_is_valid: bool,
) -> Result<Vec<Value>> {
    Ok(array(pack, layer)?
        .iter()
        .filter(|item| {
            item["i"].as_u64().is_some_and(|id| zero_is_valid || id > 0) && item["removed"] != true
        })
        .map(|item| {
            json!({
                "source_id": item["i"],
                "source_name": item["name"],
                "source_type": item["type"],
                "disposition": "spatial_distribution_seed_name_and_traits_not_canon",
                "canon": false,
                "status": "retained_candidate"
            })
        })
        .collect())
}

pub(super) fn compile_goods(pack: &Value) -> Result<Vec<Value>> {
    Ok(array(pack, "goods")?
        .iter()
        .map(|good| {
            let id = good["i"].as_u64().unwrap_or_default();
            json!({
                "source_good_id": id,
                "source_name": good["name"],
                "source_tags": good["tags"],
                "source_unit": good["unit"],
                "disposition": if id == 31 {
                    "personhood_record_not_a_normal_legal_commodity"
                } else {
                    "material_or_service_semantics_pending_modernization"
                },
                "canon": false,
                "status": "retained_candidate"
            })
        })
        .collect())
}

pub(super) fn compile_scale(full: &Value, geography: &PhysicalGeography) -> Result<Value> {
    let coordinates = &full["mapCoordinates"];
    let lat_n = number(coordinates, "latN")?;
    let lat_s = number(coordinates, "latS")?;
    let lon_w = number(coordinates, "lonW")?;
    let lon_e = number(coordinates, "lonE")?;
    let radius = 6_371.0;
    let surface = radius
        * radius
        * (lon_e - lon_w).to_radians()
        * (lat_n.to_radians().sin() - lat_s.to_radians().sin());
    let land_fraction =
        geography.metrics.generated_land_cells as f64 / geography.metrics.grid_cells as f64;
    let land_area = surface * land_fraction;
    let north_south = radius * (lat_n - lat_s).to_radians();
    let east_west =
        radius * ((lat_n + lat_s) / 2.0).to_radians().cos() * (lon_e - lon_w).to_radians();
    Ok(json!({
        "coordinate_status": "working_scale_candidate",
        "canon": false,
        "latitude_north": lat_n,
        "latitude_south": lat_s,
        "longitude_west": lon_w,
        "longitude_east": lon_e,
        "north_south_span_km": round1(north_south),
        "mid_latitude_east_west_span_km": round1(east_west),
        "author_frame_surface_area_km2": round1(surface),
        "land_fraction": round6(land_fraction),
        "working_land_area_km2": round1(land_area),
        "average_packed_land_cell_km2": round1(land_area / geography.metrics.generated_land_cells as f64),
        "average_physical_area_km2": round1(land_area / geography.areas.len() as f64),
        "interpretation": "白塔是約 2400 公里南北跨度、約 389 萬平方公里陸地量級的次大陸；cell 與 305 區域都太大，不能直接等同玩家地點"
    }))
}

pub(super) fn source_layer_dispositions(
    manifest: &Value,
    marker_count: usize,
    zone_count: usize,
) -> Value {
    let generated = &manifest["generated"];
    json!([
        {"layer":"grid_cells","count":manifest["source"]["gridCells"],"disposition":"spatial_support_not_place"},
        {"layer":"packed_cells","count":manifest["source"]["packedCells"],"disposition":"spatial_support_not_place"},
        {"layer":"physical_areas","count":305,"disposition":"operational_spatial_support_not_meaningful_place"},
        {"layer":"accepted_topology","count":37,"disposition":"accepted_macro_meaningful_places"},
        {"layer":"burgs","count":generated["burgCandidates"],"disposition":"working_meaningful_settlement_candidates"},
        {"layer":"native_routes","count":generated["routeCandidates"],"disposition":"working_corridor_seeds"},
        {"layer":"rivers","count":generated["rivers"],"disposition":"hydrology_evidence_grouped_not_one_place_per_fragment"},
        {"layer":"states","count":generated["stateCandidates"],"disposition":"boundary_mesh_candidates_not_sovereignty"},
        {"layer":"provinces","count":generated["provinceCandidates"],"disposition":"service_mesh_candidates_not_administration"},
        {"layer":"cultures","count":generated["cultureCandidates"],"disposition":"distribution_seeds_names_and_traits_not_canon"},
        {"layer":"religions","count":generated["religionCandidates"],"disposition":"distribution_seeds_names_and_traits_not_canon"},
        {"layer":"markers","count":marker_count,"disposition":"retained_and_typed_candidates"},
        {"layer":"zones","count":zone_count,"disposition":"event_overlays_not_geographic_regions"},
        {"layer":"goods","count":generated["goods"],"disposition":"economic_semantics_pending_modernization"},
        {"layer":"markets","count":generated["markets"],"disposition":"network_evidence_not_current_prices"},
        {"layer":"deals","count":generated["deals"],"disposition":"flow_evidence_not_current_transactions"},
        {"layer":"renderer","count":1,"disposition":"author_evidence_only_not_player_surface"}
    ])
}

pub(super) fn allocate_exact(weights: &[f64], total: u64) -> Vec<u64> {
    let sum = weights.iter().sum::<f64>();
    if sum <= 0.0 {
        return vec![0; weights.len()];
    }
    let exact = weights
        .iter()
        .map(|weight| weight / sum * total as f64)
        .collect::<Vec<_>>();
    let mut allocated = exact
        .iter()
        .map(|value| value.floor() as u64)
        .collect::<Vec<_>>();
    let assigned = allocated.iter().sum::<u64>();
    let mut order = exact
        .iter()
        .enumerate()
        .map(|(index, value)| (index, value.fract()))
        .collect::<Vec<_>>();
    order.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    for (index, _) in order.into_iter().take((total - assigned) as usize) {
        allocated[index] += 1;
    }
    allocated
}

pub(super) fn nearest_settlement<'a>(
    point: &Point,
    settlements: &'a [SettlementRecord],
) -> &'a SettlementRecord {
    settlements
        .iter()
        .min_by(|left, right| {
            point_distance(point, &left.coordinates)
                .total_cmp(&point_distance(point, &right.coordinates))
                .then_with(|| left.source_burg_id.cmp(&right.source_burg_id))
        })
        .expect("settlements are validated non-empty")
}

pub(super) fn point_from_xy(value: &Value, width: f64, height: f64) -> Result<Point> {
    Ok(Point {
        x: number(value, "x")? / width,
        y: number(value, "y")? / height,
    })
}

fn point_from_cell(
    cells: &[Value],
    cell_id: Option<u64>,
    width: f64,
    height: f64,
) -> Result<Option<Point>> {
    let Some(id) = cell_id else {
        return Ok(None);
    };
    let cell = cells.get(id as usize).context("center cell 不存在")?;
    let p = cell["p"].as_array().context("center cell.p 不是陣列")?;
    Ok(Some(Point {
        x: p[0].as_f64().context("center x 缺失")? / width,
        y: p[1].as_f64().context("center y 缺失")? / height,
    }))
}

pub(super) fn positive_u32(value: &Value) -> Option<u32> {
    value.as_u64().filter(|id| *id > 0).map(|id| id as u32)
}

pub(super) fn count_strings<'a>(values: impl Iterator<Item = &'a str>) -> Value {
    let mut counts = BTreeMap::<String, usize>::new();
    for value in values {
        *counts.entry(value.to_string()).or_default() += 1;
    }
    json!(counts)
}

pub(super) fn count_values(values: &[Value], key: &str) -> Value {
    count_strings(values.iter().filter_map(|value| value[key].as_str()))
}

pub(super) fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value]> {
    value[key]
        .as_array()
        .map(Vec::as_slice)
        .with_context(|| format!("{key} 不是陣列"))
}

pub(super) fn number(value: &Value, key: &str) -> Result<f64> {
    value[key]
        .as_f64()
        .with_context(|| format!("{key} 不是數字"))
}

pub(super) fn text_or(value: &Value, key: &str, fallback: &str) -> String {
    value[key].as_str().unwrap_or(fallback).to_string()
}

pub(super) fn squared_distance(left: &NormalizedPoint, right: &NormalizedPoint) -> f64 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    dx * dx + dy * dy
}

fn point_distance(left: &Point, right: &Point) -> f64 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    dx * dx + dy * dy
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

pub(super) fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

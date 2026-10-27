#![forbid(unsafe_code)]
//! 從既有 Azgaar 完整作者包編譯白塔半島正式執行種子。
//!
//! 本工具不生成新世界，也不改寫作者包。它把全部來源層逐項記帳，將已驗證的
//! 305 個物理區與 1,644 條連線升成執行資料，並讓原生道路、聚落與市場實際
//! 影響通行成本和場所能力。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use singularity_world::world::physical_geography::{
    NormalizedPoint, PhysicalArea, PhysicalGeography, validate_physical_geography,
};
use singularity_world::world::place::{WHITE_TOWER_WORLD_ID, WorldPlace, WorldRoute};
use singularity_world::world::runtime_world::{
    RuntimeAreaProfile, RuntimeCoverage, RuntimeResourceSource, RuntimeSource,
    SourceLayerDisposition, WHITE_TOWER_RUNTIME_ID, WHITE_TOWER_RUNTIME_SOURCE_KIND,
    WhiteTowerRuntimeWorld, validate_white_tower_runtime_world,
};

const PACKAGE_DIR: &str = "data/world/white_tower_azgaar_v1";
const MANIFEST_PATH: &str = "data/world/white_tower_azgaar_v1/manifest.json";
const FULL_JSON_PATH: &str = "data/world/white_tower_azgaar_v1/white_tower_peninsula_v1_full.json";
const GEOGRAPHY_PATH: &str = "data/world/white_tower_physical_geography_v1.json";
const OUTPUT_PATH: &str = "data/world/white_tower_runtime_v1.json";

#[derive(Debug, Default)]
struct AreaAccumulator {
    burg_count: usize,
    population: f64,
    capital_count: usize,
    markets: BTreeSet<u32>,
    states: BTreeSet<u32>,
    cultures: BTreeSet<u32>,
    religions: BTreeSet<u32>,
    goods: BTreeSet<u32>,
    native_route_segment_count: usize,
}

#[derive(Debug, Clone)]
struct PlacePresentation {
    title: String,
    locator: String,
    full_name: String,
}

struct AreaLookup<'a> {
    areas: &'a [PhysicalArea],
}

impl AreaLookup<'_> {
    fn associate(&self, point: &NormalizedPoint, feature_id: Option<u32>) -> (&PhysicalArea, bool) {
        let same_feature = self
            .areas
            .iter()
            .filter(|area| feature_id == Some(area.feature_id))
            .collect::<Vec<_>>();
        let used_fallback = same_feature.is_empty();
        let candidates = if used_fallback {
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
            .expect("validated geography has areas");
        (area, used_fallback)
    }
}

fn main() -> Result<()> {
    let output = parse_output_path()?;
    let geography_bytes = fs::read(GEOGRAPHY_PATH).context("讀取白塔物理地理失敗")?;
    let geography: PhysicalGeography = serde_json::from_slice(&geography_bytes)?;
    validate_physical_geography(&geography)?;

    let manifest_bytes = fs::read(MANIFEST_PATH).context("讀取 Azgaar manifest 失敗")?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let full_bytes = fs::read(FULL_JSON_PATH).context("讀取 Azgaar Full JSON 失敗")?;
    validate_source_receipts(&manifest, &full_bytes, &geography_bytes)?;
    let full: Value = serde_json::from_slice(&full_bytes).context("解析 Azgaar Full JSON 失敗")?;
    validate_source_counts(&manifest, &full, &geography)?;

    let (world, statistics) = compile_world(&manifest, &full, &geography, &manifest_bytes)?;
    validate_white_tower_runtime_world(&world)?;
    let encoded = serde_json::to_string_pretty(&world)? + "\n";
    fs::write(&output, encoded).with_context(|| format!("寫入 {} 失敗", output.display()))?;

    println!("white_tower_runtime_compile: ok");
    println!(
        "places={} routes={} native_roads={} burgs={} resources={}",
        world.places.len(),
        world.routes.len(),
        statistics.native_road_edges,
        world.coverage.burgs_accounted,
        world.resources.len()
    );
    println!("output={}", output.display());
    Ok(())
}

#[derive(Debug)]
struct CompileStatistics {
    native_road_edges: usize,
}

fn compile_world(
    manifest: &Value,
    full: &Value,
    geography: &PhysicalGeography,
    manifest_bytes: &[u8],
) -> Result<(WhiteTowerRuntimeWorld, CompileStatistics)> {
    let lookup = AreaLookup {
        areas: &geography.areas,
    };
    let mut accumulators = geography
        .areas
        .iter()
        .map(|area| (area.id.clone(), AreaAccumulator::default()))
        .collect::<BTreeMap<_, _>>();

    let (burgs_accounted, burg_fallbacks) =
        associate_burgs(full, geography, &lookup, &mut accumulators)?;
    let route_result = associate_native_routes(full, geography, &lookup, &mut accumulators)?;
    let place_presentations = compile_place_presentations(geography, &accumulators);
    let places = compile_places(geography, &accumulators, &place_presentations);
    let routes = compile_routes(geography, &route_result.native_pairs);
    let profiles = compile_profiles(geography, &accumulators);
    let resources = compile_resources(geography, &place_presentations);
    let spawn_place_id = geography
        .anchor_mappings
        .iter()
        .find(|mapping| mapping.topology_node_ref == "wt-c-wenyao")
        .and_then(|mapping| mapping.area_ref.clone())
        .context("文曜沒有可用的出生區")?;

    let generated = &manifest["generated"];
    let source_layers = source_layer_ledger(generated);
    let coverage = RuntimeCoverage {
        physical_areas_activated: places.len(),
        physical_routes_activated: routes.len(),
        native_routes_accounted: route_result.accounted,
        native_routes_crossing_areas: route_result.crossing,
        native_routes_within_one_area: route_result.within_one_area,
        burgs_accounted,
        burgs_associated_to_areas: burgs_accounted,
        burgs_using_nearest_feature_fallback: burg_fallbacks,
        markets_accounted: usize_at(generated, "markets")?,
        goods_accounted: usize_at(generated, "goods")?,
        deals_accounted: usize_at(generated, "deals")?,
    };
    let full_hash = file_hash(FULL_JSON_PATH)?;
    let geography_hash = file_hash(GEOGRAPHY_PATH)?;
    let world = WhiteTowerRuntimeWorld {
        schema_version: 1,
        id: WHITE_TOWER_RUNTIME_ID.to_string(),
        name: "白塔半島正式執行世界 v1".to_string(),
        status: "accepted_runtime".to_string(),
        canon: false,
        runtime_authority: true,
        player_facing_map: false,
        source: RuntimeSource {
            package_id: manifest["id"].as_str().unwrap_or_default().to_string(),
            package_manifest_path: MANIFEST_PATH.to_string(),
            package_manifest_sha256: sha256(manifest_bytes),
            full_json_path: FULL_JSON_PATH.to_string(),
            full_json_sha256: full_hash,
            physical_geography_path: GEOGRAPHY_PATH.to_string(),
            physical_geography_sha256: geography_hash,
            azgaar_commit: manifest["source"]["commit"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            seed: manifest["source"]["seed"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            compiler_version: 4,
        },
        spawn_place_id,
        places,
        routes,
        area_profiles: profiles,
        resources,
        source_layers,
        coverage,
    };
    let native_road_edges = world
        .routes
        .iter()
        .filter(|route| route.mode == "road")
        .count();
    Ok((world, CompileStatistics { native_road_edges }))
}

fn associate_burgs(
    full: &Value,
    geography: &PhysicalGeography,
    lookup: &AreaLookup<'_>,
    accumulators: &mut BTreeMap<String, AreaAccumulator>,
) -> Result<(usize, usize)> {
    let burgs = array_at(&full["pack"], "burgs")?;
    let mut accounted = 0;
    let mut fallbacks = 0;
    for burg in burgs.iter().filter(|value| value.is_object()) {
        accounted += 1;
        let point = normalized_point(burg, geography, "x", "y")?;
        let feature = burg["feature"].as_u64().map(|value| value as u32);
        let (area, fallback) = lookup.associate(&point, feature);
        fallbacks += usize::from(fallback);
        let accumulator = accumulators.get_mut(&area.id).expect("area accumulator");
        accumulator.burg_count += 1;
        accumulator.population += burg["population"].as_f64().unwrap_or_default();
        accumulator.capital_count += usize::from(burg["capital"].as_i64().unwrap_or_default() > 0);
        insert_positive_u32(&mut accumulator.markets, &burg["market"]);
        insert_positive_u32(&mut accumulator.states, &burg["state"]);
        insert_positive_u32(&mut accumulator.cultures, &burg["culture"]);
        let cell_id = burg["cell"].as_u64().map(|value| value as usize);
        if let Some(cell) = cell_id.and_then(|id| full["pack"]["cells"].get(id)) {
            insert_positive_u32(&mut accumulator.religions, &cell["religion"]);
        }
        if let Some(production) = burg["production"].as_array() {
            for record in production {
                insert_positive_u32(&mut accumulator.goods, &record["goodId"]);
            }
        }
    }
    Ok((accounted, fallbacks))
}

struct NativeRouteResult {
    native_pairs: BTreeSet<(String, String)>,
    accounted: usize,
    crossing: usize,
    within_one_area: usize,
}

fn associate_native_routes(
    full: &Value,
    geography: &PhysicalGeography,
    lookup: &AreaLookup<'_>,
    accumulators: &mut BTreeMap<String, AreaAccumulator>,
) -> Result<NativeRouteResult> {
    let routes = array_at(&full["pack"], "routes")?;
    let mut native_pairs = BTreeSet::new();
    let mut accounted = 0;
    let mut crossing = 0;
    let mut within_one_area = 0;
    for route in routes.iter().filter(|value| value.is_object()) {
        accounted += 1;
        let feature = route["feature"].as_u64().map(|value| value as u32);
        let points = route["points"]
            .as_array()
            .context("native route 缺 points")?;
        let mut area_path = Vec::new();
        for point in points {
            let values = point.as_array().context("native route point 無效")?;
            ensure!(values.len() >= 2, "native route point 欄位不足");
            let normalized = NormalizedPoint {
                x: values[0].as_f64().context("route x 無效")?
                    / f64::from(geography.coordinate_space.width),
                y: values[1].as_f64().context("route y 無效")?
                    / f64::from(geography.coordinate_space.height),
            };
            let (area, _) = lookup.associate(&normalized, feature);
            if area_path.last().is_none_or(|last| last != &area.id) {
                area_path.push(area.id.clone());
            }
        }
        if area_path.len() < 2 {
            within_one_area += 1;
            continue;
        }
        crossing += 1;
        for pair in area_path.windows(2) {
            let key = ordered_pair(&pair[0], &pair[1]);
            native_pairs.insert(key);
            if let Some(accumulator) = accumulators.get_mut(&pair[0]) {
                accumulator.native_route_segment_count += 1;
            }
            if let Some(accumulator) = accumulators.get_mut(&pair[1]) {
                accumulator.native_route_segment_count += 1;
            }
        }
    }
    Ok(NativeRouteResult {
        native_pairs,
        accounted,
        crossing,
        within_one_area,
    })
}

fn compile_places(
    geography: &PhysicalGeography,
    accumulators: &BTreeMap<String, AreaAccumulator>,
    place_presentations: &BTreeMap<String, PlacePresentation>,
) -> Vec<WorldPlace> {
    geography
        .areas
        .iter()
        .map(|area| {
            let accumulator = accumulators.get(&area.id).expect("area accumulator");
            let presentation = place_presentations
                .get(&area.id)
                .expect("compiled place presentation");
            let mut tags = vec![
                "white_tower".to_string(),
                "work".to_string(),
                "rest".to_string(),
                format!("player_title:{}", presentation.title),
                format!("player_locator:{}", presentation.locator),
                format!("terrain:{}", area.terrain_class),
                format!("elevation:{}", area.elevation_band),
                format!("feature:{}", area.feature_id),
                format!("resource:{}", resource_for_area(area).0),
            ];
            tags.extend(
                area.biome_names
                    .iter()
                    .map(|biome| format!("biome:{biome}")),
            );
            if area.river_cell_count > 0 {
                tags.push("river".to_string());
            }
            if accumulator.burg_count > 0 {
                tags.extend([
                    "settlement".to_string(),
                    "social".to_string(),
                    "lodging".to_string(),
                ]);
            }
            if !accumulator.markets.is_empty() {
                tags.extend(["market".to_string(), "trade".to_string()]);
            }
            tags.sort();
            tags.dedup();
            WorldPlace {
                id: area.id.clone(),
                world_id: WHITE_TOWER_WORLD_ID.to_string(),
                topology_node_ref: Some(area.topology_node_ref.clone()),
                parent_place_id: Some(area.parent_place_id.clone()),
                name: presentation.full_name.clone(),
                kind: "terrain_area".to_string(),
                scale: "terrain_area".to_string(),
                canon: false,
                status: "active".to_string(),
                traversable: true,
                source_kind: WHITE_TOWER_RUNTIME_SOURCE_KIND.to_string(),
                zone: area.topology_node_ref.clone(),
                tags,
                description: runtime_description(area, accumulator),
                legacy_grid_x: None,
                legacy_grid_y: None,
            }
        })
        .collect()
}

fn compile_routes(
    geography: &PhysicalGeography,
    native_pairs: &BTreeSet<(String, String)>,
) -> Vec<WorldRoute> {
    geography
        .route_candidates
        .iter()
        .map(|route| {
            let native_road =
                native_pairs.contains(&ordered_pair(&route.from_place_id, &route.to_place_id));
            let mode = if route.mode == "ferry" {
                "ferry"
            } else if native_road {
                "road"
            } else {
                route.mode.as_str()
            };
            let travel_cost = if native_road {
                (route.relative_cost * 2 / 3).max(1)
            } else {
                route.relative_cost
            };
            WorldRoute {
                id: route.id.clone(),
                from_place_id: route.from_place_id.clone(),
                direction: route.direction.clone(),
                to_place_id: route.to_place_id.clone(),
                mode: mode.to_string(),
                travel_cost,
                canon: false,
                status: "active".to_string(),
                source_kind: WHITE_TOWER_RUNTIME_SOURCE_KIND.to_string(),
            }
        })
        .collect()
}

fn compile_profiles(
    geography: &PhysicalGeography,
    accumulators: &BTreeMap<String, AreaAccumulator>,
) -> Vec<RuntimeAreaProfile> {
    geography
        .areas
        .iter()
        .map(|area| {
            let accumulator = accumulators.get(&area.id).expect("area accumulator");
            RuntimeAreaProfile {
                place_id: area.id.clone(),
                centroid: area.centroid.clone(),
                terrain_class: area.terrain_class.clone(),
                elevation_band: area.elevation_band.clone(),
                biome_names: area.biome_names.clone(),
                source_cell_count: area.source_cell_count,
                river_cell_count: area.river_cell_count,
                candidate_burg_count: accumulator.burg_count,
                candidate_population: round(accumulator.population, 3),
                candidate_capital_count: accumulator.capital_count,
                candidate_market_refs: accumulator.markets.iter().copied().collect(),
                candidate_state_refs: accumulator.states.iter().copied().collect(),
                candidate_culture_refs: accumulator.cultures.iter().copied().collect(),
                candidate_religion_refs: accumulator.religions.iter().copied().collect(),
                candidate_good_refs: accumulator.goods.iter().copied().collect(),
                native_route_segment_count: accumulator.native_route_segment_count,
            }
        })
        .collect()
}

fn compile_resources(
    geography: &PhysicalGeography,
    place_presentations: &BTreeMap<String, PlacePresentation>,
) -> Vec<RuntimeResourceSource> {
    geography
        .areas
        .iter()
        .map(|area| {
            let (item_id, name, origin) = resource_for_area(area);
            let capacity = i32::try_from(area.source_cell_count / 2)
                .unwrap_or(i32::MAX)
                .clamp(24, 240);
            RuntimeResourceSource {
                place_id: area.id.clone(),
                item_id: item_id.to_string(),
                name: name.to_string(),
                origin_label: format!(
                    "{}的{}",
                    place_presentations
                        .get(&area.id)
                        .expect("compiled place presentation")
                        .full_name,
                    origin
                ),
                capacity,
                regeneration_per_day: (capacity / 12).max(1),
            }
        })
        .collect()
}

/// 把物理聚合區的工程定位拆成「正式大地名」與「局部定位」兩層。
///
/// `title` 只沿用已接受的白塔地名；方位與地表特徵只進 `locator`，不得再被
/// 玩家介面當成專名標題。`full_name` 供需要單一字串的舊介面與記錄使用。
/// 低層區域仍是 non-canon runtime，不為資料庫唯一性硬造三百個假地名。
fn compile_place_presentations(
    geography: &PhysicalGeography,
    accumulators: &BTreeMap<String, AreaAccumulator>,
) -> BTreeMap<String, PlacePresentation> {
    let anchor_labels = geography
        .anchor_mappings
        .iter()
        .map(|anchor| (anchor.topology_node_ref.as_str(), anchor.label.as_str()))
        .collect::<BTreeMap<_, _>>();
    let exact_anchor_labels = geography
        .anchor_mappings
        .iter()
        .filter_map(|anchor| {
            anchor
                .area_ref
                .as_deref()
                .map(|area_id| (area_id, anchor.label.as_str()))
        })
        .collect::<BTreeMap<_, _>>();

    let mut zone_sums = BTreeMap::<&str, (f64, f64, usize)>::new();
    for area in &geography.areas {
        let entry = zone_sums
            .entry(area.topology_node_ref.as_str())
            .or_default();
        entry.0 += area.centroid.x;
        entry.1 += area.centroid.y;
        entry.2 += 1;
    }

    geography
        .areas
        .iter()
        .map(|area| {
            let title = anchor_labels
                .get(area.topology_node_ref.as_str())
                .copied()
                .unwrap_or("白塔半島")
                .to_string();
            let accumulator = accumulators.get(&area.id).expect("area accumulator");
            let terrain = terrain_label(area, accumulator);
            let (locator, full_name) = if exact_anchor_labels.contains_key(area.id.as_str()) {
                (terrain.to_string(), title.clone())
            } else {
                let (sum_x, sum_y, count) = zone_sums
                    .get(area.topology_node_ref.as_str())
                    .copied()
                    .expect("area zone center");
                let center = NormalizedPoint {
                    x: sum_x / count as f64,
                    y: sum_y / count as f64,
                };
                let locator = format!(
                    "{} · {terrain}",
                    relative_direction(&area.centroid, &center)
                );
                let full_name = format!("{title}（{locator}）");
                (locator, full_name)
            };
            (
                area.id.clone(),
                PlacePresentation {
                    title,
                    locator,
                    full_name,
                },
            )
        })
        .collect()
}

fn relative_direction(point: &NormalizedPoint, center: &NormalizedPoint) -> &'static str {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let abs_x = dx.abs();
    let abs_y = dy.abs();
    if dx.hypot(dy) < 0.018 {
        "腹地"
    } else if abs_y > abs_x * 1.8 {
        if dy < 0.0 { "北側" } else { "南側" }
    } else if abs_x > abs_y * 1.8 {
        if dx < 0.0 { "西側" } else { "東側" }
    } else {
        match (dy < 0.0, dx < 0.0) {
            (true, true) => "西北側",
            (true, false) => "東北側",
            (false, true) => "西南側",
            (false, false) => "東南側",
        }
    }
}

fn terrain_label(area: &PhysicalArea, accumulator: &AreaAccumulator) -> &'static str {
    let river_ratio = area.river_cell_count as f64 / area.source_cell_count.max(1) as f64;
    let has_market = !accumulator.markets.is_empty();
    let settled = accumulator.burg_count > 0;
    let wet = area.biome_names.iter().any(|biome| biome == "Wetland");
    match area.terrain_class.as_str() {
        "forest" if wet && river_ratio > 0.08 => "水澤林",
        "forest" if river_ratio > 0.18 => "溪谷林",
        "forest" if has_market => "林緣聚地",
        "forest" if settled => "林間聚地",
        "forest" if accumulator.native_route_segment_count >= 5 => "林道",
        "forest" if area.elevation_band == "lowland" => "低地林",
        "forest" if area.elevation_band == "highland" => "山林",
        "forest" => "丘陵林",
        "grassland" if river_ratio > 0.1 => "河畔草地",
        "grassland" if settled => "草原聚地",
        "grassland" => "草原",
        "wetland" if settled => "澤畔聚地",
        "wetland" => "濕地",
        "snow_and_ice" if area.elevation_band == "highland" => "雪嶺",
        "snow_and_ice" => "雪原",
        "tundra" if river_ratio > 0.1 => "寒地水澤",
        "tundra" => "凍原",
        "open_bare_ground" if river_ratio > 0.15 => "河灘",
        "open_bare_ground" if has_market => "市郊",
        "open_bare_ground" if settled => "聚地",
        "open_bare_ground" if area.elevation_band == "lowland" => "平原",
        "open_bare_ground" if area.elevation_band == "highland" => "高原",
        "open_bare_ground" => "丘原",
        _ => "野地",
    }
}

fn source_layer_ledger(generated: &Value) -> Vec<SourceLayerDisposition> {
    let rows = [
        (
            "grid_cells",
            100_000,
            "runtime_evidence",
            "保留原始計算格與鄰接證據，不把一格當一場所",
        ),
        (
            "packed_cells",
            57_623,
            "runtime_evidence",
            "保留高度、生態、水系與聚合來源",
        ),
        (
            "physical_areas",
            305,
            "runtime",
            "全部成為可通行的白塔地形區",
        ),
        (
            "physical_routes",
            1_644,
            "runtime",
            "全部成為人物共用的有向通行關係",
        ),
        (
            "rivers",
            number_or(generated, "rivers", 1_129),
            "runtime_evidence",
            "水系影響地形描述與物質來源，精確河名仍待校正",
        ),
        (
            "burgs",
            number_or(generated, "burgCandidates", 1_666),
            "runtime_evidence",
            "全數關聯至地形區並影響聚居、休息、社交與工作能力",
        ),
        (
            "native_routes",
            number_or(generated, "routeCandidates", 1_244),
            "runtime_evidence",
            "全數交代；跨區道路降低對應通行成本",
        ),
        (
            "states",
            number_or(generated, "stateCandidates", 23),
            "retained_candidate",
            "政治邊界與名稱保留待校正，不冒充白塔正典",
        ),
        (
            "provinces",
            number_or(generated, "provinceCandidates", 487),
            "retained_candidate",
            "行政分區保留待校正",
        ),
        (
            "cultures",
            number_or(generated, "cultureCandidates", 9),
            "retained_candidate",
            "文化候選保留並可由區域來源回查",
        ),
        (
            "religions",
            number_or(generated, "religionCandidates", 14),
            "retained_candidate",
            "宗教候選保留並可由區域來源回查",
        ),
        (
            "markers",
            number_or(generated, "markerCandidates", 456),
            "retained_candidate",
            "地標候選完整保留待逐項命名",
        ),
        (
            "zones",
            number_or(generated, "zoneCandidates", 15),
            "retained_candidate",
            "區域候選完整保留待逐項校正",
        ),
        (
            "goods",
            number_or(generated, "goods", 71),
            "runtime_evidence",
            "商品層完整保留，作區域產能與後續經濟詞彙來源",
        ),
        (
            "markets",
            number_or(generated, "markets", 54),
            "runtime_evidence",
            "市場層影響場所的交易與社交能力",
        ),
        (
            "deals",
            number_or(generated, "deals", 25_808),
            "runtime_evidence",
            "交易層完整保留作供需與生產鏈證據，不直接灌成玩家報表",
        ),
        (
            "renderer",
            1,
            "rejected_surface",
            "僅拒絕把 Azgaar 畫面當玩家 UI",
        ),
    ];
    rows.into_iter()
        .map(
            |(layer, source_count, disposition, runtime_use)| SourceLayerDisposition {
                layer: layer.to_string(),
                source_count,
                disposition: disposition.to_string(),
                runtime_use: runtime_use.to_string(),
            },
        )
        .collect()
}

fn runtime_description(area: &PhysicalArea, accumulator: &AreaAccumulator) -> String {
    let terrain = match area.terrain_class.as_str() {
        "forest" => "林冠遮住部分天光，樹根與落葉讓腳步慢下來。",
        "grassland" => "草地隨地勢起伏，視線能越過較遠的坡面。",
        "wetland" => "水道、軟泥與草叢交錯，能走的地面並不連續。",
        "snow_and_ice" => "積雪覆住高地，冰面和碎石使通行費力。",
        "tundra" => "低矮植被貼著冷地生長，裸露處可見凍硬土石。",
        "open_bare_ground" => "裸露土石沿坡面展開，遮蔽物很少。",
        _ => "不同地表在此交錯，路面隨地勢而變。",
    };
    let river = if area.river_cell_count > 0 {
        "水流穿過這一帶，低處留有沖刷和濕痕。"
    } else {
        ""
    };
    let settlement = if accumulator.burg_count > 0 {
        "沿途可見住居、腳印與反覆使用的通行痕跡。"
    } else {
        ""
    };
    let market = if accumulator.markets.is_empty() {
        ""
    } else {
        "人與貨物在較平穩的地面聚散。"
    };
    format!("{terrain}{river}{settlement}{market}")
}

fn resource_for_area(area: &PhysicalArea) -> (&'static str, &'static str, &'static str) {
    match area.terrain_class.as_str() {
        "forest" => ("mat_kindling_bundle", "柴枝束", "林下散落枝材"),
        "grassland" => ("mat_hay_bundle", "乾草束", "可收割草地"),
        "wetland" => ("mat_reed_bundle", "蘆草束", "濕地蘆草帶"),
        "snow_and_ice" | "open_bare_ground" => ("mat_rough_ore", "粗礦", "裸露岩層"),
        "tundra" => ("mat_wet_grass_bundle", "濕草束", "低矮耐寒草叢"),
        _ => ("wild_herb", "野草", "混合地表草叢"),
    }
}

fn validate_source_receipts(
    manifest: &Value,
    full_bytes: &[u8],
    geography_bytes: &[u8],
) -> Result<()> {
    let receipt = manifest["files"]
        .as_array()
        .context("manifest.files 缺失")?
        .iter()
        .find(|record| record["path"] == "white_tower_peninsula_v1_full.json")
        .context("Full JSON 收據缺失")?;
    ensure!(
        receipt["bytes"].as_u64() == Some(full_bytes.len() as u64),
        "Full JSON 大小不符"
    );
    ensure!(
        receipt["sha256"] == sha256(full_bytes),
        "Full JSON 雜湊不符"
    );
    ensure!(
        manifest["physicalGeography"]["bytes"].as_u64() == Some(geography_bytes.len() as u64),
        "物理地理大小不符"
    );
    ensure!(
        manifest["physicalGeography"]["sha256"] == sha256(geography_bytes),
        "物理地理雜湊不符"
    );
    Ok(())
}

fn validate_source_counts(
    manifest: &Value,
    full: &Value,
    geography: &PhysicalGeography,
) -> Result<()> {
    ensure!(
        full["info"]["mapName"] == "白塔半島",
        "Full JSON 地圖身分不符"
    );
    let pack = &full["pack"];
    let generated = &manifest["generated"];
    let burg_count = array_at(pack, "burgs")?
        .iter()
        .filter(|value| value.is_object())
        .count();
    ensure!(
        generated["burgCandidates"].as_u64() == Some(burg_count as u64),
        "burgs 計數不符"
    );
    ensure!(
        array_at(pack, "routes")?.len() == usize_at(generated, "routeCandidates")?,
        "routes 計數不符"
    );
    for (layer, manifest_key) in [
        ("states", "stateCandidates"),
        ("provinces", "provinceCandidates"),
        ("cultures", "cultureCandidates"),
        ("religions", "religionCandidates"),
    ] {
        let actual = array_at(pack, layer)?
            .iter()
            .filter(|value| value["i"].as_u64().is_some_and(|id| id > 0))
            .count();
        ensure!(
            generated[manifest_key].as_u64() == Some(actual as u64),
            "{layer} 計數不符"
        );
    }
    for (layer, manifest_key) in [("markers", "markerCandidates"), ("zones", "zoneCandidates")] {
        let actual = array_at(pack, layer)?
            .iter()
            .filter(|value| value["i"].as_u64().is_some() && value["removed"] != true)
            .count();
        ensure!(
            generated[manifest_key].as_u64() == Some(actual as u64),
            "{layer} 計數不符"
        );
    }
    for layer in ["goods", "markets", "deals"] {
        ensure!(
            array_at(pack, layer)?.len() == usize_at(generated, layer)?,
            "{layer} 計數不符"
        );
    }
    ensure!(geography.areas.len() == 305, "物理區數漂移");
    ensure!(geography.route_candidates.len() == 1_644, "物理路線數漂移");
    ensure!(Path::new(PACKAGE_DIR).is_dir(), "Azgaar 作者包不存在");
    Ok(())
}

fn parse_output_path() -> Result<PathBuf> {
    let mut arguments = std::env::args_os().skip(1);
    let output = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(OUTPUT_PATH));
    ensure!(arguments.next().is_none(), "參數過多");
    Ok(output)
}

fn normalized_point(
    value: &Value,
    geography: &PhysicalGeography,
    x_key: &str,
    y_key: &str,
) -> Result<NormalizedPoint> {
    Ok(NormalizedPoint {
        x: value[x_key].as_f64().context("座標 x 無效")?
            / f64::from(geography.coordinate_space.width),
        y: value[y_key].as_f64().context("座標 y 無效")?
            / f64::from(geography.coordinate_space.height),
    })
}

fn array_at<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value[key]
        .as_array()
        .with_context(|| format!("{key} 不是陣列"))
}

fn usize_at(value: &Value, key: &str) -> Result<usize> {
    usize::try_from(
        value[key]
            .as_u64()
            .with_context(|| format!("{key} 計數缺失"))?,
    )
    .with_context(|| format!("{key} 計數超出 usize"))
}

fn number_or(value: &Value, key: &str, fallback: usize) -> usize {
    value[key]
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .unwrap_or(fallback)
}

fn insert_positive_u32(target: &mut BTreeSet<u32>, value: &Value) {
    if let Some(number) = value.as_u64().filter(|number| *number > 0)
        && let Ok(number) = u32::try_from(number)
    {
        target.insert(number);
    }
}

fn ordered_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

fn squared_distance(left: &NormalizedPoint, right: &NormalizedPoint) -> f64 {
    (left.x - right.x).powi(2) + (left.y - right.y).powi(2)
}

fn round(value: f64, digits: i32) -> f64 {
    let factor = 10_f64.powi(digits);
    (value * factor).round() / factor
}

fn file_hash(path: &str) -> Result<String> {
    Ok(sha256(&fs::read(path)?))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

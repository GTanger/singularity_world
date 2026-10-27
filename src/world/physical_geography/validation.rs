//! 白塔物理地理的整體契約、來源邊界與生成證據驗證。

mod anchors;
mod hydrology;
mod land;
mod network;
mod profile;

use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use super::PhysicalGeography;
use crate::world::topology::builtin_white_tower_topology;

const SOURCE_COMMIT: &str = "73e43d2b44b9b85b42c5394a0476d90bb22d688e";
const SOURCE_SEED: &str = "white-tower-peninsula-v1";
const AGGREGATE_SIZE: f64 = 0.05;

/// 依序驗證作者資料所有層級並回傳結果；輸入只讀，沒有外部副作用。
pub(super) fn validate(geography: &PhysicalGeography) -> Result<()> {
    // 先守住非 runtime 邊界，再檢查引用與圖連通，避免半成品被轉換使用。
    validate_header(geography)?;
    let topology = builtin_white_tower_topology()?;
    let topology_ids = topology
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    let biome_names = land::validate_biomes(&geography.biomes)?;
    let area_ids = land::validate_areas(geography, &topology_ids, &biome_names)?;
    profile::validate_terrain_profile(geography)?;
    land::validate_land_features(geography)?;
    land::validate_area_references(geography)?;
    anchors::validate_anchors(geography, &topology_ids, &area_ids)?;
    network::validate_routes(geography, &topology_ids, &area_ids)?;
    network::validate_rivers(geography)?;
    hydrology::validate(geography, &topology_ids)?;
    validate_source_boundary(geography)?;
    validate_evidence(geography)?;
    network::validate_connectivity(geography, &area_ids)
}

/// 驗證資料身分、來源、座標與統計契約；輸入只讀，沒有外部副作用。
fn validate_header(geography: &PhysicalGeography) -> Result<()> {
    // 各類契約拆開，讓錯誤能準確指出漂移位置。
    validate_identity(geography)?;
    validate_source(geography)?;
    validate_coordinate_space(geography)?;
    validate_metrics(geography)
}

/// 驗證非正典作者資料身分與禁止升格旗標；輸入只讀，沒有外部副作用。
fn validate_identity(geography: &PhysicalGeography) -> Result<()> {
    // 這些旗標是作者橋接和正式世界資料之間的硬隔離。
    ensure!(
        geography.schema_version == 1,
        "物理地理 schemaVersion 必須是 1"
    );
    ensure!(
        geography.id == "white_tower_physical_geography_v1",
        "物理地理 id 不符"
    );
    ensure!(
        !geography.name.trim().is_empty(),
        "geography.name 不可為空白"
    );
    ensure!(
        geography.status == "working_proposal",
        "物理地理不可冒充 accepted"
    );
    ensure!(!geography.canon, "物理地理候選不可冒充正典");
    ensure!(
        !geography.runtime_authority,
        "物理地理候選不可成為 runtime 權威"
    );
    ensure!(
        !geography.player_facing_map,
        "Azgaar renderer 不可成為玩家地圖"
    );
    Ok(())
}

/// 驗證 Azgaar 固定版本、seed、profile 與正典權威指向；輸入只讀，沒有副作用。
fn validate_source(geography: &PhysicalGeography) -> Result<()> {
    // 固定來源使同一約束可重現，也避免上游更新無聲改變資料。
    let source = &geography.source;
    ensure!(
        source.project == "Azgaar/Fantasy-Map-Generator",
        "來源專案不符"
    );
    ensure!(source.commit == SOURCE_COMMIT, "Azgaar 來源 commit 未鎖定");
    ensure!(source.seed == SOURCE_SEED, "生成 seed 不符");
    ensure!(
        source.heightmap_profile == "whiteTowerConstrained",
        "高度圖 profile 不符"
    );
    ensure!(
        source.canon_authority == "data/world/white_tower_topology_v1.json",
        "正典權威不得指向生成資料"
    );
    Ok(())
}

/// 驗證作者正規化座標沒有偷渡成正式比例尺或 runtime 座標；輸入只讀，無副作用。
fn validate_coordinate_space(geography: &PhysicalGeography) -> Result<()> {
    // 畫布與聚合大小是重現參數，不是遊戲世界尺度。
    let space = &geography.coordinate_space;
    ensure!(
        space.kind == "normalized_authoring",
        "座標只能是作者用正規化空間"
    );
    ensure!(
        space.width == 1600 && space.height == 1000,
        "生成畫布尺寸漂移"
    );
    ensure!(
        (space.physical_aggregate_size - AGGREGATE_SIZE).abs() < f64::EPSILON,
        "物理聚合尺度漂移"
    );
    ensure!(!space.formal_scale_decided, "不可偷定正式比例尺");
    ensure!(!space.coordinate_system_decided, "不可偷定 runtime 座標制");
    Ok(())
}

/// 驗證輸出統計互相守恆並防止一 cell 一場所膨脹；輸入只讀，沒有副作用。
fn validate_metrics(geography: &PhysicalGeography) -> Result<()> {
    // 計數同時驗證 JSON 沒截斷，並守住物理 cell 與世界場所的尺度差。
    let metrics = &geography.metrics;
    ensure!(metrics.grid_cells == 100_000, "Azgaar grid cell 數漂移");
    ensure!(
        metrics.packed_cells > metrics.aggregated_land_cells,
        "packed cell 計數不合理"
    );
    ensure!(
        metrics.generated_land_cells
            == metrics.aggregated_land_cells + metrics.omitted_tiny_islet_cells,
        "陸地 cell 守恆失敗"
    );
    ensure!(
        metrics.physical_areas == geography.areas.len(),
        "physicalAreas 計數不符"
    );
    ensure!(
        metrics.route_candidates == geography.route_candidates.len(),
        "routeCandidates 計數不符"
    );
    ensure!(metrics.rivers == geography.rivers.len(), "rivers 計數不符");
    ensure!(metrics.lakes == 0, "黑沼林必須是濕地，不得生成成湖泊");
    ensure!(
        geography.areas.len() * 10 < metrics.aggregated_land_cells,
        "聚合失效：接近一 cell 一場所"
    );
    Ok(())
}

/// 驗證保留、候選與拒絕三類上游欄位沒有越界；輸入只讀，沒有副作用。
fn validate_source_boundary(geography: &PhysicalGeography) -> Result<()> {
    // 每一類都以必含集合檢查，新增欄位不會無聲取得正典身分。
    let dispositions = &geography.source_dispositions;
    ensure_contains_all(
        &dispositions.kept,
        &[
            "cell adjacency",
            "height",
            "biome",
            "river hierarchy",
            "land features",
            "deterministic provenance",
        ],
        "保留來源欄位",
    )?;
    ensure_contains_all(
        &dispositions.candidate_only,
        &[
            "aggregated physical areas",
            "direction hints",
            "relative travel costs",
            "generated names",
            "burgs",
            "routes",
            "states",
            "provinces",
            "cultures",
            "religions",
            "politics",
            "markers",
            "zones",
            "notes",
            "goods",
            "markets",
            "deals",
        ],
        "候選來源欄位",
    )?;
    ensure_contains_all(
        &dispositions.rejected,
        &["renderer as player UI"],
        "拒絕上游表面",
    )?;
    validate_invariants(geography)
}

/// 驗證一組字串包含所有必要值；輸入只讀，成功回空值，沒有副作用。
fn ensure_contains_all(values: &[String], required: &[&str], label: &str) -> Result<()> {
    // 逐項回報缺口，避免只有模糊的集合不相等錯誤。
    for expected in required {
        ensure!(
            values.iter().any(|value| value == expected),
            "缺少{label}：{expected}"
        );
    }
    Ok(())
}

/// 驗證白塔半島八個宏觀物理關係都成立；輸入只讀，沒有外部副作用。
fn validate_invariants(geography: &PhysicalGeography) -> Result<()> {
    // 這些布林值來自約束生成後的明示檢查，不可部分通過。
    let value = &geography.invariants;
    ensure!(
        value.one_mainland_and_windship_island,
        "半島與風舶嶼關係未成立"
    );
    ensure!(value.windship_channel_remains_water, "天然港灣道未保持水域");
    ensure!(
        value.white_tower_heads_green_ridge,
        "白塔與青脊山關係未成立"
    );
    ensure!(value.green_ridge_meets_east_sea, "青脊山與東海關係未成立");
    ensure!(
        value.three_source_valleys_meet_near_wenyao,
        "三源谷未在文曜附近會流"
    );
    ensure!(value.lower_river_reaches_black_marsh, "下游未抵達黑沼林");
    ensure!(value.black_marsh_is_wetland_not_lake, "黑沼林被錯作湖泊");
    ensure!(
        value.central_forest_is_not_black_marsh,
        "中央森林與黑沼林主次混淆"
    );
    Ok(())
}

/// 驗證生成頁面與瀏覽器 console 沒有錯誤；警告只保留作證據，沒有副作用。
fn validate_evidence(geography: &PhysicalGeography) -> Result<()> {
    // 執行時間警告不影響資料；真正的 console/page error 必須阻擋採用。
    let evidence = &geography.generation_evidence;
    ensure!(evidence.console_errors.is_empty(), "Azgaar console 有錯誤");
    ensure!(evidence.page_errors.is_empty(), "Azgaar page 有錯誤");
    ensure!(
        evidence
            .rejected_surface_errors
            .iter()
            .all(|error| error == "Name is too short! Random name will be selected"),
        "Azgaar 被拒絕表面含未知錯誤"
    );
    ensure!(
        evidence.rejected_surface_errors.is_empty()
            || geography
                .source_dispositions
                .candidate_only
                .iter()
                .any(|value| value == "generated names"),
        "生成名稱錯誤只能隨明示候選的名稱表面保存"
    );
    Ok(())
}

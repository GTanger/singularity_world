//! 生態、聚合陸地區塊與主要陸塊守恆驗證。

use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use crate::world::physical_geography::{
    Biome, NormalizedBounds, NormalizedPoint, PhysicalArea, PhysicalGeography,
};
use crate::world::place::WHITE_TOWER_WORLD_ID;

/// 驗證生態 id/name 唯一且非空，回傳可供區塊引用的名稱集合；沒有副作用。
pub(super) fn validate_biomes(biomes: &[Biome]) -> Result<BTreeSet<&str>> {
    // 名稱集合供後續 area biomeNames 做閉合引用檢查。
    ensure!(!biomes.is_empty(), "biomes 不可為空");
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for biome in biomes {
        ensure!(ids.insert(biome.id), "重複 biome id：{}", biome.id);
        ensure_nonblank("biome.name", &biome.name)?;
        ensure!(
            names.insert(biome.name.as_str()),
            "重複 biome name：{}",
            biome.name
        );
    }
    Ok(names)
}

/// 驗證全部物理區塊及 cell 守恆，回傳唯一 area id 集合；沒有外部副作用。
pub(super) fn validate_areas<'a>(
    geography: &'a PhysicalGeography,
    topology_ids: &BTreeSet<&str>,
    biome_names: &BTreeSet<&str>,
) -> Result<BTreeSet<&'a str>> {
    // 每個 area 先獨立驗證，最後再核對聚合 cell 總數。
    ensure!(!geography.areas.is_empty(), "areas 不可為空");
    let mut ids = BTreeSet::new();
    let mut source_cells = 0_usize;
    for area in &geography.areas {
        ensure!(ids.insert(area.id.as_str()), "重複 area id：{}", area.id);
        validate_area(area, topology_ids, biome_names)?;
        source_cells += area.source_cell_count;
    }
    ensure!(
        source_cells == geography.metrics.aggregated_land_cells,
        "area cell 總數不守恆"
    );
    Ok(ids)
}

/// 驗證單一區塊身分、引用、狀態、幾何與描述；輸入只讀，沒有副作用。
fn validate_area(
    area: &PhysicalArea,
    topology_ids: &BTreeSet<&str>,
    biome_names: &BTreeSet<&str>,
) -> Result<()> {
    // 身分與狀態先守住 non-canon/material 邊界，再檢查作者物理數值。
    ensure_nonblank("area.id", &area.id)?;
    ensure!(
        area.world_id == WHITE_TOWER_WORLD_ID,
        "{} worldId 不符",
        area.id
    );
    ensure!(
        topology_ids.contains(area.topology_node_ref.as_str()),
        "{} topologyNodeRef 不存在",
        area.id
    );
    ensure!(
        area.parent_place_id == format!("topology:{}", area.topology_node_ref),
        "{} parentPlaceId 不符",
        area.id
    );
    ensure!(
        area.kind == "physical_area_candidate",
        "{} kind 不符",
        area.id
    );
    ensure!(area.scale == "terrain_area", "{} scale 不符", area.id);
    ensure!(
        !area.canon && area.status == "material",
        "{} 不可冒充正典場所",
        area.id
    );
    ensure!(area.traversable, "{} 候選地理區塊應可通行", area.id);
    ensure!(
        area.source_kind == "azgaar_physical",
        "{} sourceKind 不符",
        area.id
    );
    validate_area_physics(area, biome_names)?;
    ensure_nonblank(&format!("{}.name", area.id), &area.name)?;
    ensure!(
        area.description.contains("不是正式地名"),
        "{} 必須明示名稱不是正典",
        area.id
    );
    Ok(())
}

/// 驗證單一區塊的聚合大小、正規化範圍、高度與生態；輸入只讀，沒有副作用。
fn validate_area_physics(area: &PhysicalArea, biome_names: &BTreeSet<&str>) -> Result<()> {
    // 24 cells 是本 profile 的碎片合併下限，避免生成無意義微區塊。
    ensure!(area.source_cell_count >= 24, "{} 未完成碎片合併", area.id);
    ensure_normalized_point(&area.id, &area.centroid)?;
    ensure_normalized_bounds(&area.id, &area.bounds)?;
    ensure!(
        area.bounds.min_x <= area.centroid.x && area.centroid.x <= area.bounds.max_x,
        "{} centroid.x 不在 bounds",
        area.id
    );
    ensure!(
        area.bounds.min_y <= area.centroid.y && area.centroid.y <= area.bounds.max_y,
        "{} centroid.y 不在 bounds",
        area.id
    );
    ensure!(
        area.height.min >= 20 && area.height.min <= area.height.max,
        "{} height 範圍不符",
        area.id
    );
    ensure!(
        area.height.mean >= f64::from(area.height.min)
            && area.height.mean <= f64::from(area.height.max),
        "{} mean height 不在範圍",
        area.id
    );
    ensure!(
        !area.biome_names.is_empty(),
        "{} biomeNames 不可為空",
        area.id
    );
    ensure!(
        area.biome_names
            .iter()
            .all(|name| biome_names.contains(name.as_str())),
        "{} 引用了未宣告 biome",
        area.id
    );
    Ok(())
}

/// 驗證陸塊 id、面積與主要陸塊 cell 守恆；輸入只讀，沒有外部副作用。
pub(super) fn validate_land_features(geography: &PhysicalGeography) -> Result<()> {
    // 大於等於 100 cells 的 continent/island 才進聚合，其餘小礁島明示略去。
    let mut ids = BTreeSet::new();
    let mut all_cells = 0_usize;
    let mut primary_cells = 0_usize;
    let mut primary_groups = Vec::new();
    for feature in &geography.land_features {
        ensure!(
            ids.insert(feature.id),
            "重複 land feature id：{}",
            feature.id
        );
        ensure!(feature.feature_type == "island", "陸地 feature type 漂移");
        ensure!(
            feature.cells > 0 && feature.area > 0.0,
            "land feature 數值不合理"
        );
        all_cells += feature.cells;
        if feature.cells >= 100 && matches!(feature.group.as_str(), "continent" | "island") {
            primary_cells += feature.cells;
            primary_groups.push(feature.group.as_str());
        }
    }
    primary_groups.sort_unstable();
    ensure!(
        primary_groups == ["continent", "island"],
        "必須恰有半島本體與風舶嶼兩個主要陸塊"
    );
    ensure!(
        all_cells == geography.metrics.generated_land_cells,
        "land feature cell 總數不守恆"
    );
    ensure!(
        primary_cells == geography.metrics.aggregated_land_cells,
        "主要陸塊 cell 總數不符"
    );
    Ok(())
}

/// 驗證每個區塊的陸塊與主要河流引用都存在；輸入只讀，沒有外部副作用。
pub(super) fn validate_area_references(geography: &PhysicalGeography) -> Result<()> {
    // 物理屬性只能引用同一份生成資料中的 feature 與 river。
    let feature_ids = geography
        .land_features
        .iter()
        .map(|feature| feature.id)
        .collect::<BTreeSet<_>>();
    let river_ids = geography
        .rivers
        .iter()
        .map(|river| river.source_river_id)
        .collect::<BTreeSet<_>>();
    for area in &geography.areas {
        ensure!(
            feature_ids.contains(&area.feature_id),
            "{} featureId 不存在",
            area.id
        );
        ensure!(
            area.major_river_ids.iter().all(|id| river_ids.contains(id)),
            "{} majorRiverIds 有斷鏈",
            area.id
        );
    }
    Ok(())
}

/// 驗證作者點位落在 0 到 1 的正規化空間；輸入只讀，沒有副作用。
fn ensure_normalized_point(label: &str, point: &NormalizedPoint) -> Result<()> {
    // 正規化座標只供作者聚合，不代表世界比例尺。
    ensure!(
        (0.0..=1.0).contains(&point.x) && (0.0..=1.0).contains(&point.y),
        "{label} centroid 超出正規化空間"
    );
    Ok(())
}

/// 驗證作者包圍盒落在正規化空間且方向正確；輸入只讀，沒有副作用。
fn ensure_normalized_bounds(label: &str, bounds: &NormalizedBounds) -> Result<()> {
    // min/max 顛倒或越界都表示聚合輸出損壞。
    ensure!(
        (0.0..=1.0).contains(&bounds.min_x) && (0.0..=1.0).contains(&bounds.max_x),
        "{label} x bounds 超出正規化空間"
    );
    ensure!(
        (0.0..=1.0).contains(&bounds.min_y) && (0.0..=1.0).contains(&bounds.max_y),
        "{label} y bounds 超出正規化空間"
    );
    ensure!(
        bounds.min_x <= bounds.max_x && bounds.min_y <= bounds.max_y,
        "{label} bounds 顛倒"
    );
    Ok(())
}

/// 驗證字串去除空白後仍有內容；輸入只讀，沒有副作用。
fn ensure_nonblank(label: &str, value: &str) -> Result<()> {
    // label 直接進錯誤訊息，讓壞資料能定位到欄位。
    ensure!(!value.trim().is_empty(), "{label} 不可為空白");
    Ok(())
}

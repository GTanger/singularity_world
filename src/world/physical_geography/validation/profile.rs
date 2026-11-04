//! 白塔約束生成 profile 的地表比例與位置回歸驗證。

use anyhow::{Result, ensure};

use crate::world::physical_geography::PhysicalGeography;

const SNOW_MAX_CELLS: usize = 2_500;
const SNOW_MAX_SOUTH: f64 = 0.30;
const WETLAND_MIN_CELLS: usize = 3_500;
const WETLAND_MAX_CELLS: usize = 6_000;

struct TerrainEnvelope {
    cells: usize,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

/// 驗證雪峰與水尾澤沒有退回過大的規則色塊；輸入只讀，沒有副作用。
pub(super) fn validate_terrain_profile(geography: &PhysicalGeography) -> Result<()> {
    // 數值只守住已目視通過的外包範圍；輪廓是否自然仍需人工看圖驗收。
    let snow = summarize(geography, "snow_and_ice")?;
    ensure!(snow.cells <= SNOW_MAX_CELLS, "白塔雪域再次過大");
    ensure!(snow.max_y <= SNOW_MAX_SOUTH, "白塔雪域再次沿青脊山南伸");
    ensure!(snow.max_x - snow.min_x <= 0.20, "白塔雪域東西跨度過大");

    let wetland = summarize(geography, "wetland")?;
    ensure!(
        (WETLAND_MIN_CELLS..=WETLAND_MAX_CELLS).contains(&wetland.cells),
        "水尾澤濕地份量失衡"
    );
    ensure!(
        wetland.min_y >= 0.50 && wetland.max_y <= 0.92,
        "水尾澤南北位置漂移"
    );
    ensure!(
        wetland.max_x - wetland.min_x <= 0.35 && wetland.max_y - wetland.min_y >= 0.25,
        "水尾澤濕地外包形狀失衡"
    );
    Ok(())
}

/// 彙總指定地表類別的 cell 數與外包範圍；輸入只讀，沒有副作用。
fn summarize(geography: &PhysicalGeography, terrain: &str) -> Result<TerrainEnvelope> {
    // area 已經過 bounds 驗證，這裡只做同類地表的整體包絡。
    let areas = geography
        .areas
        .iter()
        .filter(|area| area.terrain_class == terrain)
        .collect::<Vec<_>>();
    ensure!(!areas.is_empty(), "缺少 {terrain} 地表");
    Ok(TerrainEnvelope {
        cells: areas.iter().map(|area| area.source_cell_count).sum(),
        min_x: areas
            .iter()
            .map(|area| area.bounds.min_x)
            .fold(1.0, f64::min),
        min_y: areas
            .iter()
            .map(|area| area.bounds.min_y)
            .fold(1.0, f64::min),
        max_x: areas
            .iter()
            .map(|area| area.bounds.max_x)
            .fold(0.0, f64::max),
        max_y: areas
            .iter()
            .map(|area| area.bounds.max_y)
            .fold(0.0, f64::max),
    })
}

//! 三源、共同匯流、黑沼林下游與生成診斷的交叉關係驗證。

use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use crate::world::physical_geography::{MajorWatercourse, NormalizedPoint, PhysicalGeography};

use super::geometry::same_point;
use super::{COURSE_SPECS, WENYAO, valid_distance};

/// 驗證三條源流共用真匯流點，且下游穿過黑沼林後由南界接續。
pub(super) fn validate_relationships(geography: &PhysicalGeography) -> Result<()> {
    // 不採信輸出的布林值，直接重算角色間的幾何與流域關係。
    let hydrology = &geography.major_hydrology;
    let branches = hydrology
        .watercourses
        .iter()
        .filter(|course| course.role.ends_with("_headwater"))
        .collect::<Vec<_>>();
    let lower = hydrology
        .watercourses
        .iter()
        .find(|course| course.role == "shared_lower_river")
        .ok_or_else(|| anyhow::anyhow!("缺少中部下游大河"))?;
    ensure!(branches.len() == 3, "來源河必須恰有三條");
    validate_branches(
        &branches,
        hydrology.confluence_cell_id,
        &hydrology.confluence_point,
    )?;
    ensure!(
        lower.start_cell_id == hydrology.confluence_cell_id,
        "下游未從共同匯流 cell 起始"
    );
    ensure!(
        same_point(&lower.start_point, &hydrology.confluence_point),
        "下游未從共同匯流點起始"
    );
    validate_lower_course(lower)?;
    validate_hydrology_invariants(geography)
}

/// 驗證三源的起始河互異、流域相同且終點一致；輸入只讀。
fn validate_branches(
    branches: &[&MajorWatercourse],
    confluence_cell: u32,
    confluence_point: &NormalizedPoint,
) -> Result<()> {
    // 共同流域不足以證明真匯流，三條分支還必須落到同一個 cell。
    let starts = branches
        .iter()
        .map(|course| course.start_river_id)
        .collect::<BTreeSet<_>>();
    let basins = branches
        .iter()
        .map(|course| course.basin_id)
        .collect::<BTreeSet<_>>();
    ensure!(starts.len() == 3, "三源起始河不得重複");
    ensure!(basins.len() == 1, "三源不在同一流域");
    for branch in branches {
        ensure!(
            branch.end_cell_id == confluence_cell,
            "來源河未落到共同匯流 cell"
        );
        ensure!(
            same_point(&branch.end_point, confluence_point),
            "來源河未落到共同匯流點"
        );
        ensure!(
            branch.confluence_fit_distance.is_some(),
            "來源河缺匯流貼合距離"
        );
        ensure!(
            branch.black_marsh_fit_distance.is_none(),
            "來源河不應宣稱抵達黑沼林"
        );
    }
    Ok(())
}

/// 驗證共同下游的文曜、黑沼林與南界位置契約；輸入只讀。
fn validate_lower_course(lower: &MajorWatercourse) -> Result<()> {
    // 南界是縮略圖外的接續，不代表河流在此入海或世界結束。
    ensure!(
        lower.confluence_fit_distance.is_none(),
        "下游不應重複分支匯流欄位"
    );
    ensure!(lower.source_fit_distance <= 0.075, "三源匯流點離文曜過遠");
    ensure!(
        lower
            .black_marsh_fit_distance
            .is_some_and(|value| value <= 0.055),
        "下游只擦過或錯過黑沼林"
    );
    ensure!(lower.end_point.y >= 0.94, "下游未接續至半島南界");
    ensure!(
        (lower.end_point.x - 0.5).abs() <= 0.12,
        "下游從西南海岸錯誤流出"
    );
    let computed = point_distance(&lower.start_point, WENYAO);
    ensure!(
        (computed - lower.source_fit_distance).abs() <= 0.000_002,
        "匯流距離紀錄不符"
    );
    Ok(())
}

/// 驗證水文選擇診斷恰好覆蓋三條來源河；輸入只讀。
pub(super) fn validate_diagnostics(geography: &PhysicalGeography) -> Result<()> {
    // 診斷值讓未來 profile 漂移能說明選中了哪條生成河，而不是只留布林結果。
    let diagnostics = &geography.major_hydrology.selection_diagnostics;
    ensure!(diagnostics.len() == 3, "水文選擇診斷必須恰有三筆");
    let mut refs = BTreeSet::new();
    for diagnostic in diagnostics {
        ensure!(
            refs.insert(diagnostic.topology_node_ref.as_str()),
            "水文診斷角色重複"
        );
        ensure!(
            diagnostic.score.is_finite() && diagnostic.score >= 0.0,
            "水文選擇分數無效"
        );
        ensure!(
            diagnostic.checkpoint_distances.len() == 8,
            "水文檢查點數量漂移"
        );
        ensure!(
            diagnostic
                .checkpoint_distances
                .iter()
                .copied()
                .all(valid_distance),
            "水文檢查點距離無效"
        );
    }
    for (topology_ref, role, _) in COURSE_SPECS {
        if role.ends_with("_headwater") {
            ensure!(
                refs.contains(topology_ref),
                "水文診斷缺少來源河：{topology_ref}"
            );
        }
    }
    Ok(())
}

/// 驗證生成器聲明的五項水文 invariant 全部成立；輸入只讀。
fn validate_hydrology_invariants(geography: &PhysicalGeography) -> Result<()> {
    // 幾何已重算；布林欄位也必須同步，避免證據互相矛盾。
    let value = &geography.major_hydrology.invariants;
    ensure!(value.distinct_headwaters, "三源起始河 invariant 未成立");
    ensure!(value.shared_basin, "共同流域 invariant 未成立");
    ensure!(
        value.confluence_near_wenyao,
        "文曜附近匯流 invariant 未成立"
    );
    ensure!(
        value.lower_river_reaches_black_marsh,
        "黑沼林下游 invariant 未成立"
    );
    ensure!(
        value.lower_river_continues_south,
        "南界接續 invariant 未成立"
    );
    let distance = point_distance(&geography.major_hydrology.confluence_point, WENYAO);
    ensure!(distance <= 0.075, "共同匯流點離文曜過遠");
    Ok(())
}

/// 計算正規化點到固定參考點的歐氏距離；輸入只讀。
fn point_distance(point: &NormalizedPoint, expected: (f64, f64)) -> f64 {
    (point.x - expected.0).hypot(point.y - expected.1)
}

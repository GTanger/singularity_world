//! 三源匯流與水尾澤下游主幹的作者映射契約驗證。

mod geometry;
mod relationships;

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};

use crate::world::physical_geography::{MajorWatercourse, PhysicalGeography};

use geometry::{validate_course_geometry, validate_point};
use relationships::{validate_diagnostics, validate_relationships};

const COURSE_SPECS: [(&str, &str, &str); 4] = [
    (
        "wt-c-north-source-river",
        "north_headwater",
        "中部三河北來源",
    ),
    ("wt-c-west-source-river", "west_headwater", "中部三河西來源"),
    ("wt-c-east-source-river", "east_headwater", "中部三河東來源"),
    (
        "wt-c-central-lower-river",
        "shared_lower_river",
        "中部下游大河",
    ),
];
const WENYAO: (f64, f64) = (0.46, 0.45);

/// 驗證主要水系確實映射四個既有拓撲角色，且精確河道仍維持候選身分。
pub(super) fn validate(geography: &PhysicalGeography, topology_ids: &BTreeSet<&str>) -> Result<()> {
    // 先驗每條河道，再檢查四條之間的匯流關係與生成診斷。
    let hydrology = &geography.major_hydrology;
    ensure!(
        hydrology.status == "working_proposal",
        "主要水系不可冒充 accepted"
    );
    ensure!(!hydrology.canon, "精確主要河道不可冒充正典");
    ensure!(
        hydrology.watercourses.len() == 4,
        "主要水系必須恰有四個結構角色"
    );
    validate_point(&hydrology.confluence_point, "匯流點")?;
    ensure!(
        (hydrology.confluence_cell_id as usize) < geography.metrics.packed_cells,
        "匯流 cell 越界"
    );
    let rivers = geography
        .rivers
        .iter()
        .map(|river| (river.source_river_id, river))
        .collect::<BTreeMap<_, _>>();
    let mut refs = BTreeSet::new();
    for course in &hydrology.watercourses {
        validate_course(course, geography, topology_ids, &rivers)?;
        ensure!(
            refs.insert(course.topology_node_ref.as_str()),
            "主要水系拓撲角色重複"
        );
    }
    validate_required_refs(&refs)?;
    validate_relationships(geography)?;
    validate_diagnostics(geography)
}

/// 驗證一條主要河道的身分、幾何與 Azgaar 河流引用；輸入只讀。
fn validate_course(
    course: &MajorWatercourse,
    geography: &PhysicalGeography,
    topology_ids: &BTreeSet<&str>,
    rivers: &BTreeMap<u32, &crate::world::physical_geography::PhysicalRiver>,
) -> Result<()> {
    // 拓撲角色是既有正典；這裡只允許非正典的精確路徑映射到它。
    let spec = COURSE_SPECS
        .iter()
        .find(|spec| spec.0 == course.topology_node_ref)
        .ok_or_else(|| anyhow::anyhow!("未知主要水系角色：{}", course.topology_node_ref))?;
    ensure!(
        topology_ids.contains(course.topology_node_ref.as_str()),
        "主要水系拓撲引用不存在"
    );
    ensure!(
        course.role == spec.1 && course.structural_label == spec.2,
        "主要水系角色標記錯置"
    );
    ensure!(
        course.status == "working_proposal" && !course.canon,
        "精確河道不可升格正典"
    );
    validate_course_geometry(course, geography.metrics.packed_cells)?;
    validate_course_rivers(course, rivers)?;
    ensure!(
        course.source_fit_distance.is_finite() && course.source_fit_distance >= 0.0,
        "來源貼合距離無效"
    );
    ensure!(
        course.confluence_fit_distance.is_none_or(valid_distance),
        "匯流貼合距離無效"
    );
    ensure!(
        course.black_marsh_fit_distance.is_none_or(valid_distance),
        "水尾澤貼合距離無效"
    );
    Ok(())
}

/// 驗證主要河道引用的生成河、起始河與流域一致；輸入只讀。
fn validate_course_rivers(
    course: &MajorWatercourse,
    rivers: &BTreeMap<u32, &crate::world::physical_geography::PhysicalRiver>,
) -> Result<()> {
    // 一條結構水道可跨多個 Azgaar 河段，但必須全在同一實際流域。
    ensure!(
        rivers.contains_key(&course.start_river_id),
        "主要水系 startRiverId 不存在"
    );
    ensure!(
        rivers.contains_key(&course.basin_id),
        "主要水系 basinId 不存在"
    );
    ensure!(
        !course.generated_river_ids.is_empty(),
        "主要水系缺生成河段引用"
    );
    let ids = course
        .generated_river_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    ensure!(
        ids.len() == course.generated_river_ids.len(),
        "主要水系生成河段重複"
    );
    ensure!(
        ids.contains(&course.start_river_id),
        "主要水系未包含起始河段"
    );
    for id in ids {
        let river = rivers
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("主要水系河段 {id} 不存在"))?;
        ensure!(river.basin_id == course.basin_id, "主要水系跨越不同流域");
    }
    Ok(())
}

/// 驗證四個必要拓撲角色沒有遺漏；輸入只讀。
fn validate_required_refs(refs: &BTreeSet<&str>) -> Result<()> {
    // 精確比對可阻止新增第五條未經討論的「主要河」混入。
    for (topology_ref, _, _) in COURSE_SPECS {
        ensure!(
            refs.contains(topology_ref),
            "缺少主要水系角色：{topology_ref}"
        );
    }
    Ok(())
}

/// 判斷診斷距離為有限非負值；輸入只讀。
fn valid_distance(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

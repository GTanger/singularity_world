//! 主要河道簡化線、端點與正規化座標驗證。

use anyhow::{Result, ensure};

use crate::world::physical_geography::{MajorWatercourse, NormalizedPoint};

/// 驗證主要河道路徑端點、cell 邊界與正規化座標；輸入只讀。
pub(super) fn validate_course_geometry(
    course: &MajorWatercourse,
    packed_cells: usize,
) -> Result<()> {
    // path 是簡化顯示線；sourceCellCount 保存未簡化的原始追蹤長度。
    ensure!(course.path.len() >= 2, "主要河道路徑過短");
    ensure!(
        course.source_cell_count >= course.path.len(),
        "河道路徑簡化計數矛盾"
    );
    ensure!(
        (course.start_cell_id as usize) < packed_cells,
        "主要河道起點 cell 越界"
    );
    ensure!(
        (course.end_cell_id as usize) < packed_cells,
        "主要河道終點 cell 越界"
    );
    ensure!(
        course.start_cell_id != course.end_cell_id,
        "主要河道不可零長度"
    );
    for point in &course.path {
        validate_point(point, "主要河道路徑")?;
    }
    validate_point(&course.start_point, "主要河道起點")?;
    validate_point(&course.end_point, "主要河道終點")?;
    ensure!(
        same_point(&course.start_point, &course.path[0]),
        "主要河道起點與 path 不符"
    );
    ensure!(
        same_point(&course.end_point, course.path.last().expect("path checked")),
        "主要河道終點與 path 不符"
    );
    Ok(())
}

/// 驗證一個作者座標位於有限的正規化畫布；輸入只讀。
pub(super) fn validate_point(point: &NormalizedPoint, label: &str) -> Result<()> {
    // NaN 會繞過一般範圍比較，因此先檢查有限值。
    ensure!(
        point.x.is_finite() && point.y.is_finite(),
        "{label}含非有限座標"
    );
    ensure!((0.0..=1.0).contains(&point.x), "{label} x 越界");
    ensure!((0.0..=1.0).contains(&point.y), "{label} y 越界");
    Ok(())
}

/// 比較兩個已量化至六位小數的作者座標；輸入只讀。
pub(super) fn same_point(left: &NormalizedPoint, right: &NormalizedPoint) -> bool {
    (left.x - right.x).abs() <= 0.000_001 && (left.y - right.y).abs() <= 0.000_001
}

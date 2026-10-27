#![forbid(unsafe_code)]
//! 驗證白塔半島 Azgaar 物理地理橋接，不寫入資料庫、不啟用候選路線。

use anyhow::Result;
use singularity_world::world::physical_geography::builtin_white_tower_physical_geography;

fn main() -> Result<()> {
    // 載入並驗證內建資料，再建立不啟用路線的記憶體候選圖。
    let geography = builtin_white_tower_physical_geography()?;
    let graph = geography.candidate_place_graph()?;
    println!(
        "白塔半島物理地理橋接通過：{} 個作者場所候選、{} 條候選路線、{} 條生成河流、{} 個主要水系角色、{} 個場所圖節點；runtimeAuthority=false，playerFacingMap=false。",
        geography.areas.len(),
        geography.route_candidates.len(),
        geography.rivers.len(),
        geography.major_hydrology.watercourses.len(),
        graph.places().count()
    );
    Ok(())
}

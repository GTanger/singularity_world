use anyhow::Result;

use super::*;

#[test]
fn bundled_white_tower_physical_geography_passes_strict_validation() -> Result<()> {
    // 載入內建資料並確認固定生成規模與碎片合併下限。
    let geography = builtin_white_tower_physical_geography()?;
    assert_eq!(geography.areas.len(), 305);
    assert_eq!(geography.route_candidates.len(), 1_644);
    assert_eq!(geography.rivers.len(), 1_129);
    assert_eq!(geography.major_hydrology.watercourses.len(), 4);
    assert_eq!(geography.metrics.aggregated_land_cells, 54_911);
    assert!(
        geography
            .areas
            .iter()
            .all(|area| area.source_cell_count >= 24)
    );
    Ok(())
}

#[test]
fn validator_rejects_major_hydrology_promoted_to_canon() -> Result<()> {
    // 模擬把精確 Azgaar 河道升格；正典只接受既有四段拓撲角色。
    let mut geography = builtin_white_tower_physical_geography()?;
    geography.major_hydrology.canon = true;
    let error = validate_physical_geography(&geography).expect_err("canon promotion must fail");
    assert!(error.to_string().contains("不可冒充正典"));
    Ok(())
}

#[test]
fn validator_rejects_confluence_moved_away_from_wenyao() -> Result<()> {
    // 模擬共同匯流點向南漂移，避免三源再次到南哨附近才真正合流。
    let mut geography = builtin_white_tower_physical_geography()?;
    geography.major_hydrology.confluence_point.y = 0.7;
    let error = validate_physical_geography(&geography).expect_err("late confluence must fail");
    assert!(error.to_string().contains("共同匯流點"));
    Ok(())
}

#[test]
fn validator_rejects_lower_river_escaping_to_southwest_coast() -> Result<()> {
    // 模擬主幹在南哨後轉向西南海岸，必須阻擋這種只擦過黑沼林的退化。
    let mut geography = builtin_white_tower_physical_geography()?;
    let lower = geography
        .major_hydrology
        .watercourses
        .iter_mut()
        .find(|course| course.role == "shared_lower_river")
        .expect("lower river");
    lower.end_point.x = 0.2;
    lower.path.last_mut().expect("lower path").x = 0.2;
    let error = validate_physical_geography(&geography).expect_err("southwest escape must fail");
    assert!(error.to_string().contains("西南海岸"));
    Ok(())
}

#[test]
fn validator_rejects_oversized_snow_field() -> Result<()> {
    // 模擬雪地候選向南膨脹，防止白塔峰再次變成北半島冰蓋。
    let mut geography = builtin_white_tower_physical_geography()?;
    let snow = geography
        .areas
        .iter_mut()
        .find(|area| area.terrain_class == "snow_and_ice")
        .expect("snow area");
    snow.bounds.max_y = 0.5;
    let error = validate_physical_geography(&geography).expect_err("oversized snow must fail");
    assert!(error.to_string().contains("雪域"));
    Ok(())
}

#[test]
fn candidates_convert_to_place_graph_without_becoming_runtime_routes() -> Result<()> {
    // 建圖只驗證結構，candidate 路線不應參與 active 尋路。
    let geography = builtin_white_tower_physical_geography()?;
    let graph = geography.candidate_place_graph()?;
    assert_eq!(graph.places().count(), geography.areas.len());
    let first = &geography.areas[0];
    let place = graph.place(&first.id).expect("candidate place");
    assert!(!place.canon);
    assert_eq!(place.status, "material");
    assert!(
        graph
            .find_path(&geography.areas[0].id, &geography.areas[1].id)
            .is_none()
    );
    assert!(
        geography
            .world_route_candidates()
            .iter()
            .all(|route| route.status == "candidate")
    );
    Ok(())
}

#[test]
fn validator_rejects_authoring_data_promoted_to_runtime() -> Result<()> {
    // 模擬誤把作者資料升格，必須在驗證入口被拒絕。
    let mut geography = builtin_white_tower_physical_geography()?;
    geography.runtime_authority = true;
    let error = validate_physical_geography(&geography).expect_err("runtime promotion must fail");
    assert!(error.to_string().contains("runtime 權威"));
    Ok(())
}

#[test]
fn validator_rejects_route_with_missing_place() -> Result<()> {
    // 模擬斷裂終點，確保路線引用不會帶病進 PlaceGraph。
    let mut geography = builtin_white_tower_physical_geography()?;
    geography.route_candidates[0].to_place_id = "missing-place".to_string();
    let error = validate_physical_geography(&geography).expect_err("missing endpoint must fail");
    assert!(error.to_string().contains("終點不存在"));
    Ok(())
}

#[test]
fn validator_rejects_cell_to_place_explosion() -> Result<()> {
    // 模擬聚合失效，確保 cell 不會直接膨脹成世界場所。
    let mut geography = builtin_white_tower_physical_geography()?;
    geography.metrics.aggregated_land_cells = geography.areas.len() * 10;
    geography.metrics.generated_land_cells =
        geography.metrics.aggregated_land_cells + geography.metrics.omitted_tiny_islet_cells;
    let error =
        validate_physical_geography(&geography).expect_err("cell-place explosion must fail");
    assert!(error.to_string().contains("一 cell 一場所"));
    Ok(())
}

#[test]
fn validator_rejects_anchor_mapped_to_a_different_topology_node() -> Result<()> {
    // 模擬錨點借用其他區域的 area，確保掛回正典節點時不只檢查 id 存在。
    let mut geography = builtin_white_tower_physical_geography()?;
    let anchor_topology = geography.anchor_mappings[0].topology_node_ref.clone();
    let wrong_area = geography
        .areas
        .iter()
        .find(|area| area.topology_node_ref != anchor_topology)
        .expect("area under another topology")
        .id
        .clone();
    geography.anchor_mappings[0].area_ref = Some(wrong_area);
    let error = validate_physical_geography(&geography).expect_err("wrong topology must fail");
    assert!(error.to_string().contains("不同拓撲節點"));
    Ok(())
}

#[test]
fn validator_rejects_wenyao_sitting_on_a_river_cell() -> Result<()> {
    // 文曜採用附近既有古河階格；若日後又抓回河道格，必須立即拒絕。
    let mut geography = builtin_white_tower_physical_geography()?;
    let wenyao = geography
        .anchor_mappings
        .iter_mut()
        .find(|anchor| anchor.topology_node_ref == "wt-c-wenyao")
        .expect("wenyao anchor");
    wenyao.river = 1;
    let error = validate_physical_geography(&geography).expect_err("river cell must fail");
    assert!(error.to_string().contains("文曜不可壓在河道"));
    Ok(())
}

#[test]
fn validator_rejects_south_sentinel_sitting_in_wetland() -> Result<()> {
    // 南哨採用現成穩定乾地格；不可因最近距離而退回黑沼濕地。
    let mut geography = builtin_white_tower_physical_geography()?;
    let sentinel = geography
        .anchor_mappings
        .iter_mut()
        .find(|anchor| anchor.topology_node_ref == "wt-s-south-sentinel")
        .expect("south sentinel anchor");
    sentinel.biome_name = "Wetland".to_string();
    let error = validate_physical_geography(&geography).expect_err("wetland cell must fail");
    assert!(error.to_string().contains("南哨不可落在濕地"));
    Ok(())
}

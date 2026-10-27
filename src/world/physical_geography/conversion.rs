//! 非正典物理地理資料到既有場所／路線型別的純轉換；不寫資料庫、不啟用路線。

use anyhow::Result;

use super::{PhysicalArea, PhysicalGeography, PhysicalRouteCandidate};
use crate::world::place::{PlaceGraph, WHITE_TOWER_WORLD_ID, WorldPlace, WorldRoute};

impl PhysicalGeography {
    /// 把全部物理區塊轉成非正典 `WorldPlace` 候選並回傳；沒有外部副作用。
    pub fn world_place_candidates(&self) -> Vec<WorldPlace> {
        // 每個聚合區塊只產生一個場所候選，不把底層 cell 展開成場所。
        self.areas.iter().map(to_world_place).collect()
    }

    /// 把全部物理相鄰關係轉成 `candidate` 路線並回傳；沒有外部副作用。
    pub fn world_route_candidates(&self) -> Vec<WorldRoute> {
        // status 原樣保留 candidate，避免 PlaceGraph 尋路把作者資料當 active 路線。
        self.route_candidates.iter().map(to_world_route).collect()
    }

    /// 用場所與路線候選建立結構檢查用 `PlaceGraph`；只配置記憶體，不寫 runtime。
    pub fn candidate_place_graph(&self) -> Result<PlaceGraph> {
        // PlaceGraph::new 驗證端點與方向，但 candidate 路線不參與實際尋路。
        PlaceGraph::new(self.world_place_candidates(), self.world_route_candidates())
    }
}

/// 把一個物理區塊轉成非正典場所候選；輸入只讀，回傳新值，沒有副作用。
fn to_world_place(area: &PhysicalArea) -> WorldPlace {
    // 標籤保存可查詢的物理性質，不把作者正規化座標當 runtime 身分。
    let mut tags = vec![
        format!("terrain:{}", area.terrain_class),
        format!("elevation:{}", area.elevation_band),
        format!("feature:{}", area.feature_id),
    ];
    tags.extend(
        area.biome_names
            .iter()
            .map(|biome| format!("biome:{biome}")),
    );
    WorldPlace {
        id: area.id.clone(),
        world_id: area.world_id.clone(),
        topology_node_ref: Some(area.topology_node_ref.clone()),
        parent_place_id: Some(area.parent_place_id.clone()),
        name: area.name.clone(),
        kind: area.kind.clone(),
        scale: area.scale.clone(),
        canon: false,
        status: area.status.clone(),
        traversable: area.traversable,
        source_kind: area.source_kind.clone(),
        zone: WHITE_TOWER_WORLD_ID.to_string(),
        tags,
        description: area.description.clone(),
        legacy_grid_x: None,
        legacy_grid_y: None,
    }
}

/// 把一條物理連線轉成非正典路線候選；輸入只讀，回傳新值，沒有副作用。
fn to_world_route(route: &PhysicalRouteCandidate) -> WorldRoute {
    // 相對成本只進作者候選；status 仍是 candidate，不能觸發 active 尋路。
    WorldRoute {
        id: route.id.clone(),
        from_place_id: route.from_place_id.clone(),
        direction: route.direction.clone(),
        to_place_id: route.to_place_id.clone(),
        mode: route.mode.clone(),
        travel_cost: route.relative_cost,
        canon: false,
        status: route.status.clone(),
        source_kind: route.source_kind.clone(),
    }
}

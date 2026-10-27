//! 物理路線候選、風舶渡船、河流引用與候選圖連通性驗證。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Result, ensure};

use crate::world::physical_geography::{PhysicalGeography, PhysicalRiver, PhysicalRouteCandidate};

/// 驗證全部路線 id、端點、方向、狀態與渡船關係；輸入只讀，沒有外部副作用。
pub(super) fn validate_routes(
    geography: &PhysicalGeography,
    topology_ids: &BTreeSet<&str>,
    area_ids: &BTreeSet<&str>,
) -> Result<()> {
    // directed_pairs 防止同一對端點重複灌入候選路線。
    let mut ids = BTreeSet::new();
    let mut directed_pairs = BTreeSet::new();
    let mut ferry = Vec::new();
    for route in &geography.route_candidates {
        ensure!(ids.insert(route.id.as_str()), "重複 route id：{}", route.id);
        ensure!(
            directed_pairs.insert((route.from_place_id.as_str(), route.to_place_id.as_str())),
            "重複有向 route：{}",
            route.id
        );
        validate_route(route, topology_ids, area_ids)?;
        if route.mode == "ferry" {
            ferry.push(route);
        }
    }
    validate_ferry(&ferry)
}

/// 驗證單一路線端點、方向、成本、狀態與通行模式；輸入只讀，沒有副作用。
fn validate_route(
    route: &PhysicalRouteCandidate,
    topology_ids: &BTreeSet<&str>,
    area_ids: &BTreeSet<&str>,
) -> Result<()> {
    // 方向限制在本輪物理平面；上下進出由正式場所設計另行建立。
    ensure!(
        area_ids.contains(route.from_place_id.as_str()),
        "{} 起點不存在",
        route.id
    );
    ensure!(
        area_ids.contains(route.to_place_id.as_str()),
        "{} 終點不存在",
        route.id
    );
    ensure!(
        route.from_place_id != route.to_place_id,
        "{} 不可連向自己",
        route.id
    );
    ensure!(
        matches!(route.direction.as_str(), "東" | "西" | "南" | "北"),
        "{} 方向無效",
        route.id
    );
    ensure!(
        matches!(route.mode.as_str(), "walk" | "difficult_ground" | "ferry"),
        "{} mode 無效",
        route.id
    );
    ensure!(route.relative_cost > 0, "{} 成本必須大於零", route.id);
    ensure!(
        !route.canon && route.status == "candidate",
        "{} 不可冒充 active 正典路線",
        route.id
    );
    ensure!(
        route.source_kind == "azgaar_physical",
        "{} sourceKind 不符",
        route.id
    );
    validate_route_surface(route, topology_ids)
}

/// 驗證陸路共同邊界或渡船 via 節點；輸入只讀，沒有副作用。
fn validate_route_surface(
    route: &PhysicalRouteCandidate,
    topology_ids: &BTreeSet<&str>,
) -> Result<()> {
    // 渡船可跨水域無共同陸邊；其他路線必須由真實 cell 邊界產生。
    if route.mode == "ferry" {
        let via = route.via_topology_node_ref.as_deref().unwrap_or_default();
        ensure!(
            topology_ids.contains(via),
            "{} viaTopologyNodeRef 不存在",
            route.id
        );
    } else {
        ensure!(route.shared_border_cells > 0, "{} 陸路缺共同邊界", route.id);
        ensure!(
            route.via_topology_node_ref.is_none(),
            "{} 陸路不應有 viaTopologyNodeRef",
            route.id
        );
    }
    Ok(())
}

/// 驗證風舶嶼與風舶角恰有一組經港灣道的雙向渡船；輸入只讀，沒有副作用。
fn validate_ferry(ferry: &[&PhysicalRouteCandidate]) -> Result<()> {
    // 渡船是兩個主要陸塊保持整體連通的唯一非陸接候選。
    ensure!(ferry.len() == 2, "風舶嶼渡船候選必須恰有雙向兩條");
    ensure!(
        ferry
            .iter()
            .all(|route| route.via_topology_node_ref.as_deref()
                == Some("wt-w-natural-harbor-channel")),
        "渡船必須經天然港灣道"
    );
    ensure!(
        ferry[0].from_place_id == ferry[1].to_place_id,
        "渡船起終點未互換"
    );
    ensure!(
        ferry[0].to_place_id == ferry[1].from_place_id,
        "渡船候選必須互為反向"
    );
    Ok(())
}

/// 驗證河流 id、父河、流域與非負物理量；輸入只讀，沒有外部副作用。
pub(super) fn validate_rivers(geography: &PhysicalGeography) -> Result<()> {
    // Azgaar 根河可把 parent 指向自身；只禁止引用不存在的河。
    let ids = geography
        .rivers
        .iter()
        .map(|river| river.source_river_id)
        .collect::<BTreeSet<_>>();
    ensure!(ids.len() == geography.rivers.len(), "river id 重複");
    for river in &geography.rivers {
        validate_river(river, &ids, geography.metrics.packed_cells)?;
    }
    Ok(())
}

/// 驗證一條河的父子／流域引用與可選數值；輸入只讀，沒有副作用。
fn validate_river(river: &PhysicalRiver, ids: &BTreeSet<u32>, packed_cells: usize) -> Result<()> {
    // 0 表示沒有父河，其餘 parent/basin 必須存在於同一份輸出。
    ensure!(
        river.parent_river_id == 0 || ids.contains(&river.parent_river_id),
        "river {} parent 不存在",
        river.source_river_id
    );
    ensure!(
        ids.contains(&river.basin_id),
        "river {} basin 不存在",
        river.source_river_id
    );
    ensure!(
        river
            .source_cell_id
            .is_none_or(|id| (id as usize) < packed_cells),
        "river {} source cell 越界",
        river.source_river_id
    );
    ensure!(
        river
            .mouth_cell_id
            .is_none_or(|id| (id as usize) < packed_cells),
        "river {} mouth cell 越界",
        river.source_river_id
    );
    validate_nonnegative("length", river.length, river.source_river_id)?;
    validate_nonnegative("discharge", river.discharge, river.source_river_id)?;
    validate_nonnegative("width", river.width, river.source_river_id)
}

/// 驗證可選河流數值不是負數；輸入只讀，沒有副作用。
fn validate_nonnegative(label: &str, value: Option<f64>, river_id: u32) -> Result<()> {
    // 缺值可保留，負值代表來源資料損壞。
    ensure!(
        value.is_none_or(|number| number >= 0.0),
        "river {river_id} {label} 不可為負"
    );
    Ok(())
}

/// 忽略 candidate 狀態做作者圖 BFS，驗證所有物理區塊可達；沒有外部副作用。
pub(super) fn validate_connectivity(
    geography: &PhysicalGeography,
    area_ids: &BTreeSet<&str>,
) -> Result<()> {
    // 這只驗證作者候選圖結構；不把 candidate 路線變成 runtime active。
    let outgoing = build_outgoing(geography);
    let start = *area_ids
        .first()
        .ok_or_else(|| anyhow::anyhow!("沒有 area 可驗證"))?;
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(current) = queue.pop_front() {
        for next in outgoing.get(current).into_iter().flatten() {
            if seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    ensure!(
        seen.len() == area_ids.len(),
        "物理候選圖不連通：{}/{}",
        seen.len(),
        area_ids.len()
    );
    Ok(())
}

/// 把有向候選路線整理成 BFS 鄰接表並回傳；只配置記憶體，沒有外部副作用。
fn build_outgoing(geography: &PhysicalGeography) -> BTreeMap<&str, Vec<&str>> {
    // 狀態不在此篩選，因為目標是檢查作者候選圖本身是否完整。
    let mut outgoing = BTreeMap::new();
    for route in &geography.route_candidates {
        outgoing
            .entry(route.from_place_id.as_str())
            .or_insert_with(Vec::new)
            .push(route.to_place_id.as_str());
    }
    outgoing
}

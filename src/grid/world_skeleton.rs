//! Data-backed world skeleton loader.
//!
//! A skeleton is the non-random, designed set of cells that makes the world
//! traversable. It may fill missing cells and missing connections, but it must
//! not overwrite already polished player-facing copy in PostgreSQL.

use std::collections::HashSet;

use serde::Deserialize;

use super::cell::Terrain;
use super::coord::{Direction, SquareCoord};
use super::grid::{ExplicitExit, GridCell, SquareGrid};

const SINGULARITY_WORLD_SKELETON_JSON: &str =
    include_str!("../../data/world/singularity_world_skeleton.json");

#[derive(Debug, Deserialize)]
pub struct WorldSkeleton {
    id: String,
    name: String,
    version: u32,
    cells: Vec<SkeletonCell>,
}

#[derive(Debug, Deserialize)]
struct SkeletonCell {
    x: i32,
    y: i32,
    m4: String,
    name: String,
    terrain: Terrain,
    zone: String,
    #[serde(default)]
    tags: Vec<String>,
    description: String,
    #[serde(default)]
    explored: bool,
    #[serde(default)]
    exits: Vec<SkeletonExit>,
}

#[derive(Debug, Clone, Deserialize)]
struct SkeletonExit {
    dir: Direction,
    to: [i32; 2],
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WorldSkeletonApplyReport {
    pub inserted: usize,
    pub updated: usize,
}

impl WorldSkeletonApplyReport {
    pub fn changed(self) -> bool {
        self.inserted > 0 || self.updated > 0
    }
}

pub fn ensure_builtin_world_skeleton(
    grid: &mut SquareGrid,
) -> anyhow::Result<WorldSkeletonApplyReport> {
    let skeleton = load_builtin_world_skeleton()?;
    apply_world_skeleton(grid, &skeleton)
}

pub fn load_builtin_world_skeleton() -> anyhow::Result<WorldSkeleton> {
    let skeleton: WorldSkeleton = serde_json::from_str(SINGULARITY_WORLD_SKELETON_JSON)?;
    validate_world_skeleton(&skeleton)?;
    Ok(skeleton)
}

pub fn apply_world_skeleton(
    grid: &mut SquareGrid,
    skeleton: &WorldSkeleton,
) -> anyhow::Result<WorldSkeletonApplyReport> {
    validate_world_skeleton(skeleton)?;

    let mut report = WorldSkeletonApplyReport::default();

    for skeleton_cell in &skeleton.cells {
        let coord = SquareCoord::new(skeleton_cell.x, skeleton_cell.y);
        if let Some(existing) = grid.get_mut(coord) {
            let changed = merge_cell(existing, skeleton_cell);
            if changed {
                report.updated += 1;
            }
        } else {
            grid.insert(cell_from_skeleton(skeleton_cell));
            report.inserted += 1;
        }
    }

    if report.updated > 0 {
        grid.mark_dirty();
    }

    Ok(report)
}

fn validate_world_skeleton(skeleton: &WorldSkeleton) -> anyhow::Result<()> {
    if skeleton.id.trim().is_empty() {
        anyhow::bail!("world skeleton id is empty");
    }
    if skeleton.name.trim().is_empty() {
        anyhow::bail!("world skeleton name is empty");
    }
    if skeleton.version == 0 {
        anyhow::bail!("world skeleton version must be positive");
    }

    let mut coords = HashSet::new();
    for cell in &skeleton.cells {
        if cell.m4.trim().is_empty() {
            anyhow::bail!("world skeleton cell ({},{}) m4 is empty", cell.x, cell.y);
        }
        if cell.name.trim().is_empty() {
            anyhow::bail!("world skeleton cell ({},{}) name is empty", cell.x, cell.y);
        }
        let coord = SquareCoord::new(cell.x, cell.y);
        if !coords.insert(coord) {
            anyhow::bail!("duplicate world skeleton cell at {coord}");
        }
    }

    for cell in &skeleton.cells {
        let from = SquareCoord::new(cell.x, cell.y);
        for exit in &cell.exits {
            let target = SquareCoord::new(exit.to[0], exit.to[1]);
            if !coords.contains(&target) {
                anyhow::bail!(
                    "world skeleton exit target missing: {} {} -> {}",
                    from,
                    exit.dir,
                    target
                );
            }
        }
    }

    Ok(())
}

fn cell_from_skeleton(skeleton_cell: &SkeletonCell) -> GridCell {
    let coord = SquareCoord::new(skeleton_cell.x, skeleton_cell.y);
    let mut cell = GridCell::new(coord, skeleton_cell.terrain, skeleton_cell.name.clone());
    cell.zone = skeleton_cell.zone.clone();
    cell.tags = skeleton_cell.tags.clone();
    cell.description = skeleton_cell.description.clone();
    cell.explored = skeleton_cell.explored;
    cell.explicit_exits = exits_from_skeleton(skeleton_cell);
    cell
}

fn exits_from_skeleton(skeleton_cell: &SkeletonCell) -> Vec<ExplicitExit> {
    skeleton_cell
        .exits
        .iter()
        .map(|exit| ExplicitExit {
            dir: exit.dir,
            target: SquareCoord::new(exit.to[0], exit.to[1]),
        })
        .collect()
}

fn merge_cell(existing: &mut GridCell, skeleton_cell: &SkeletonCell) -> bool {
    let mut changed = false;

    let should_update_name = !skeleton_cell.name.trim().is_empty()
        && (existing.name.trim().is_empty()
            || should_rename_north_bank_ferry(existing.name.trim(), skeleton_cell));
    if should_update_name {
        existing.name = skeleton_cell.name.clone();
        changed = true;
    }
    if existing.zone.trim().is_empty() && !skeleton_cell.zone.trim().is_empty() {
        existing.zone = skeleton_cell.zone.clone();
        changed = true;
    }
    let existing_is_scene_stub = existing.tags.iter().any(|tag| tag == "scene_stub");
    let skeleton_is_scene_stub = skeleton_cell.tags.iter().any(|tag| tag == "scene_stub");
    let refreshes_stub_copy = skeleton_is_scene_stub && existing_is_scene_stub;
    let upgrades_stub_copy = existing_is_scene_stub && !skeleton_is_scene_stub;
    let should_update_description = !skeleton_cell.description.trim().is_empty()
        && (existing.description.trim().is_empty()
            || should_refresh_north_bank_ferry_description(&existing.description, skeleton_cell)
            || ((refreshes_stub_copy || upgrades_stub_copy)
                && existing.description != skeleton_cell.description));
    if should_update_description {
        existing.description = skeleton_cell.description.clone();
        changed = true;
    }
    if upgrades_stub_copy {
        let before_len = existing.tags.len();
        existing.tags.retain(|tag| tag != "scene_stub");
        if existing.tags.len() != before_len {
            changed = true;
        }
    }
    if !existing.explored && skeleton_cell.explored {
        existing.explored = true;
        changed = true;
    }

    for tag in &skeleton_cell.tags {
        if !existing.tags.iter().any(|existing_tag| existing_tag == tag) {
            existing.tags.push(tag.clone());
            changed = true;
        }
    }

    for exit in exits_from_skeleton(skeleton_cell) {
        if let Some(existing_exit) = existing
            .explicit_exits
            .iter()
            .find(|existing_exit| existing_exit.target == exit.target)
        {
            if existing_exit.dir != exit.dir {
                let target = existing_exit.target;
                if let Some(existing_exit) = existing
                    .explicit_exits
                    .iter_mut()
                    .find(|existing_exit| existing_exit.target == target)
                {
                    existing_exit.dir = exit.dir;
                    changed = true;
                }
            }
        } else {
            existing.explicit_exits.push(exit);
            changed = true;
        }
    }

    changed
}

fn should_rename_north_bank_ferry(existing_name: &str, skeleton_cell: &SkeletonCell) -> bool {
    is_north_bank_ferry_cell(skeleton_cell)
        && matches!(existing_name, "北岸等候處" | "北岸候渡處" | "北岸後渡處")
        && existing_name != skeleton_cell.name
}

fn should_refresh_north_bank_ferry_description(
    existing_description: &str,
    skeleton_cell: &SkeletonCell,
) -> bool {
    is_north_bank_ferry_cell(skeleton_cell)
        && !skeleton_cell.description.trim().is_empty()
        && existing_description != skeleton_cell.description
        && ["北岸等候處", "北岸候渡處", "北岸後渡處"]
            .iter()
            .any(|old_name| existing_description.contains(old_name))
}

fn is_north_bank_ferry_cell(skeleton_cell: &SkeletonCell) -> bool {
    skeleton_cell.m4 == "M4-027-01" && skeleton_cell.name == "北岸渡口"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_world_skeleton_loads_and_expands_empty_grid() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        assert_eq!(skeleton.id, "singularity_world_pangu");
        assert_eq!(skeleton.cells.len(), 119);
        assert_eq!(
            skeleton
                .cells
                .iter()
                .filter(|cell| cell.m4.starts_with("M4-"))
                .count(),
            109
        );

        let mut grid = SquareGrid::new();
        let report = apply_world_skeleton(&mut grid, &skeleton).unwrap();

        assert_eq!(report.inserted, skeleton.cells.len());
        assert_eq!(report.updated, 0);
        assert!(grid.get(SquareCoord::new(0, 0)).is_some());
        assert!(grid.get(SquareCoord::new(0, 1)).is_some());
        assert!(grid.can_walk(SquareCoord::new(0, 0), SquareCoord::new(0, 1)));
        assert!(grid.get(SquareCoord::new(0, 3)).is_some());
        assert!(grid.get(SquareCoord::new(0, 5)).is_some());
        assert!(grid.get(SquareCoord::new(2, 5)).is_some());
        assert!(grid.get(SquareCoord::new(-1, 6)).is_some());
        assert!(grid.get(SquareCoord::new(0, -4)).is_some());
        assert!(grid.get(SquareCoord::new(3, -2)).is_some());
        assert!(grid.get(SquareCoord::new(3, 0)).is_some());
        assert!(grid.get(SquareCoord::new(5, 1)).is_some());
        assert!(grid.get(SquareCoord::new(-1, -10)).is_some());
        assert!(grid.get(SquareCoord::new(-10, -3)).is_some());
        assert!(grid.get(SquareCoord::new(2, 8)).is_some());
        assert!(grid.get(SquareCoord::new(-7, 7)).is_some());
        assert!(grid.get(SquareCoord::new(-5, -4)).is_some());
        assert!(grid.get(SquareCoord::new(11, -6)).is_some());
    }

    #[test]
    fn builtin_world_skeleton_keeps_existing_player_copy() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let mut grid = SquareGrid::new();
        let mut cell = GridCell::new(SquareCoord::new(0, 0), Terrain::Urban, "自訂北岸");
        cell.description = "已經打磨過的畫面文字。".to_string();
        cell.explicit_exits.push(ExplicitExit {
            dir: Direction::South,
            target: SquareCoord::new(0, -1),
        });
        grid.insert(cell);
        grid.clear_dirty();

        let report = apply_world_skeleton(&mut grid, &skeleton).unwrap();
        let north_bank = grid.get(SquareCoord::new(0, 0)).unwrap();

        assert_eq!(report.inserted, skeleton.cells.len() - 1);
        assert_eq!(north_bank.name, "自訂北岸");
        assert_eq!(north_bank.description, "已經打磨過的畫面文字。");
        assert!(north_bank.tags.iter().any(|tag| tag == "m3_027"));
        assert!(
            north_bank
                .explicit_exits
                .iter()
                .any(|exit| exit.target == SquareCoord::new(0, 1))
        );
    }

    #[test]
    fn builtin_world_skeleton_renames_old_north_bank_scene_name() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let mut grid = SquareGrid::new();
        let mut cell = GridCell::new(SquareCoord::new(0, 0), Terrain::Urban, "北岸等候處");
        cell.description = "北岸等候處貼著河霧，踏板旁有人等船。".to_string();
        grid.insert(cell);
        grid.clear_dirty();

        let report = apply_world_skeleton(&mut grid, &skeleton).unwrap();
        let north_bank = grid.get(SquareCoord::new(0, 0)).unwrap();

        assert_eq!(report.inserted, skeleton.cells.len() - 1);
        assert_eq!(report.updated, 1);
        assert_eq!(north_bank.name, "北岸渡口");
        assert!(north_bank.description.contains("北岸渡口"));
        assert!(!north_bank.description.contains("北岸等候處"));
    }

    #[test]
    fn builtin_world_skeleton_refreshes_scene_stub_copy_only() {
        let skeleton = WorldSkeleton {
            id: "stub_refresh_test".to_string(),
            name: "stub refresh test".to_string(),
            version: 1,
            cells: vec![SkeletonCell {
                x: 4,
                y: -3,
                m4: "M4-TEST-01".to_string(),
                name: "測試佔位場景".to_string(),
                terrain: Terrain::Urban,
                zone: "test_zone".to_string(),
                tags: vec!["scene_stub".to_string()],
                description: "新的佔位描述。".to_string(),
                explored: false,
                exits: vec![],
            }],
        };
        let stub = skeleton.cells.first().unwrap();
        let coord = SquareCoord::new(stub.x, stub.y);
        let mut grid = SquareGrid::new();
        let mut cell = GridCell::new(coord, stub.terrain, stub.name.clone());
        cell.description = "舊的佔位描述。".to_string();
        cell.tags.push("scene_stub".to_string());
        grid.insert(cell);
        grid.clear_dirty();

        let report = apply_world_skeleton(&mut grid, &skeleton).unwrap();
        let refreshed = grid.get(coord).unwrap();

        assert_eq!(report.inserted, 0);
        assert_eq!(report.updated, 1);
        assert_eq!(refreshed.description, stub.description);
        assert!(refreshed.tags.iter().any(|tag| tag == "scene_stub"));
    }

    #[test]
    fn builtin_world_skeleton_upgrades_scene_stub_to_polished_copy() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let polished = skeleton
            .cells
            .iter()
            .find(|cell| cell.m4 == "M4-032-01")
            .unwrap();
        let coord = SquareCoord::new(polished.x, polished.y);
        let mut grid = SquareGrid::new();
        let mut cell = GridCell::new(coord, polished.terrain, polished.name.clone());
        cell.description = "舊的佔位描述。".to_string();
        cell.tags.push("m3_032".to_string());
        cell.tags.push("scene_stub".to_string());
        grid.insert(cell);
        grid.clear_dirty();

        let report = apply_world_skeleton(&mut grid, &skeleton).unwrap();
        let refreshed = grid.get(coord).unwrap();

        assert!(report.updated > 0);
        assert_eq!(refreshed.description, polished.description);
        assert!(refreshed.tags.iter().any(|tag| tag == "scene_polished"));
        assert!(!refreshed.tags.iter().any(|tag| tag == "scene_stub"));
    }

    #[test]
    fn builtin_world_skeleton_exits_target_known_cells() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let coords: HashSet<SquareCoord> = skeleton
            .cells
            .iter()
            .map(|cell| SquareCoord::new(cell.x, cell.y))
            .collect();

        for cell in &skeleton.cells {
            for exit in &cell.exits {
                let target = SquareCoord::new(exit.to[0], exit.to[1]);
                assert!(coords.contains(&target), "{} exit to {}", cell.name, target);
            }
        }
    }

    #[test]
    fn builtin_world_skeleton_has_first_outer_road_chain() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let mut grid = SquareGrid::new();
        apply_world_skeleton(&mut grid, &skeleton).unwrap();

        let route = [
            SquareCoord::new(0, 0),
            SquareCoord::new(0, 1),
            SquareCoord::new(0, 2),
            SquareCoord::new(0, 3),
            SquareCoord::new(0, 4),
            SquareCoord::new(0, 5),
        ];
        for pair in route.windows(2) {
            assert!(
                grid.can_walk(pair[0], pair[1]),
                "{} -> {}",
                pair[0],
                pair[1]
            );
            assert!(
                grid.can_walk(pair[1], pair[0]),
                "{} -> {}",
                pair[1],
                pair[0]
            );
        }

        assert_eq!(grid.get(SquareCoord::new(0, 2)).unwrap().name, "北堤泥路");
        assert_eq!(grid.get(SquareCoord::new(0, 3)).unwrap().name, "田埂水口");
        assert_eq!(grid.get(SquareCoord::new(0, 5)).unwrap().name, "田區路口");
    }

    #[test]
    fn builtin_world_skeleton_has_m3_034_field_ring() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let mut grid = SquareGrid::new();
        apply_world_skeleton(&mut grid, &skeleton).unwrap();

        let route = [
            SquareCoord::new(0, 5),
            SquareCoord::new(0, 6),
            SquareCoord::new(-1, 6),
            SquareCoord::new(-1, 5),
            SquareCoord::new(0, 5),
            SquareCoord::new(1, 5),
            SquareCoord::new(2, 5),
        ];
        for pair in route.windows(2) {
            assert!(
                grid.can_walk(pair[0], pair[1]),
                "{} -> {}",
                pair[0],
                pair[1]
            );
            assert!(
                grid.can_walk(pair[1], pair[0]),
                "{} -> {}",
                pair[1],
                pair[0]
            );
        }

        assert_eq!(grid.get(SquareCoord::new(0, 6)).unwrap().name, "農舍前坪");
        assert_eq!(grid.get(SquareCoord::new(-1, 6)).unwrap().name, "穀場小棚");
        assert_eq!(grid.get(SquareCoord::new(2, 5)).unwrap().name, "秧苗田邊");
    }

    #[test]
    fn builtin_world_skeleton_has_m3_030_town_subdistrict() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let mut grid = SquareGrid::new();
        apply_world_skeleton(&mut grid, &skeleton).unwrap();

        let origin = SquareCoord::new(0, 0);
        let town_gate = SquareCoord::new(3, 0);
        let route = [
            SquareCoord::new(0, 0),
            SquareCoord::new(0, 1),
            SquareCoord::new(2, 0),
            town_gate,
            SquareCoord::new(4, 0),
            SquareCoord::new(5, 0),
            SquareCoord::new(5, 1),
        ];
        for pair in route.windows(2) {
            assert!(
                grid.can_walk(pair[0], pair[1]),
                "{} -> {}",
                pair[0],
                pair[1]
            );
            assert!(
                grid.can_walk(pair[1], pair[0]),
                "{} -> {}",
                pair[1],
                pair[0]
            );
        }

        assert_eq!(grid.get(town_gate).unwrap().name, "中鎮鎮口");
        let market_valley_road = grid.get(SquareCoord::new(2, 0)).unwrap();
        assert!(
            market_valley_road
                .explicit_exits
                .iter()
                .any(|exit| exit.target == SquareCoord::new(0, 1) && exit.dir == Direction::North)
        );
        assert_eq!(grid.get(SquareCoord::new(4, 1)).unwrap().name, "中鎮井邊巷");
        assert_eq!(
            grid.get(SquareCoord::new(4, -1)).unwrap().name,
            "中鎮倉場門口"
        );
        assert!(
            grid.find_path(origin, SquareCoord::new(4, -1)).is_some(),
            "北岸 should reach 中鎮倉場門口"
        );
    }

    #[test]
    fn builtin_world_skeleton_has_m3_044_southern_city_subdistrict() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let mut grid = SquareGrid::new();
        apply_world_skeleton(&mut grid, &skeleton).unwrap();

        let origin = SquareCoord::new(0, 0);
        let outer_road = SquareCoord::new(-1, -5);
        let crossroads = SquareCoord::new(-1, -10);
        let route = [
            SquareCoord::new(0, 0),
            SquareCoord::new(0, -1),
            SquareCoord::new(0, -2),
            SquareCoord::new(0, -3),
            SquareCoord::new(0, -4),
            SquareCoord::new(0, -5),
            outer_road,
            SquareCoord::new(-1, -6),
            SquareCoord::new(-1, -7),
            SquareCoord::new(-1, -8),
            SquareCoord::new(-1, -9),
            crossroads,
        ];
        for pair in route.windows(2) {
            assert!(
                grid.can_walk(pair[0], pair[1]),
                "{} -> {}",
                pair[0],
                pair[1]
            );
            assert!(
                grid.can_walk(pair[1], pair[0]),
                "{} -> {}",
                pair[1],
                pair[0]
            );
        }

        assert_eq!(grid.get(outer_road).unwrap().name, "南城外驛道");
        assert_eq!(grid.get(SquareCoord::new(-1, -6)).unwrap().name, "南城甕門");
        assert_eq!(
            grid.get(SquareCoord::new(-1, -8)).unwrap().name,
            "中軸大街南段"
        );
        assert_eq!(grid.get(crossroads).unwrap().name, "大十字通衢");
        assert!(
            grid.find_path(origin, crossroads).is_some(),
            "北岸 should reach 大十字通衢"
        );
    }

    #[test]
    fn builtin_world_skeleton_has_m3_020_western_city_state_slice() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let mut grid = SquareGrid::new();
        apply_world_skeleton(&mut grid, &skeleton).unwrap();

        let origin = SquareCoord::new(0, 0);
        let outer_slope = SquareCoord::new(-8, 0);
        let council_steps = SquareCoord::new(-10, -3);
        let route = [
            origin,
            SquareCoord::new(-1, 0),
            SquareCoord::new(-2, 0),
            SquareCoord::new(-3, 0),
            SquareCoord::new(-4, 0),
            SquareCoord::new(-5, 0),
            SquareCoord::new(-6, 0),
            SquareCoord::new(-7, 0),
            outer_slope,
            SquareCoord::new(-9, 0),
            SquareCoord::new(-10, 0),
            SquareCoord::new(-10, -1),
            SquareCoord::new(-10, -2),
            council_steps,
        ];
        for pair in route.windows(2) {
            assert!(
                grid.can_walk(pair[0], pair[1]),
                "{} -> {}",
                pair[0],
                pair[1]
            );
            assert!(
                grid.can_walk(pair[1], pair[0]),
                "{} -> {}",
                pair[1],
                pair[0]
            );
        }

        assert_eq!(grid.get(outer_slope).unwrap().name, "城邦外坡道");
        assert_eq!(grid.get(SquareCoord::new(-9, 0)).unwrap().name, "石壘門");
        assert_eq!(
            grid.get(SquareCoord::new(-10, -1)).unwrap().name,
            "旗石廣場"
        );
        assert_eq!(grid.get(council_steps).unwrap().name, "議棚前階");
        assert_eq!(
            grid.get(SquareCoord::new(-11, -1)).unwrap().name,
            "鑄幣巷口"
        );
        assert_eq!(grid.get(SquareCoord::new(-9, -1)).unwrap().name, "傭工棚邊");
        assert!(
            grid.find_path(origin, council_steps).is_some(),
            "北岸 should reach 議棚前階"
        );
    }

    #[test]
    fn builtin_world_skeleton_updates_existing_exit_direction() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let mut grid = SquareGrid::new();
        let mut cell = GridCell::new(SquareCoord::new(2, 0), Terrain::Road, "短市外谷地路");
        cell.explicit_exits.push(ExplicitExit {
            dir: Direction::West,
            target: SquareCoord::new(0, 1),
        });
        grid.insert(cell);
        grid.clear_dirty();

        let report = apply_world_skeleton(&mut grid, &skeleton).unwrap();
        let updated = grid.get(SquareCoord::new(2, 0)).unwrap();

        assert!(report.updated > 0);
        assert!(
            updated
                .explicit_exits
                .iter()
                .any(|exit| exit.target == SquareCoord::new(0, 1) && exit.dir == Direction::North)
        );
    }

    #[test]
    fn builtin_world_skeleton_reaches_representative_outer_regions() {
        let skeleton = load_builtin_world_skeleton().unwrap();
        let mut grid = SquareGrid::new();
        apply_world_skeleton(&mut grid, &skeleton).unwrap();

        let origin = SquareCoord::new(0, 0);
        let endpoints = [
            (SquareCoord::new(2, 8), "北灣守望石階"),
            (SquareCoord::new(-7, 7), "巡邏院"),
            (SquareCoord::new(-5, -4), "車隊場"),
            (SquareCoord::new(-10, -3), "議棚前階"),
            (SquareCoord::new(11, -6), "匯兌小屋"),
        ];

        for (coord, name) in endpoints {
            let cell = grid.get(coord).unwrap();
            assert_eq!(cell.name, name);
            assert!(
                grid.find_path(origin, coord).is_some(),
                "北岸 should reach {name} at {coord}"
            );
        }
    }
}

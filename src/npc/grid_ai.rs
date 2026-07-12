//! 實作 `src/npc/mod.rs` 中定義的 GridNpcManager 核心邏輯 (方格網 AI)
//! 包含 pathfinding, intent 決策, 以及與資料庫的同步。

use crate::db::{add_to_inventory, get_entity, get_schedule_target};
use crate::grid::{SquareCoord, SquareGrid, grid_room_id_from_coord};
use crate::npc::IntentType;
use crate::store::get_store;
use crate::world::materials;
use std::collections::{HashMap, HashSet};

/// 與常數 `SurvivalLineMg` 一致：低於此鎂才可能在未觀測時補鎂。
pub const SURVIVAL_LINE_MG: i32 = 50;

#[derive(Debug, Clone, Default)]
pub struct NpcState {
    pub path: Vec<SquareCoord>,
    pub dwell_ticks: i32,
    pub last_intent: Option<IntentType>,
}

pub struct GridNpcManager {
    /// 緩存 NPC 的運行時狀態 (路徑、停留時間等)
    pub states: HashMap<String, NpcState>,
}

impl Default for GridNpcManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GridNpcManager {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// 每一跳 (tick) 驅動方格世界所有 NPC：決策 -> 移動 -> 採集
    pub fn tick(&mut self, grid: &mut SquareGrid, now_unix: i64, hour: i32) -> Vec<GridMoveEvent> {
        let mut events = Vec::new();
        let mut harvest_queue = Vec::new();

        let Some(arc) = get_store() else {
            return events;
        };
        let npc_coords: HashMap<String, SquareCoord>;
        {
            let s = arc.read();
            npc_coords = s
                .get_npc_ids_with_grid()
                .into_iter()
                .filter_map(|id| {
                    let e = s.get_entity(&id)?;
                    let x = e.grid_x?;
                    let y = e.grid_y?;
                    Some((id, SquareCoord { x, y }))
                })
                .collect();
        }

        let occupancy: HashSet<SquareCoord> = npc_coords.values().cloned().collect();

        for (entity_id, pos) in &npc_coords {
            let state = self.states.entry(entity_id.clone()).or_default();

            if state.dwell_ticks > 0 {
                state.dwell_ticks -= 1;
                continue;
            }

            if !state.path.is_empty() {
                let next_pos = state.path[0];
                if !occupancy.contains(&next_pos) && grid.can_walk(*pos, next_pos) {
                    state.path.remove(0);
                    let description = format!(
                        "從 ({},{}) 往 {} ({},{}) 移動了一步",
                        pos.x,
                        pos.y,
                        grid.get(next_pos)
                            .map(|c| c.name.clone())
                            .unwrap_or_else(|| "街道".into()),
                        next_pos.x,
                        next_pos.y
                    );
                    events.push(GridMoveEvent {
                        entity_id: entity_id.clone(),
                        from: *pos,
                        to: next_pos,
                        intent: state.last_intent.unwrap_or(IntentType::Wander),
                        description,
                    });

                    let mut s = arc.write();
                    let _ = s.set_entity_grid(entity_id, next_pos.x, next_pos.y);
                } else {
                    state.path.clear();
                }
                continue;
            }

            let intent = decide_grid(entity_id, *pos, grid, state.last_intent, hour);
            state.last_intent = Some(intent);

            match intent {
                IntentType::Work | IntentType::Idle => {
                    if let Some(target) = get_schedule_target(entity_id, hour) {
                        let parts: Vec<&str> = target.room.split(',').collect();
                        if parts.len() == 2
                            && let (Ok(tx), Ok(ty)) =
                                (parts[0].parse::<i32>(), parts[1].parse::<i32>())
                        {
                            let target_pos = SquareCoord { x: tx, y: ty };
                            if target_pos != *pos {
                                if let Some(path) = grid.find_path(*pos, target_pos) {
                                    state.path = path.into_iter().skip(1).collect();
                                }
                            } else {
                                state.dwell_ticks = 5;
                            }
                        }
                    }
                }
                IntentType::Gather => {
                    if let Some(target) = find_nearest_resource(grid, *pos, 6) {
                        if target == *pos {
                            harvest_queue.push((entity_id.clone(), *pos));
                            state.dwell_ticks = 2;
                        } else if let Some(path) = grid.find_path(*pos, target) {
                            state.path = path.into_iter().skip(1).collect();
                        }
                    }
                }
                IntentType::Wander => {
                    if let Some(target) = pick_random_walkable(grid, *pos, 3)
                        && let Some(path) = grid.find_path(*pos, target)
                    {
                        state.path = path.into_iter().skip(1).collect();
                    }
                }
                _ => {
                    state.dwell_ticks = 3;
                }
            }
        }

        for (entity_id, coord) in &harvest_queue {
            harvest_from_grid(grid, now_unix, entity_id, *coord);
        }

        tick_regrowth(grid, &occupancy);
        events
    }
}

#[derive(Debug, Clone)]
pub struct GridMoveEvent {
    pub entity_id: String,
    pub from: SquareCoord,
    pub to: SquareCoord,
    pub intent: IntentType,
    pub description: String,
}

fn decide_grid(
    entity_id: &str,
    pos: SquareCoord,
    grid: &SquareGrid,
    _prev: Option<IntentType>,
    hour: i32,
) -> IntentType {
    if let Some(target) = get_schedule_target(entity_id, hour) {
        let parts: Vec<&str> = target.room.split(',').collect();
        if parts.len() == 2
            && let (Ok(tx), Ok(ty)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>())
            && (tx != pos.x || ty != pos.y)
        {
            return if target.is_work {
                IntentType::Work
            } else {
                IntentType::Idle
            };
        }
    }

    let state = match build_decision_state(entity_id) {
        Ok(s) => s,
        Err(_) => return IntentType::Wander,
    };

    let survival = if state.magnesium >= SURVIVAL_LINE_MG {
        0.0
    } else {
        1.0 - (f64::from(state.magnesium) / f64::from(SURVIVAL_LINE_MG))
    };

    let has_resource_nearby = find_nearest_resource(grid, pos, 4).is_some();

    if survival > 0.4 && has_resource_nearby {
        return IntentType::Gather;
    }

    IntentType::Wander
}

struct DecisionState {
    magnesium: i32,
}

fn build_decision_state(entity_id: &str) -> anyhow::Result<DecisionState> {
    let e = get_entity(entity_id)?.ok_or_else(|| anyhow::anyhow!("Entity not found"))?;
    Ok(DecisionState {
        magnesium: e.magnesium,
    })
}

fn find_nearest_resource(grid: &SquareGrid, pos: SquareCoord, range: i32) -> Option<SquareCoord> {
    let mut best_dist = i32::MAX;
    let mut best_pos = None;

    for dy in -range..=range {
        for dx in -range..=range {
            let target = SquareCoord {
                x: pos.x + dx,
                y: pos.y + dy,
            };
            if let Some(cell) = grid.get(target) {
                let k = cell.terrain.kind_str();
                if k == "farm_field" || k == "forest" {
                    let dist = dx.abs() + dy.abs();
                    if dist < best_dist {
                        best_dist = dist;
                        best_pos = Some(target);
                    }
                }
            }
        }
    }
    best_pos
}

fn pick_random_walkable(grid: &SquareGrid, pos: SquareCoord, range: i32) -> Option<SquareCoord> {
    use rand::Rng;
    let mut rng = rand::rng();
    let dx = rng.random_range(-range..=range);
    let dy = rng.random_range(-range..=range);
    let target = SquareCoord {
        x: pos.x + dx,
        y: pos.y + dy,
    };
    if grid
        .get(target)
        .map(|c| c.terrain.walkable())
        .unwrap_or(false)
    {
        Some(target)
    } else {
        None
    }
}

fn tick_regrowth(_grid: &mut SquareGrid, _occupancy: &HashSet<SquareCoord>) {}

fn harvest_from_grid(grid: &mut SquareGrid, now: i64, entity_id: &str, pos: SquareCoord) {
    if grid.get(pos).is_none() {
        return;
    }
    let room_id = grid_room_id_from_coord(pos.x, pos.y);
    if let Some(gathered) = materials::try_gather_material_for_room_preserving(&room_id, now, 1) {
        let _ = add_to_inventory(entity_id, &gathered.item_id, 1);
    }
}

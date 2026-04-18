//! 方格 NPC 行為引擎 — 讓 NPC 在正方格上移動、採集、漫遊。
//!
//! 此模組是 `hex_ai` 的機械性分支（fork）：決策邏輯、採集機制、
//! 資源回復算法完全相同，僅座標系從六角格（`HexCoord`）換成正方格（`SquareCoord`）。
//!
//! 與 room 系統完全平行，不碰 RoomGraph。每個方格 NPC 維護自己的
//! 路徑與意圖狀態，由 simulation_loop 每個 travel tick 呼叫一次。

use std::collections::HashMap;

use rand::Rng;

use crate::db::{add_magnesium, add_to_inventory, get_entity, get_npc_ids_with_grid, log_npc_event, set_entity_grid, EVT_GATHER};
use crate::grid::{SquareCoord, SquareGrid};
use crate::npc::decision::{build_decision_state, IntentType, SURVIVAL_LINE_MG};
use crate::npc::narrative;

/// 單個 NPC 的方格行為狀態
#[derive(Debug, Clone)]
struct GridNpcState {
    /// 當前意圖
    intent: IntentType,
    /// 剩餘路徑（不含當前位置）
    path: Vec<SquareCoord>,
    /// 抵達目的地後停留的 tick 數（模擬採集/休息時間）
    dwell_ticks: i32,
    /// 上次意圖（用於慣性）
    prev_intent: Option<IntentType>,
}

impl Default for GridNpcState {
    fn default() -> Self {
        Self {
            intent: IntentType::Wander,
            path: Vec::new(),
            dwell_ticks: 0,
            prev_intent: None,
        }
    }
}

/// 方格 NPC 行為管理器
#[derive(Default)]
pub struct GridNpcManager {
    states: HashMap<String, GridNpcState>,
}

impl GridNpcManager {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// 每個 travel tick 呼叫一次。回傳有移動的 NPC 列表（供觀景窗等外部系統用）。
    pub fn tick(&mut self, grid: &mut SquareGrid, now_unix: i64) -> Vec<GridMoveEvent> {
        let mut events = Vec::new();

        let npc_ids = get_npc_ids_with_grid();

        // 找出所有有方格座標的 NPC
        let mut grid_npcs: Vec<(String, SquareCoord)> = Vec::new();
        for id in &npc_ids {
            if let Ok(Some(ch)) = get_entity(id)
                && let (Some(x), Some(y)) = (ch.grid_x, ch.grid_y)
            {
                grid_npcs.push((id.clone(), SquareCoord::new(x, y)));
            }
        }

        if grid_npcs.is_empty() {
            return events;
        }

        // 建立「格子上有幾人」索引，供 density 計算
        let mut occupancy: HashMap<SquareCoord, u32> = HashMap::new();
        for (_, coord) in &grid_npcs {
            *occupancy.entry(*coord).or_insert(0) += 1;
        }

        // 收集需要扣減的資源（entity_id, coord）
        let mut harvest_queue: Vec<(String, SquareCoord)> = Vec::new();

        for (entity_id, pos) in &grid_npcs {
            let state = self.states.entry(entity_id.clone()).or_default();

            // 停留中（採集/休息）
            if state.dwell_ticks > 0 {
                state.dwell_ticks -= 1;
                if state.dwell_ticks == 0 {
                    // 停留結束，執行採集
                    if state.intent == IntentType::Gather {
                        harvest_queue.push((entity_id.clone(), *pos));
                    }
                }
                continue;
            }

            // 有路徑 → 走一步
            if let Some(next) = state.path.first().copied() {
                state.path.remove(0);
                if grid.contains(next) {
                    let _ = set_entity_grid(entity_id, next.x, next.y);
                    // 取 NPC 名稱供敘事句使用（display_title 為空則退回 id）
                    let npc_name = get_entity(entity_id)
                        .ok()
                        .flatten()
                        .map(|e| if e.display_title.is_empty() { entity_id.to_string() } else { e.display_title })
                        .unwrap_or_else(|| entity_id.to_string());
                    let tick_seed = now_unix as u64;
                    let desc = match state.intent {
                        IntentType::Gather => narrative::gather(entity_id, &npc_name, tick_seed),
                        IntentType::Idle   => narrative::idle(entity_id, &npc_name, tick_seed),
                        _                  => narrative::wander(entity_id, &npc_name, tick_seed),
                    };
                    events.push(GridMoveEvent {
                        entity_id: entity_id.clone(),
                        from: *pos,
                        to: next,
                        intent: state.intent,
                        description: desc,
                    });
                    // 到達目的地：採集需停留
                    if state.path.is_empty() && state.intent == IntentType::Gather {
                        state.dwell_ticks = 2;
                    }
                } else {
                    state.path.clear();
                }
                continue;
            }

            // 無路徑 → 做決策
            let intent = decide_grid(entity_id, *pos, grid, state.prev_intent);
            state.prev_intent = Some(state.intent);
            state.intent = intent;

            match intent {
                IntentType::Gather => {
                    if let Some(target) = find_nearest_resource(grid, *pos, 6) {
                        if target == *pos {
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
                IntentType::Idle => {
                    state.dwell_ticks = 3;
                }
                _ => {
                    if let Some(target) = pick_random_walkable(grid, *pos, 2)
                        && let Some(path) = grid.find_path(*pos, target)
                    {
                        state.path = path.into_iter().skip(1).collect();
                    }
                }
            }
        }

        // 執行採集：扣 HexObject.quantity，加 inventory + 鎂
        for (entity_id, coord) in &harvest_queue {
            harvest_from_grid(grid, now_unix, entity_id, *coord);
        }

        // 資源回復（density_multiplier）
        tick_regrowth(grid, &occupancy);

        events
    }
}

/// 方格 NPC 移動事件
#[derive(Debug, Clone)]
pub struct GridMoveEvent {
    pub entity_id: String,
    pub from: SquareCoord,
    pub to: SquareCoord,
    pub intent: IntentType,
    /// 行為敘事句（已填入 NPC 名稱，可直接推給前端）
    pub description: String,
}

// ── 決策 ──────────────────────────────────────────────────

/// 方格世界簡化決策：需求驅動，不是腳本驅動。
/// 與 `decide_hex` 邏輯完全相同，僅座標型別換為 `SquareCoord`。
fn decide_grid(entity_id: &str, pos: SquareCoord, grid: &SquareGrid, prev: Option<IntentType>) -> IntentType {
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

    let mut rng = rand::rng();

    // 慣性：正在做的事傾向繼續做
    if let Some(prev_intent) = prev {
        match prev_intent {
            IntentType::Gather if has_resource_nearby => {
                if rng.random_range(0.0..1.0) < 0.6 {
                    return IntentType::Gather;
                }
            }
            IntentType::Wander => {
                if rng.random_range(0.0..1.0) < 0.3 {
                    return IntentType::Wander;
                }
            }
            _ => {}
        }
    }

    // 飢餓 → 找資源
    if survival > 0.3 && has_resource_nearby {
        return IntentType::Gather;
    }

    // 有東西可採就採（即使不餓，人會順手撿）
    if has_resource_nearby && rng.random_range(0.0..1.0) < 0.4 {
        return IntentType::Gather;
    }

    // 偶爾發呆
    if rng.random_range(0.0..1.0) < 0.15 {
        return IntentType::Idle;
    }

    IntentType::Wander
}

// ── 尋找 ──────────────────────────────────────────────────

/// 找最近的有資源（quantity > 0）的格子，BFS 限定 max_dist 步。
/// GridCell 的 `.coord` 型別為 `SquareCoord`，與 HexCell 的 `.coord: HexCoord` 對稱。
fn find_nearest_resource(grid: &SquareGrid, origin: SquareCoord, max_dist: u32) -> Option<SquareCoord> {
    use std::collections::{HashSet, VecDeque};

    // 先檢查腳下
    if let Some(cell) = grid.get(origin)
        && cell.objects.iter().any(|o| o.quantity > 0)
    {
        return Some(origin);
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(origin);
    queue.push_back((origin, 0u32));

    while let Some((coord, dist)) = queue.pop_front() {
        if dist >= max_dist {
            continue;
        }
        for nb_cell in grid.neighbors(coord) {
            let nb = nb_cell.coord;
            if visited.contains(&nb) {
                continue;
            }
            visited.insert(nb);
            if !nb_cell.terrain.walkable() {
                continue;
            }
            if nb_cell.objects.iter().any(|o| o.quantity > 0) {
                return Some(nb);
            }
            queue.push_back((nb, dist + 1));
        }
    }
    None
}

/// 隨機選一個 max_dist 步內的可行走格子。
fn pick_random_walkable(grid: &SquareGrid, origin: SquareCoord, max_dist: u32) -> Option<SquareCoord> {
    use std::collections::{HashSet, VecDeque};

    let mut candidates = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(origin);
    queue.push_back((origin, 0u32));

    while let Some((coord, dist)) = queue.pop_front() {
        if dist >= max_dist {
            continue;
        }
        for nb_cell in grid.neighbors(coord) {
            let nb = nb_cell.coord;
            if visited.contains(&nb) {
                continue;
            }
            visited.insert(nb);
            if !nb_cell.terrain.walkable() {
                continue;
            }
            candidates.push(nb);
            queue.push_back((nb, dist + 1));
        }
    }

    if candidates.is_empty() {
        return None;
    }
    let mut rng = rand::rng();
    Some(candidates[rng.random_range(0..candidates.len())])
}

// ── 採集（真實扣量）──────────────────────────────────────

/// 從格子上的第一個有量資源採集 1 單位，扣 HexObject.quantity。
fn harvest_from_grid(grid: &mut SquareGrid, now_unix: i64, entity_id: &str, coord: SquareCoord) {
    let Some(cell) = grid.get_mut(coord) else { return };
    let Some(obj) = cell.objects.iter_mut().find(|o| o.quantity > 0) else { return };

    obj.quantity = obj.quantity.saturating_sub(1);
    let item_name = obj.name.clone();
    grid.mark_dirty();

    let _ = add_to_inventory(entity_id, &item_name, 1);
    let _ = add_magnesium(entity_id, 3);
    let payload = format!("在 ({},{}) 採集了{}", coord.x, coord.y, item_name);
    log_npc_event(now_unix, entity_id, EVT_GATHER, &payload);
}

// ── 資源回復 ──────────────────────────────────────────────

/// density_multiplier：0人→3.0、1人→1.0、2人→0.6、3+人→0.3
/// Token 物理的 110V 版本：有人→恢復慢，沒人→恢復快。
fn density_multiplier(count: u32) -> f64 {
    match count {
        0 => 3.0,
        1 => 1.0,
        2 => 0.6,
        _ => 0.3,
    }
}

/// 每 tick 資源回復：regrowth_per_tick × density_multiplier。
fn tick_regrowth(grid: &mut SquareGrid, occupancy: &HashMap<SquareCoord, u32>) {
    let coords: Vec<SquareCoord> = grid.coords().copied().collect();
    for coord in coords {
        let pop = occupancy.get(&coord).copied().unwrap_or(0);
        let mult = density_multiplier(pop);

        let Some(cell) = grid.get_mut(coord) else { continue };
        let mut changed = false;
        for obj in &mut cell.objects {
            if obj.regrowth_per_tick == 0 || obj.quantity >= obj.max_quantity {
                continue;
            }
            let growth = (f64::from(obj.regrowth_per_tick) * mult).round() as u32;
            if growth > 0 {
                obj.quantity = (obj.quantity + growth).min(obj.max_quantity);
                changed = true;
            }
        }
        if changed { grid.mark_dirty(); }
    }
}

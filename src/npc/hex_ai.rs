//! Hex NPC 行為引擎 — 讓 NPC 在六角格上移動、漫遊。
//!
//! 注意：HexCell.objects 為 RoomObject（無 quantity/regrowth），
//! 採集與資源回復邏輯已移至 grid_ai（正方格）。此模組僅保留移動與漫遊。

use std::collections::HashMap;

use rand::Rng;

use crate::db::{get_entity, get_npc_ids_with_room, set_entity_hex};
use crate::hex::{HexCoord, HexGrid};
use crate::npc::decision::IntentType;

/// 單個 NPC 的 hex 行為狀態
#[derive(Debug, Clone)]
struct HexNpcState {
    /// 當前意圖
    intent: IntentType,
    /// 剩餘路徑（不含當前位置）
    path: Vec<HexCoord>,
    /// 抵達目的地後停留的 tick 數（模擬休息時間）
    dwell_ticks: i32,
}

impl Default for HexNpcState {
    fn default() -> Self {
        Self {
            intent: IntentType::Wander,
            path: Vec::new(),
            dwell_ticks: 0,
        }
    }
}

/// Hex NPC 行為管理器（觀景窗用，僅移動漫遊，無採集）
#[derive(Default)]
pub struct HexNpcManager {
    states: HashMap<String, HexNpcState>,
}

impl HexNpcManager {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// 每個 travel tick 呼叫一次。回傳有移動的 NPC 列表。
    pub fn tick(&mut self, grid: &mut HexGrid, _now_unix: i64) -> Vec<HexMoveEvent> {
        let mut events = Vec::new();

        let npc_ids = get_npc_ids_with_room();
        let mut hex_npcs: Vec<(String, HexCoord)> = Vec::new();
        for id in &npc_ids {
            if let Ok(Some(ch)) = get_entity(id)
                && let (Some(q), Some(r)) = (ch.hex_q, ch.hex_r)
            {
                hex_npcs.push((id.clone(), HexCoord::new(q, r)));
            }
        }

        if hex_npcs.is_empty() {
            return events;
        }

        for (entity_id, pos) in &hex_npcs {
            let state = self.states.entry(entity_id.clone()).or_default();

            // 停留中
            if state.dwell_ticks > 0 {
                state.dwell_ticks -= 1;
                continue;
            }

            // 有路徑 → 走一步
            if let Some(next) = state.path.first().copied() {
                state.path.remove(0);
                if grid.contains(next) {
                    let _ = set_entity_hex(entity_id, next.q, next.r);
                    events.push(HexMoveEvent {
                        entity_id: entity_id.clone(),
                        from: *pos,
                        to: next,
                        intent: state.intent,
                    });
                } else {
                    state.path.clear();
                }
                continue;
            }

            // 無路徑 → 漫遊或發呆
            let mut rng = rand::rng();
            if rng.random_range(0.0..1.0) < 0.15 {
                state.intent = IntentType::Idle;
                state.dwell_ticks = 3;
            } else {
                state.intent = IntentType::Wander;
                if let Some(target) = pick_random_walkable(grid, *pos, 3)
                    && let Some(path) = grid.find_path(*pos, target)
                {
                    state.path = path.into_iter().skip(1).collect();
                }
            }
        }

        events
    }
}

/// NPC 移動事件
#[derive(Debug, Clone)]
pub struct HexMoveEvent {
    pub entity_id: String,
    pub from: HexCoord,
    pub to: HexCoord,
    pub intent: IntentType,
}

/// 隨機選一個 max_dist 步內的可行走格子。
fn pick_random_walkable(grid: &HexGrid, origin: HexCoord, max_dist: u32) -> Option<HexCoord> {
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

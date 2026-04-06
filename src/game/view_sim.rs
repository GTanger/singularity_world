// 視野內即時模擬，對齊既有 game/view_sim。

use std::collections::HashSet;

use crate::db;
use crate::entity::Character;

use super::observe::{now_unix, Observer};
use super::zone::{in_view, VIEW_RADIUS};

/// 觀測者格點座標（對齊既有 `Pos`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

/// 在任一觀測者視野內的實體 ID（去重，對齊既有 `InViewEntityIDs`）。
#[must_use]
pub fn in_view_entity_ids(observers: &[Pos], entities: &[Character]) -> Vec<String> {
    let mut seen = HashSet::new();
    for e in entities {
        for o in observers {
            if in_view(o.x, o.y, e.x, e.y) {
                seen.insert(e.id.clone());
                break;
            }
        }
    }
    seen.into_iter().collect()
}

/// 單一 NPC 一 tick 模擬；第一版 no-op（對齊既有 `SimulateOneTick`）。
pub fn simulate_one_tick(_entity_id: &str) -> anyhow::Result<()> {
    Ok(())
}

/// 僅對視野內 NPC 跑觀測與 tick（對齊既有 `RunViewSimulation`）。
pub fn run_view_simulation(
    get_observer_positions: impl Fn() -> Vec<Pos>,
    obs: Option<&dyn Observer>,
) {
    let observers = get_observer_positions();
    if observers.is_empty() {
        return;
    }
    let mut x_min = observers[0].x;
    let mut x_max = observers[0].x;
    let mut y_min = observers[0].y;
    let mut y_max = observers[0].y;
    for p in observers.iter().skip(1) {
        x_min = x_min.min(p.x);
        x_max = x_max.max(p.x);
        y_min = y_min.min(p.y);
        y_max = y_max.max(p.y);
    }
    x_min -= VIEW_RADIUS;
    x_max += VIEW_RADIUS;
    y_min -= VIEW_RADIUS;
    y_max += VIEW_RADIUS;

    let Ok(npcs) = db::get_entities_in_box(x_min, x_max, y_min, y_max, "npc") else {
        return;
    };
    if npcs.is_empty() {
        return;
    }
    let in_view_ids = in_view_entity_ids(&observers, &npcs);
    let now = now_unix();
    for id in in_view_ids {
        if let Some(o) = obs {
            o.on_observe(&id, "", now);
        }
        let _ = simulate_one_tick(&id);
    }
}

// 每 tick 推進移動中實體的位置（對齊既有 `game/movement_tick`）。

use std::path::Path;

use crate::{db, entity::{Character, WalkOrRun}, event::{self, types as evt}};

use super::chunk_view::get_chunk_and_entities_in_view;

/// 單一實體本 tick 移動後的座標，供 server 廣播。
pub struct MovedResult {
    pub id: String,
    pub x: i32,
    pub y: i32,
}

/// 對所有 `move_state == "moving"` 的實體推進座標（walk=1 格, run=2 格），回傳有變動的 `(id, x, y)`（對齊既有 `AdvanceMovement`）。
pub fn advance_movement(maps_path: &Path, now: i64) -> anyhow::Result<Vec<MovedResult>> {
    let list = db::get_moving_entities()?;
    if list.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for e in &list {
        let (Some(tx), Some(ty)) = (e.target_x, e.target_y) else {
            continue;
        };
        if e.x == tx && e.y == ty {
            let _ = db::update_position(&e.id, tx, ty);
            let _ = event::append(now, &e.id, evt::ARRIVED, "");
            out.push(MovedResult { id: e.id.clone(), x: tx, y: ty });
            continue;
        }
        let steps = if e.walk_or_run == WalkOrRun::Run { 2 } else { 1 };
        let (mut nx, mut ny) = (e.x, e.y);
        for _ in 0..steps {
            let Ok(view) = get_chunk_and_entities_in_view(nx, ny, maps_path) else {
                break;
            };
            let dx = sign(tx - nx);
            let dy = sign(ty - ny);
            if dx == 0 && dy == 0 {
                break;
            }
            let (next_x, next_y) = (nx + dx, ny + dy);
            if !view.can_move_to_world(next_x, next_y) {
                let _ = event::append(now, &e.id, evt::BLOCKED, "");
                let _ = db::update_position(&e.id, nx, ny);
                break;
            }
            nx = next_x;
            ny = next_y;
            if nx == tx && ny == ty {
                break;
            }
        }
        if nx != e.x || ny != e.y {
            let _ = db::update_position(&e.id, nx, ny);
            out.push(MovedResult { id: e.id.clone(), x: nx, y: ny });
        }
        if nx == tx && ny == ty {
            let _ = db::update_position(&e.id, tx, ty);
            let _ = event::append(now, &e.id, evt::ARRIVED, "");
            if let Some(other) = entity_at(tx, ty, &e.id) {
                let _ = event::append(now, &e.id, evt::CONTACT, &other.id);
            }
        }
    }
    Ok(out)
}

/// 回傳在 `(x, y)` 的實體（排除 `exclude_id`），用於接觸判定（對齊既有 `EntityAt`）。
pub fn entity_at(x: i32, y: i32, exclude_id: &str) -> Option<Character> {
    let list = db::get_entities_in_box(x, x, y, y, "").ok()?;
    list.into_iter().find(|e| e.id != exclude_id)
}

fn sign(n: i32) -> i32 {
    if n < 0 { -1 } else if n > 0 { 1 } else { 0 }
}

// 觀測觸發與延遲坍縮回推，對齊既有 game/observe。

use crate::db;
use crate::entity::{Character, EntityKind};
use crate::event::{self, types};

/// 觀測發生時由視野／房間邏輯呼叫（對齊既有 `Observer`）。
pub trait Observer {
    fn on_observe(&self, entity_id: &str, observer_id: &str, at: i64);
}

/// 寫入 `observed` 事件並更新 `last_observed_at`（對齊既有 `Observed`）。
#[derive(Debug, Default, Clone, Copy)]
pub struct Observed;

impl Observer for Observed {
    fn on_observe(&self, entity_id: &str, observer_id: &str, at: i64) {
        let _ = event::mark_observed(entity_id, observer_id, at);
    }
}

/// 進入房間時對房內 NPC 標記觀測（對齊既有 `ObserveRoom`）。
pub fn observe_room(room_id: &str, observer_id: &str, at: i64) {
    let Ok(entities) = db::get_entities_in_room(room_id, -1) else {
        return;
    };
    for e in entities {
        if e.kind == EntityKind::Npc {
            let _ = event::mark_observed(&e.id, observer_id, at);
        }
    }
}

/// 進入某六角格時對同格 NPC 標記觀測（Hex 格網權威座標）。
pub fn observe_hex(q: i32, r: i32, observer_id: &str, at: i64) {
    let Ok(entities) = db::get_entities_at_hex(q, r, -1) else {
        return;
    };
    for e in entities {
        if e.kind == EntityKind::Npc {
            let _ = event::mark_observed(&e.id, observer_id, at);
        }
    }
}

/// 自 store 與事件日誌回推 `as_of` 時點狀態（對齊既有 `Collapse`）。
pub fn collapse(entity_id: &str, as_of: i64) -> anyhow::Result<(Character, String)> {
    let Some(mut c) = db::get_entity(entity_id)? else {
        anyhow::bail!("entity not found");
    };
    let mut room_id = db::get_entity_room(entity_id)?;
    let now = now_unix();
    if as_of >= now {
        return Ok((c, room_id));
    }
    let Ok(rows) = event::events_in_range(entity_id, as_of, now) else {
        return Ok((c, room_id));
    };
    for r in rows {
        match r.event_type.as_str() {
            t if t == types::VIT => {
                if let Ok(v) = r.payload.parse::<i32>()
                    && v >= 0
                {
                    c.vit = v;
                }
            }
            t if t == types::MOVE => {
                if !r.payload.is_empty() {
                    room_id = r.payload;
                }
            }
            _ => {}
        }
    }
    Ok((c, room_id))
}

/// 當前 Unix 秒（對齊既有 `NowUnix`）。
#[must_use]
pub fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

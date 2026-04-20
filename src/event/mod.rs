// event 模組 — 事件類型常數、日誌列與寫入／查詢，對齊既有 event/types + event/journal。

use crate::db::ErrNoStore;
use crate::pg;

/// 事件類型常數（對齊決策 004）。
pub mod types {
    pub const OBSERVED: &str = "observed";
    pub const MOVE: &str = "move";
    pub const ARRIVED: &str = "arrived";
    pub const BLOCKED: &str = "blocked";
    pub const CONTACT: &str = "contact";
    pub const COMBAT: &str = "combat";
    pub const TRADE: &str = "trade";
    pub const VIT: &str = "vit";
}

/// 事件日誌一筆，供回推時依時間序套用（對齊既有 `event.EventRow`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRow {
    pub at: i64,
    pub event_type: String,
    pub payload: String,
}

/// 將一筆事件寫入 PG（直走 pool，不經 store lock）。
pub fn append(at: i64, entity_id: &str, event_type: &str, payload: &str) -> anyhow::Result<()> {
    let pool = pg::pool().ok_or(ErrNoStore)?;
    pg::event::append(&pool, at, entity_id, event_type, payload)
}

/// 回傳該實體在 `at` 之前（含）最近一筆該類型事件的 payload。
pub fn last_by_entity(entity_id: &str, event_type: &str, at: i64) -> anyhow::Result<String> {
    let pool = pg::pool().ok_or(ErrNoStore)?;
    Ok(pg::event::last_by_entity(&pool, entity_id, event_type, at))
}

/// 觀測觸發：寫入一筆 `observed` 並更新該實體的 `last_observed_at`。
pub fn mark_observed(entity_id: &str, observer_id: &str, at: i64) -> anyhow::Result<()> {
    append(at, entity_id, types::OBSERVED, observer_id)?;
    crate::db::update_last_observed(entity_id, at)
}

/// 回傳該實體在 `[from_at, to_at]` 區間內的事件，依 `at` 升序（與 store 內順序一致）。
pub fn events_in_range(
    entity_id: &str,
    from_at: i64,
    to_at: i64,
) -> anyhow::Result<Vec<EventRow>> {
    let pool = pg::pool().ok_or(ErrNoStore)?;
    let entries = pg::event::events_in_range(&pool, entity_id, from_at, to_at);
    Ok(entries
        .into_iter()
        .map(|e| EventRow {
            at: e.at,
            event_type: e.event_type,
            payload: e.payload,
        })
        .collect())
}

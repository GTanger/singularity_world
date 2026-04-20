//! NPC 個人事件日誌常數與門面（對齊既有 `db/npc_events`）。

use crate::store;

/// 乞討事件類型。
pub const EVT_BEG: &str = "beg";
/// 採集。
pub const EVT_GATHER: &str = "gather";
/// 獲僱。
pub const EVT_HIRED: &str = "hired";
/// 與人交談（社交到達寫入）。
pub const EVT_TALK: &str = "talk";
/// 交易。
pub const EVT_TRADE: &str = "trade";
/// 死亡（區域廣播用），對齊既有 `EvtDeath`。
pub const EVT_DEATH: &str = "death";

/// 寫入一筆 NPC 事件（對齊既有 `LogNPCEvent`）。
pub fn log_npc_event(at: i64, entity_id: &str, event_type: &str, payload: &str) {
    let Some(arc) = store::get_store() else {
        return;
    };
    // read lock 夠：append_event 只讀 db_pool 做 PG INSERT
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    let _ = s.append_event(at, entity_id, event_type, payload);
}

/// 最近 n 筆事件（對齊既有 `GetRecentEvents`）；無 store 回空。
#[must_use]
pub fn get_recent_events(entity_id: &str, n: usize) -> Vec<store::EventEntry> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read().unwrap_or_else(|e| e.into_inner());
    s.recent_by_entity(entity_id, n)
}

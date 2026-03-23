// event 模組 — 事件類型常數與事件列結構，對齊 Go event/types.go + event/journal.go。
// Phase 1 僅定義型別；Phase 3 補 store 互動邏輯。

/// 事件類型常數。
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

/// 事件日誌一筆，供回推時依時間序套用。
#[derive(Debug, Clone)]
pub struct EventRow {
    pub at: i64,
    pub event_type: String,
    pub payload: String,
}

//! 人物正在做的移動與最近結果；供世界運轉、重啟續行與診斷查詢。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityJourney {
    pub entity_id: String,
    pub reason: String,
    pub destination_place_id: Option<String>,
    pub planned_path: Vec<String>,
    pub consequence: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonWorldState {
    pub entity_id: String,
    pub place_id: String,
    pub current_activity: String,
    pub reason: String,
    pub destination_place_id: Option<String>,
    pub planned_path: Vec<String>,
    pub consequence: String,
    pub updated_at: i64,
}

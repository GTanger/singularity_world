// model 模組 — 與 db、store 共用的資料型別，避免循環依賴。

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 房間內可互動物件（Look/Read/Smell 等），name 須與房間描述中〔〕一致。
/// move_to_room_id 為選填：當 sockets 含 Move 時，執行 Move 會將玩家切換到該房間。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomObject {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub sockets: Vec<String>,
    #[serde(default)]
    pub responses: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub move_to_room_id: String,
}

/// 單一房間節點。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zone: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<RoomObject>,
}

/// 單一出口：方向 → 目標房間。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exit {
    pub direction: String,
    pub to_room_id: String,
    #[serde(default)]
    pub to_room_name: String,
}

//! store 資料型別定義。
//!
//! 所有「空表格」——Store 使用的 DTO 結構與 JSON 載入/寫出輔助結構。
//! 純定義，零行為。外部透過 `store::types::*` 或 `store::*`（經 re-export）取用。

use serde::{Deserialize, Serialize};

use crate::model;

// ══════════════════════════════════════
//  對外 DTO 型別
// ══════════════════════════════════════

/// 場所：id、名稱、room_ids、max_staff。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub room_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub max_staff: i32,
}

/// 指派：誰、什麼職業、哪個場所。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub entity_id: String,
    pub occupation_id: String,
    pub venue_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub assigned_by: String,
}

/// 排班：工作房、休息房、班次起迄。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub entity_id: String,
    pub work_room: String,
    pub rest_room: String,
    #[serde(default)]
    pub shift_start: i32,
    #[serde(default)]
    pub shift_end: i32,
}

/// 實體（玩家/NPC）供 JSON 背板。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub display_char: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub move_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_y: Option<i32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub walk_or_run: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_started_at: Option<i64>,
    #[serde(default)]
    pub vit: i32,
    #[serde(default)]
    pub qi: i32,
    #[serde(default)]
    pub dex: i32,
    #[serde(default)]
    pub magnesium: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_observed_at: Option<i64>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub gender: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soul_seed: Option<i64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub display_title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub activated_nodes: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub equipment_slots: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub inventory: String,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub disposition: i32,
    /// 表面可觀測行為（非內心意圖），供玩家 Look 時顯示。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub current_activity: String,
    /// Hex even-q 座標 q；`None` 表示尚未綁定 Hex 世界（僅 Room／平面 x,y）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hex_q: Option<i32>,
    /// 野外六角 even-q 座標 r。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hex_r: Option<i32>,
    /// 正方格座標 x（東增）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_x: Option<i32>,
    /// 正方格座標 y（北增）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_y: Option<i32>,
}

/// 物品定義。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub item_type: String,
    #[serde(default)]
    pub weight: f64,
    #[serde(default)]
    pub stackable: i32,
    #[serde(default)]
    pub denomination: i32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub vit_bonus: i32,
    #[serde(default)]
    pub dex_bonus: i32,
    #[serde(default)]
    pub atk_bonus: i32,
}

/// 事件日誌一筆。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    pub at: i64,
    pub entity_id: String,
    pub event_type: String,
    pub payload: String,
}

/// NPC 長期記憶一條。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalEntry {
    pub entity_id: String,
    pub content: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub created_at: i64,
}

/// NPC 短期記憶：對某位玩家的見面次數與好感度。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMemory {
    pub entity_id: String,
    pub subject_id: String,
    #[serde(default)]
    pub meet_count: i32,
    #[serde(default)]
    pub favorability: i32,
}

/// 兩名 NPC 的短期話題線狀態。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcThread {
    pub thread_key: String,
    #[serde(default)]
    pub topic_type: String,
    #[serde(default)]
    pub phase: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<String>,
    #[serde(default)]
    pub turn_count: i32,
    #[serde(default)]
    pub cooldown_until: i64,
    #[serde(default)]
    pub updated_at: i64,
}

/// 兩名 NPC 的關係狀態。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDyad {
    pub a_id: String,
    pub b_id: String,
    #[serde(default)]
    pub familiarity: i32,
    #[serde(default)]
    pub sentiment: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub updated_at: i64,
}

/// 可衰減的社會傳聞。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRumor {
    pub id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub room_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zone: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub source_score: i32,
    #[serde(default)]
    pub weight: i32,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub mention_count: i32,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub last_used_at: i64,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub blocked_until: i64,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub penalty_count: i32,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub last_penalty_at: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub last_penalty_reason: String,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub expires_at: i64,
}

/// 傳聞池批次摘要。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRumorDigest {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub source_count: i32,
    #[serde(default)]
    pub updated_at: i64,
}

// ══════════════════════════════════════
//  serde 輔助
// ══════════════════════════════════════

pub(crate) fn is_zero_i32(v: &i32) -> bool { *v == 0 }
pub(crate) fn is_zero_i64(v: &i64) -> bool { *v == 0 }

// ══════════════════════════════════════
//  JSON 載入/寫出輔助結構（模組內部使用）
// ══════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct RoomFileOne {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) zone: String,
    #[serde(default)]
    pub(crate) exits: Vec<ExitOut>,
    #[serde(default)]
    pub(crate) objects: Vec<model::RoomObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ExitOut {
    pub(crate) direction: String,
    pub(crate) to: String,
}

#[derive(Deserialize, Default)]
pub(crate) struct RoomsFile {
    #[serde(default)]
    pub(crate) rooms: Vec<RoomDef>,
    #[serde(default)]
    pub(crate) exits: Vec<ExitDef>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RoomDef {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) zone: String,
    #[serde(default)]
    pub(crate) description: String,
}

#[derive(Deserialize, Default)]
pub(crate) struct ExitDef {
    pub(crate) from: String,
    pub(crate) direction: String,
    pub(crate) to: String,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct EntityRoomsFile {
    #[serde(default)]
    pub(crate) entries: Vec<EntityRoomEntry>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct EntityRoomEntry {
    pub(crate) entity_id: String,
    pub(crate) room_id: String,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct VenuesFile {
    #[serde(default)]
    pub(crate) venues: Vec<Venue>,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct AssignmentsFile {
    #[serde(default)]
    pub(crate) entries: Vec<Assignment>,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct SchedulesFile {
    #[serde(default)]
    pub(crate) entries: Vec<Schedule>,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct EntitiesFile {
    #[serde(default)]
    pub(crate) entities: Vec<Entity>,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct ItemsFile {
    #[serde(default)]
    pub(crate) items: Vec<Item>,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct NpcThreadsFile {
    #[serde(default)]
    pub(crate) entries: Vec<NpcThread>,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct NpcDyadsFile {
    #[serde(default)]
    pub(crate) entries: Vec<NpcDyad>,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct NpcRumorsFile {
    #[serde(default)]
    pub(crate) entries: Vec<NpcRumor>,
}

/// 與既有 `npcRumorDigestFile`、現行 `npc_rumor_digest.json` 包一層 `digest` 對齊。
#[derive(Serialize, Deserialize)]
pub(crate) struct NpcRumorDigestFile {
    pub(crate) digest: NpcRumorDigest,
}

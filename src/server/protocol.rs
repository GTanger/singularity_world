//! WebSocket／HTTP JSON 訊息型別（對齊既有 `server/protocol`）。
//! 欄位名與 `omitempty` 行為需與前端一致。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 客戶端送出的 JSON；`type` 決定行為。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub player_id: String,
    #[serde(default)]
    pub password: String,
    #[serde(default, rename = "display_char")]
    pub display_char: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub direction: String,
    #[serde(default, rename = "entity_id")]
    pub entity_id: String,
    #[serde(default, rename = "item_id")]
    pub item_id: String,
    #[serde(default, rename = "target_slot")]
    pub target_slot: String,
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub action: String,
    #[serde(default, rename = "player_input")]
    pub player_input: String,
}

/// 房間內單一實體（對齊 `ViewEntity`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewEntity {
    pub id: String,
    pub kind: String,
    #[serde(rename = "display_char")]
    pub display_char: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
}

/// `do_action` 敘事結果（對齊 `ActionResultMsg`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionResultMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub action: String,
    #[serde(rename = "target_id")]
    pub target_id: String,
    #[serde(rename = "target_name")]
    pub target_name: String,
    pub narrative: String,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "move_target_id"
    )]
    pub move_target_id: String,
}

/// 單一出口（對齊 `ExitView`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitView {
    pub direction: String,
    #[serde(rename = "to_room_id")]
    pub to_room_id: String,
    #[serde(rename = "to_room_name")]
    pub to_room_name: String,
}

/// 房間內可互動物件（對齊 `ViewObject`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewObject {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<String>,
}

/// 當前房間視野（對齊 `RoomViewMsg`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomViewMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "room_id")]
    pub room_id: String,
    #[serde(rename = "room_name")]
    pub room_name: String,
    pub description: String,
    pub exits: Vec<ExitView>,
    pub entities: Vec<ViewEntity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<ViewObject>,
    #[serde(rename = "server_unix")]
    pub server_unix: i64,
    #[serde(rename = "game_time_sec_since_midnight")]
    pub game_time_sec_since_midnight: i32,
    #[serde(rename = "game_days_since_epoch")]
    pub game_days_since_epoch: i32,
}

/// 登入成功後玩家狀態（對齊 `MeMsg`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "player_id")]
    pub player_id: String,
    #[serde(rename = "room_id")]
    pub room_id: String,
    #[serde(rename = "room_name")]
    pub room_name: String,
    pub vit: i32,
    pub qi: i32,
    pub dex: i32,
    #[serde(rename = "hp_cur")]
    pub hp_cur: i32,
    #[serde(rename = "hp_max")]
    pub hp_max: i32,
    #[serde(rename = "inner_cur")]
    pub inner_cur: i32,
    #[serde(rename = "inner_max")]
    pub inner_max: i32,
    #[serde(rename = "spirit_cur")]
    pub spirit_cur: i32,
    #[serde(rename = "spirit_max")]
    pub spirit_max: i32,
    #[serde(rename = "stamina_cur")]
    pub stamina_cur: i32,
    #[serde(rename = "stamina_max")]
    pub stamina_max: i32,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "display_title"
    )]
    pub display_title: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "origin_sentence"
    )]
    pub origin_sentence: String,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "activated_nodes"
    )]
    pub activated_nodes: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "topology_costs"
    )]
    pub topology_costs: Vec<f64>,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "equipment_slots"
    )]
    pub equipment_slots: HashMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "equipment_names"
    )]
    pub equipment_names: HashMap<String, String>,
}

/// 移動廣播（對齊 `MovedMsg`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovedMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "player_id")]
    pub player_id: String,
    #[serde(rename = "room_id")]
    pub room_id: String,
    #[serde(rename = "room_name")]
    pub room_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub direction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PongMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrateMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyDebugAckMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryItemView {
    #[serde(rename = "item_id")]
    pub item_id: String,
    pub name: String,
    #[serde(rename = "item_type")]
    pub item_type: String,
    pub qty: i32,
    pub weight: f64,
    #[serde(rename = "sub_total")]
    pub sub_total: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub slot: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub items: Vec<InventoryItemView>,
    #[serde(rename = "current_weight")]
    pub current_weight: f64,
    #[serde(rename = "max_weight")]
    pub max_weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityStatusMsg {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "entity_id")]
    pub entity_id: String,
    #[serde(rename = "display_char")]
    pub display_char: String,
    pub vit: i32,
    pub qi: i32,
    pub dex: i32,
    #[serde(rename = "hp_cur")]
    pub hp_cur: i32,
    #[serde(rename = "hp_max")]
    pub hp_max: i32,
    #[serde(rename = "inner_cur")]
    pub inner_cur: i32,
    #[serde(rename = "inner_max")]
    pub inner_max: i32,
    #[serde(rename = "spirit_cur")]
    pub spirit_cur: i32,
    #[serde(rename = "spirit_max")]
    pub spirit_max: i32,
    #[serde(rename = "stamina_cur")]
    pub stamina_cur: i32,
    #[serde(rename = "stamina_max")]
    pub stamina_max: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magnesium: Option<i32>,
    #[serde(rename = "is_self")]
    pub is_self: bool,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "display_title"
    )]
    pub display_title: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        rename = "origin_sentence"
    )]
    pub origin_sentence: String,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "activated_nodes"
    )]
    pub activated_nodes: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "topology_costs"
    )]
    pub topology_costs: Vec<f64>,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "equipment_slots"
    )]
    pub equipment_slots: HashMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "equipment_names"
    )]
    pub equipment_names: HashMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "HashMap::is_empty",
        rename = "equipment_descs"
    )]
    pub equipment_descs: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_msg_minimal_json() {
        let j = r#"{"type":"ping"}"#;
        let m: ClientMsg = serde_json::from_str(j).unwrap();
        assert_eq!(m.msg_type, "ping");
        assert!(m.player_id.is_empty());
    }

    #[test]
    fn room_view_roundtrip() {
        let v = RoomViewMsg {
            msg_type: "view".into(),
            room_id: "r1".into(),
            room_name: "大廳".into(),
            description: "…".into(),
            exits: vec![ExitView {
                direction: "東".into(),
                to_room_id: "r2".into(),
                to_room_name: "走廊".into(),
            }],
            entities: vec![ViewEntity {
                id: "p1".into(),
                kind: "player".into(),
                display_char: "我".into(),
                display_name: "p1".into(),
                actions: vec![],
            }],
            objects: vec![],
            server_unix: 1700000000,
            game_time_sec_since_midnight: 3600,
            game_days_since_epoch: 1,
        };
        let s = serde_json::to_string(&v).unwrap();
        let v2: RoomViewMsg = serde_json::from_str(&s).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn entity_status_magnesium_omit() {
        let e = EntityStatusMsg {
            msg_type: "entity_status".into(),
            entity_id: "n1".into(),
            display_char: "店".into(),
            vit: 1,
            qi: 2,
            dex: 3,
            hp_cur: 10,
            hp_max: 10,
            inner_cur: 0,
            inner_max: 0,
            spirit_cur: 0,
            spirit_max: 0,
            stamina_cur: 0,
            stamina_max: 0,
            magnesium: None,
            is_self: false,
            display_title: String::new(),
            origin_sentence: String::new(),
            activated_nodes: vec![],
            topology_costs: vec![],
            equipment_slots: HashMap::new(),
            equipment_names: HashMap::new(),
            equipment_descs: HashMap::new(),
        };
        let s = serde_json::to_string(&e).unwrap();
        assert!(!s.contains("magnesium"));
        let e2: EntityStatusMsg = serde_json::from_str(&s).unwrap();
        assert_eq!(e2.magnesium, None);
    }

    #[test]
    fn action_result_with_move_target_id() {
        let j = r#"{"type":"action_result","action":"Look","target_id":"o1","target_name":"門","narrative":"…","success":true,"move_target_id":"door_1"}"#;
        let a: ActionResultMsg = serde_json::from_str(j).unwrap();
        assert_eq!(a.move_target_id, "door_1");
        let out = serde_json::to_string(&a).unwrap();
        assert!(out.contains("move_target_id"));
    }
}

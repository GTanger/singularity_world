//! Room editor 資料型別 + 請求型別。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RoomEditorExit {
    pub direction: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RoomEditorRoomFile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zone: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exits: Vec<RoomEditorExit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<model::RoomObject>,
}

#[derive(Serialize)]
pub(super) struct RoomEditorNode {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub zone: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<model::RoomObject>,
}

#[derive(Serialize)]
pub(super) struct RoomEditorEdge {
    pub from: String,
    pub to: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoomEditorPos {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize)]
pub(super) struct RoomEditorGraphResp {
    pub nodes: Vec<RoomEditorNode>,
    pub edges: Vec<RoomEditorEdge>,
    pub layout: HashMap<String, RoomEditorPos>,
    pub base_path: String,
}

// ── 請求型別 ──

#[derive(Deserialize)]
pub struct CreateReq {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub objects: Vec<model::RoomObject>,
    #[serde(default)]
    pub clone_from: String,
}

#[derive(Deserialize)]
pub struct UpdateReq {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub objects: Vec<model::RoomObject>,
}

#[derive(Deserialize)]
pub struct LinkReq {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub reverse: bool,
    #[serde(default)]
    pub reverse_direction: String,
}

#[derive(Deserialize)]
pub struct LayoutReq {
    pub positions: HashMap<String, RoomEditorPos>,
}

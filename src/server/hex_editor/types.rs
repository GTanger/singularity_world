//! Handler 請求 / 回應型別。

use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::hex::{LinkLayer, Terrain, TransportLinkClass, TransportMode};
use crate::model::RoomObject;
use crate::server::http_api::AdminQuery;

#[derive(Deserialize)]
pub struct CellReq {
    pub q: i32,
    pub r: i32,
    #[serde(default)]
    pub terrain: Terrain,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub objects: Vec<RoomObject>,
}

#[derive(Deserialize)]
pub struct WallReq {
    pub aq: i32,
    pub ar: i32,
    pub bq: i32,
    pub br: i32,
    #[serde(default)]
    pub remove: bool,
}

#[derive(Deserialize)]
pub struct PortalReq {
    pub name: String,
    pub from_q: i32,
    pub from_r: i32,
    pub to_q: i32,
    pub to_r: i32,
    #[serde(default = "default_true")]
    pub bidirectional: bool,
    #[serde(default)]
    pub counts_as_official_link: bool,
}

#[derive(Deserialize)]
pub struct TransportEdgeReq {
    pub id: Option<String>,
    pub aq: i32,
    pub ar: i32,
    pub bq: i32,
    pub br: i32,
    pub mode: TransportMode,
    #[serde(default = "default_true")]
    pub operational: bool,
    #[serde(default)]
    pub link_class: TransportLinkClass,
    pub weight: Option<f64>,
}

pub(super) fn default_true() -> bool {
    true
}

/// GET /api/hex/path 查詢參數（含 `mg_key`）
#[derive(Deserialize)]
pub struct PathGetQuery {
    #[serde(flatten)]
    pub admin: AdminQuery,
    pub from_q: i32,
    pub from_r: i32,
    pub to_q: i32,
    pub to_r: i32,
    #[serde(default)]
    pub layer: HexPathLayer,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HexPathLayer {
    #[default]
    Exploration,
    Official,
}

impl From<HexPathLayer> for LinkLayer {
    fn from(v: HexPathLayer) -> Self {
        match v {
            HexPathLayer::Exploration => LinkLayer::Exploration,
            HexPathLayer::Official => LinkLayer::Official,
        }
    }
}

#[derive(Deserialize)]
pub struct RevealReq {
    pub q: i32,
    pub r: i32,
}

#[derive(Deserialize)]
pub struct RevealRegionReq {
    pub center_q: i32,
    pub center_r: i32,
    pub radius: i32,
}

#[derive(Deserialize)]
pub struct WorldSeedReq {
    pub world_seed: u64,
}

pub(super) fn ok_json() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true}))
}

pub(super) fn ok_count(n: usize) -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true, "count": n}))
}

pub(super) fn err_json(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": msg})))
}

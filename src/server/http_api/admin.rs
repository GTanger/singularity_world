//! 管理授權 + wipe-entities + 設計常數。

use axum::extract::{Json, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::{config, gametext, store};

// ── 授權共用 ──

#[derive(Deserialize)]
pub struct AdminQuery {
    pub mg_key: Option<String>,
}

pub(super) fn is_admin_authorized(cfg: &config::Server, query: &AdminQuery) -> bool {
    if cfg.management_key.is_empty() {
        return true;
    }
    query.mg_key.as_deref() == Some(&cfg.management_key)
}

// ── /api/design-constants ──

pub async fn design_constants() -> impl IntoResponse {
    Json(config::design_constants())
}

// ── /api/admin/wipe-entities ──

pub async fn wipe_entities(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    Query(q): Query<AdminQuery>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權的管理操作"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let mut s = st.write();
    if let Err(e) = s.clear_all_entities() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    let body = gametext::admin_wipe_response();
    (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "application/json")], body).into_response()
}

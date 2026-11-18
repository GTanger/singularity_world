//! 人物靜態立繪 HTTP API 與玩家 OpenRouter BYOK 金鑰端點（`/api/rim/portrait/*`, `/api/rim/byok`）。

use axum::Json;
use axum::extract::Query;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use crate::ai::openai_chat::{
    DEFAULT_OPENROUTER_IMAGE_MODEL, get_player_openrouter_key, set_player_openrouter_key,
};
use crate::grid::substrate::rim_portrait_gen::{
    PortraitLookup, PortraitRequest, invalidate_actor_cache, lookup_or_schedule,
};
use crate::grid::substrate::rim_sim::global_rim;

use super::rim_auth::resolve_viewer_pid;

#[derive(Deserialize)]
pub struct PortraitQuery {
    pub id: Option<usize>,
    pub name: Option<String>,
    pub gender: Option<String>,
    pub soul_seed: Option<String>,
    pub role: Option<String>,
    pub wear_hat: Option<bool>,
    pub show_pack: Option<bool>,
    pub tool: Option<String>,
    pub hp_ratio: Option<f32>,
}

#[derive(Deserialize)]
pub struct ByokPostBody {
    pub api_key: String,
}

#[derive(Deserialize)]
pub struct RegenPostBody {
    pub reset_base: Option<bool>,
}

fn build_request_from_query(headers: &HeaderMap, q: &PortraitQuery) -> PortraitRequest {
    let viewer_pid = resolve_viewer_pid(headers);
    let q_id = q.id.unwrap_or(1);
    let q_name = q.name.clone().unwrap_or_else(|| "角色".to_string());

    let mut owner_player_id: Option<String> = None;
    let mut name = q_name;
    let mut gender = q.gender.clone().unwrap_or_else(|| "M".to_string());
    let mut soul_seed = q
        .soul_seed
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    let mut role_tag = q.role.clone().unwrap_or_default();
    let mut wear_hat = q.wear_hat.unwrap_or(true);
    let mut show_pack = q.show_pack.unwrap_or(true);
    let mut tool_label = q.tool.clone().filter(|t| !t.trim().is_empty());
    let mut injured = q.hp_ratio.is_some_and(|r| r < 0.78);
    let mut matched_id = q_id;

    // 若請求對象對應到在場實況角色，優先以權威服端資料為準並檢查是否屬登入者本人（姓名須相符，避免 demo id 碰撞）
    if let Ok(world) = global_rim().lock() {
        let actor_opt = world.actors.iter().find(|a| {
            if q.name.as_deref().is_some_and(|n| !n.trim().is_empty()) {
                a.name == name || (q.id == Some(a.id) && name.starts_with("陌生"))
            } else {
                q.id.is_some() && a.id == q_id
            }
        });
        if let Some(a) = actor_opt {
            matched_id = a.id;
            name = a.name.clone();
            gender = a.gender.clone();
            if a.soul_seed != 0 {
                soul_seed = a.soul_seed;
            }
            if !a.role_tag.is_empty() {
                role_tag = a.role_tag.clone();
            }
            if q.wear_hat.is_none() {
                wear_hat = a.equipped_hat;
            }
            if q.show_pack.is_none() {
                show_pack = a.equipped_pack || a.carrying.is_some();
            }
            if q.tool.is_none() {
                tool_label = a.equipped_tool.map(|t| t.label().to_string());
            }
            if q.hp_ratio.is_none() {
                injured = (a.hp as f32 / a.max_hp.max(1) as f32) < 0.78
                    || a.injury_ticks > 0
                    || a.downed_ticks > 0;
            }
            if let Some(ref pid) = viewer_pid
                && a.entity_id.as_deref() == Some(pid.as_str())
            {
                owner_player_id = Some(pid.clone());
            }
        }
    }

    PortraitRequest {
        id: matched_id,
        name,
        gender,
        soul_seed,
        role_tag,
        wear_hat,
        show_pack,
        tool_label,
        injured,
        owner_player_id,
    }
}

fn sniff_image_mime(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, 0x50, 0x4e, 0x47]) {
        "image/png"
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/jpeg"
    }
}

pub async fn rim_portrait_image(headers: HeaderMap, Query(q): Query<PortraitQuery>) -> Response {
    let req = build_request_from_query(&headers, &q);
    match lookup_or_schedule(&req) {
        PortraitLookup::Ready { path, .. } => serve_image_file(&path, "ready"),
        PortraitLookup::Updating { base_path, .. } => serve_image_file(&base_path, "updating"),
        PortraitLookup::Pending { .. } => (
            StatusCode::ACCEPTED,
            [(header::CACHE_CONTROL, "no-store")],
            Json(json!({ "status": "pending" })),
        )
            .into_response(),
    }
}

fn serve_image_file(path: &std::path::Path, status: &str) -> Response {
    let Ok(bytes) = std::fs::read(path) else {
        return StatusCode::ACCEPTED.into_response();
    };
    let mime = sniff_image_mime(&bytes);
    let mut resp = Response::new(axum::body::Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    let headers = resp.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(v) = HeaderValue::from_str(status) {
        headers.insert("X-Portrait-Status", v);
    }
    resp
}

pub async fn rim_portrait_regen(headers: HeaderMap, Json(body): Json<RegenPostBody>) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "msg": "請先登入角色" })),
        )
            .into_response();
    };
    let req_opt = {
        let Ok(world) = global_rim().lock() else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        };
        world
            .actors
            .iter()
            .find(|a| a.entity_id.as_deref() == Some(pid.as_str()))
            .map(|a| PortraitRequest {
                id: a.id,
                name: a.name.clone(),
                gender: a.gender.clone(),
                soul_seed: a.soul_seed,
                role_tag: a.role_tag.clone(),
                wear_hat: a.equipped_hat,
                show_pack: a.equipped_pack || a.carrying.is_some(),
                tool_label: a.equipped_tool.map(|t| t.label().to_string()),
                injured: (a.hp as f32 / a.max_hp.max(1) as f32) < 0.78
                    || a.injury_ticks > 0
                    || a.downed_ticks > 0,
                owner_player_id: Some(pid.clone()),
            })
    };
    let Some(req) = req_opt else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "ok": false, "msg": "尚未進入世界" })),
        )
            .into_response();
    };
    let include_base = body.reset_base.unwrap_or(true);
    let _ = invalidate_actor_cache(&req, include_base);
    let _ = lookup_or_schedule(&req);
    Json(json!({
        "ok": true,
        "msg": "已開始重新繪製人像",
    }))
    .into_response()
}

pub async fn rim_byok_get(headers: HeaderMap) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "msg": "請先登入角色" })),
        )
            .into_response();
    };
    let has_key = tokio::task::spawn_blocking(move || get_player_openrouter_key(&pid).is_some())
        .await
        .ok()
        .unwrap_or(false);
    Json(json!({
        "ok": true,
        "has_key": has_key,
        "openrouter_model": DEFAULT_OPENROUTER_IMAGE_MODEL,
    }))
    .into_response()
}

pub async fn rim_byok_post(headers: HeaderMap, Json(body): Json<ByokPostBody>) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "ok": false, "msg": "請先登入角色" })),
        )
            .into_response();
    };
    let pid_save = pid.clone();
    let raw_key = body.api_key;
    let has_key =
        tokio::task::spawn_blocking(move || set_player_openrouter_key(&pid_save, &raw_key))
            .await
            .ok()
            .unwrap_or(false);
    let msg = if has_key {
        "已儲存個人金鑰"
    } else {
        "已清除個人金鑰"
    };
    Json(json!({
        "ok": true,
        "has_key": has_key,
        "openrouter_model": DEFAULT_OPENROUTER_IMAGE_MODEL,
        "msg": msg,
    }))
    .into_response()
}

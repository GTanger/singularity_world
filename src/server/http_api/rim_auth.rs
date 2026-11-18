//! 輕量版 2D 開放世界 HTTP Cookie 帳號認證與開譜曆時間共用輸出。

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use axum::Json;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::db;
use crate::entity::{EntityKind, Gender};
use crate::game::{self, now_unix};
use crate::gametext;
use crate::grid::substrate::rim_sim::global_rim;
use crate::pg::{self, rim_persist};

const SESSION_COOKIE_NAME: &str = "sw_rim_sid";
static RIM_SESSIONS: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Deserialize)]
pub struct RimAuthBody {
    pub action: String,
    pub player_id: String,
    pub password: String,
    #[serde(default)]
    pub display_char: String,
    #[serde(default)]
    pub gender: String,
}

/// 統一產出權威開譜曆日時辰 JSON 與顯示標題（供登入頁、狀態頁與晝夜天色系統共用）。
pub(super) fn build_sky_time_payload() -> (String, Value) {
    let real_unix = now_unix();
    let (epoch, scale) = game::game_time_cfg();
    let (sec_since_midnight, hour, min, days) = game::game_time_now(real_unix, epoch, scale);
    let date_label = game::format_kaipu_date_label(days);
    let period = game::time_period_label(hour);
    let world_clock = format!("{date_label}・{period}");
    let sky_time = json!({
        "date_label": date_label,
        "period": period,
        "hour": hour,
        "min": min,
        "sec_since_midnight": sec_since_midnight,
        "server_unix": real_unix,
        "scale": scale,
    });
    (world_clock, sky_time)
}

pub(super) fn extract_sid_from_headers(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let trimmed = part.trim();
        if let Some(val) = trimmed.strip_prefix("sw_rim_sid=") {
            let sid = val.trim();
            if !sid.is_empty() && sid.len() <= 96 {
                return Some(sid.to_string());
            }
        }
    }
    None
}

pub(super) fn resolve_viewer_pid(headers: &HeaderMap) -> Option<String> {
    let sid = extract_sid_from_headers(headers)?;
    if let Ok(guard) = RIM_SESSIONS.read()
        && let Some(pid) = guard.get(&sid)
    {
        return Some(pid.clone());
    }
    let pool = pg::pool()?;
    let pid = rim_persist::resolve_session(&pool, &sid)?;
    if let Ok(mut guard) = RIM_SESSIONS.write() {
        guard.insert(sid, pid.clone());
    }
    Some(pid)
}

fn json_err(status: StatusCode, msg: &str) -> Response {
    (status, Json(json!({ "ok": false, "msg": msg }))).into_response()
}

fn authenticate_or_create_entity(
    body: RimAuthBody,
) -> Result<(String, String, String, i64), (StatusCode, String)> {
    let pid = body.player_id.trim().to_string();
    let password = body.password;
    let is_create = body.action == "create";

    if pid.is_empty() {
        let key = if is_create {
            "create_need_id"
        } else {
            "login_need_id"
        };
        return Err((StatusCode::BAD_REQUEST, gametext::client(key)));
    }
    if password.is_empty() {
        let key = if is_create {
            "create_need_password"
        } else {
            "login_need_password"
        };
        return Err((StatusCode::BAD_REQUEST, gametext::client(key)));
    }

    if is_create {
        if password.len() < 6 {
            return Err((
                StatusCode::BAD_REQUEST,
                gametext::client("create_password_short"),
            ));
        }
        let pid_len = pid.chars().count();
        if !(2..=32).contains(&pid_len) {
            return Err((StatusCode::BAD_REQUEST, gametext::client("create_id_len")));
        }
        let existing = db::get_entity(&pid).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                gametext::client("create_failed"),
            )
        })?;
        if existing.is_some() {
            return Err((StatusCode::CONFLICT, gametext::client("create_id_taken")));
        }
        let mut display_char = body.display_char.trim().to_string();
        if display_char.is_empty() {
            display_char = pid.chars().next().map_or_else(
                || gametext::client("display_char_default"),
                |c| c.to_string(),
            );
        } else if let Some(first) = display_char.chars().next() {
            display_char = first.to_string();
        }
        let gender = if body.gender == "女" || body.gender == "F" {
            "F"
        } else {
            "M"
        };
        db::insert_entity(&pid, &display_char, gender).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                gametext::client("create_entity_failed"),
            )
        })?;
        db::create_auth(&pid, &password).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                gametext::client("create_auth_failed"),
            )
        })?;
    } else {
        let ent = db::get_entity(&pid).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                gametext::client("login_verify_failed"),
            )
        })?;
        let Some(ent) = ent else {
            return Err((
                StatusCode::UNAUTHORIZED,
                gametext::client("login_not_found"),
            ));
        };
        if !matches!(ent.kind, EntityKind::Player) {
            return Err((StatusCode::FORBIDDEN, gametext::client("login_not_player")));
        }
        if !db::has_password_for_entity(&pid) {
            return Err((
                StatusCode::UNAUTHORIZED,
                gametext::client("login_no_password"),
            ));
        }
        let ok = db::verify_password(&pid, &password).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                gametext::client("login_verify_failed"),
            )
        })?;
        if !ok {
            return Err((
                StatusCode::UNAUTHORIZED,
                gametext::client("login_wrong_password"),
            ));
        }
    }

    let ent = db::get_entity(&pid).ok().flatten().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            gametext::client("login_verify_failed"),
        )
    })?;
    let glyph = if ent.display_char.trim().is_empty() {
        pid.chars()
            .next()
            .map_or_else(|| "我".into(), |c| c.to_string())
    } else {
        ent.display_char
    };
    let gender = match ent.gender {
        Some(Gender::F) => "F".to_string(),
        _ => "M".to_string(),
    };
    let seed = ent.soul_seed.unwrap_or(0);
    Ok((pid, glyph, gender, seed))
}

pub async fn rim_auth_post(Json(body): Json<RimAuthBody>) -> Response {
    let res = tokio::task::spawn_blocking(move || authenticate_or_create_entity(body)).await;
    let (pid, glyph, gender, soul_seed) = match res {
        Ok(Ok(info)) => info,
        Ok(Err((st, msg))) => return json_err(st, &msg),
        Err(_) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, "認證執行逾時"),
    };

    if let Ok(mut world) = global_rim().lock() {
        world.ensure_player_actor(&pid, &glyph, &gender, soul_seed);
        world.persist_if_needed(true);
    }

    let sid = Uuid::new_v4().simple().to_string();
    if let Ok(mut guard) = RIM_SESSIONS.write() {
        guard.insert(sid.clone(), pid.clone());
    }
    if let Some(pool) = pg::pool() {
        let sid_clone = sid.clone();
        let pid_clone = pid.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let _ = rim_persist::save_session(&pool, &sid_clone, &pid_clone);
        })
        .await;
    }

    let (world_clock, sky_time) = build_sky_time_payload();
    let cookie =
        format!("{SESSION_COOKIE_NAME}={sid}; Path=/; Max-Age=2592000; HttpOnly; SameSite=Lax");
    let mut resp = Json(json!({
        "ok": true,
        "player_id": pid,
        "glyph": glyph,
        "gender": gender,
        "soul_seed": soul_seed.to_string(),
        "world_clock": world_clock,
        "sky_time": sky_time,
    }))
    .into_response();
    if let Ok(val) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, val);
    }
    resp
}

pub async fn rim_auth_me(headers: HeaderMap) -> Response {
    let (world_clock, sky_time) = build_sky_time_payload();
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "ok": false,
                "world_clock": world_clock,
                "sky_time": sky_time,
            })),
        )
            .into_response();
    };

    let pid_lookup = pid.clone();
    let ent_opt = tokio::task::spawn_blocking(move || db::get_entity(&pid_lookup).ok().flatten())
        .await
        .ok()
        .flatten();
    let Some(ent) = ent_opt else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "ok": false,
                "world_clock": world_clock,
                "sky_time": sky_time,
            })),
        )
            .into_response();
    };

    let glyph = if ent.display_char.trim().is_empty() {
        pid.chars()
            .next()
            .map_or_else(|| "我".into(), |c| c.to_string())
    } else {
        ent.display_char
    };
    let gender = match ent.gender {
        Some(Gender::F) => "F".to_string(),
        _ => "M".to_string(),
    };
    let soul_seed = ent.soul_seed.unwrap_or(0);
    if let Ok(mut world) = global_rim().lock() {
        world.ensure_player_actor(&pid, &glyph, &gender, soul_seed);
    }

    Json(json!({
        "ok": true,
        "player_id": pid,
        "glyph": glyph,
        "gender": gender,
        "soul_seed": soul_seed.to_string(),
        "world_clock": world_clock,
        "sky_time": sky_time,
    }))
    .into_response()
}

pub async fn rim_auth_logout(headers: HeaderMap) -> Response {
    if let Some(sid) = extract_sid_from_headers(&headers) {
        if let Ok(mut guard) = RIM_SESSIONS.write() {
            guard.remove(&sid);
        }
        if let Some(pool) = pg::pool() {
            let _ = tokio::task::spawn_blocking(move || {
                let _ = rim_persist::delete_session(&pool, &sid);
            })
            .await;
        }
    }
    let clear_cookie = format!("{SESSION_COOKIE_NAME}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax");
    let mut resp = Json(json!({ "ok": true })).into_response();
    if let Ok(val) = HeaderValue::from_str(&clear_cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, val);
    }
    resp
}

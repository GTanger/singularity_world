//! 外海大島（`Feature 6`）奇幻百戶大世界 HTTP JSON 端點。

use std::time::{SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use crate::grid::outer_island::dialogue_llm::generate_npc_dialogue;
use crate::grid::outer_island::global_island;

use super::rim_auth::resolve_viewer_pid;

#[derive(Deserialize)]
pub struct IslandSetupBody {
    pub display_name: String,
    #[serde(default)]
    pub race: String,
    #[serde(default)]
    pub appearance: String,
    #[serde(default)]
    pub origin_pack: String,
}

#[derive(Deserialize)]
pub struct IslandMoveBody {
    pub to: String,
}

#[derive(Deserialize)]
pub struct IslandActBody {
    pub action: String,
    #[serde(default)]
    pub param: String,
}

#[derive(Deserialize)]
pub struct IslandTalkBody {
    pub npc_id: String,
    #[serde(default)]
    pub topic: String,
    #[serde(default)]
    pub message: String,
}

fn unauth_resp() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "ok": false, "msg": "請先向港務官核對通行身分。" })),
    )
        .into_response()
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

pub async fn island_state_get(headers: HeaderMap) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return unauth_resp();
    };
    let payload =
        tokio::task::spawn_blocking(move || global_island().build_player_view(&pid, &pid))
            .await
            .unwrap_or_else(|_| json!({ "ok": false, "msg": "海風吹散了回音，請稍候再看。" }));
    Json(payload).into_response()
}

pub async fn island_map_get(headers: HeaderMap) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return unauth_resp();
    };
    let payload = tokio::task::spawn_blocking(move || global_island().build_map_view(&pid, &pid))
        .await
        .unwrap_or_else(|_| json!({ "ok": false, "msg": "海霧迷濛，請稍候再展卷。" }));
    Json(payload).into_response()
}

pub async fn island_setup_post(headers: HeaderMap, Json(body): Json<IslandSetupBody>) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return unauth_resp();
    };
    let payload = tokio::task::spawn_blocking(move || {
        let eng = global_island();
        match eng.setup_player(
            &pid,
            &body.display_name,
            &body.race,
            &body.appearance,
            &body.origin_pack,
        ) {
            Ok(msg) => json!({
                "ok": true,
                "msg": msg,
                "view": eng.build_player_view(&pid, &pid),
            }),
            Err(err) => json!({ "ok": false, "msg": err }),
        }
    })
    .await
    .unwrap_or_else(|_| json!({ "ok": false, "msg": "港務木冊暫時忙碌，請稍候再試。" }));
    Json(payload).into_response()
}

pub async fn island_move_post(headers: HeaderMap, Json(body): Json<IslandMoveBody>) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return unauth_resp();
    };
    let ms = now_unix_ms();
    let payload = tokio::task::spawn_blocking(move || {
        let eng = global_island();
        match eng.move_player(&pid, body.to.trim(), ms) {
            Ok(msg) => json!({
                "ok": true,
                "msg": msg,
                "view": eng.build_player_view(&pid, &pid),
            }),
            Err(err) => json!({
                "ok": false,
                "msg": err,
                "view": eng.build_player_view(&pid, &pid),
            }),
        }
    })
    .await
    .unwrap_or_else(|_| json!({ "ok": false, "msg": "腳下石板濕滑，請再邁一步。" }));
    Json(payload).into_response()
}

pub async fn island_act_post(headers: HeaderMap, Json(body): Json<IslandActBody>) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return unauth_resp();
    };
    let payload = tokio::task::spawn_blocking(move || {
        let eng = global_island();
        let res = if body.action.trim() == "explore" {
            eng.explore_here(&pid)
        } else {
            eng.do_action(&pid, body.action.trim(), &body.param)
        };
        match res {
            Ok(msg) => json!({
                "ok": true,
                "msg": msg,
                "view": eng.build_player_view(&pid, &pid),
            }),
            Err(err) => json!({
                "ok": false,
                "msg": err,
                "view": eng.build_player_view(&pid, &pid),
            }),
        }
    })
    .await
    .unwrap_or_else(|_| json!({ "ok": false, "msg": "四周人聲嘈雜，請稍候再試。" }));
    Json(payload).into_response()
}

pub async fn island_talk_post(headers: HeaderMap, Json(body): Json<IslandTalkBody>) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return unauth_resp();
    };
    let ms = now_unix_ms();
    let payload = tokio::task::spawn_blocking(move || {
        let eng = global_island();
        let input = match eng.prepare_talk_input(
            &pid,
            body.npc_id.trim(),
            &body.topic,
            &body.message,
            ms,
        ) {
            Ok(i) => i,
            Err(err) => return json!({ "ok": false, "msg": err }),
        };
        let npc_id = input.npc.id.clone();
        let npc_name = input.npc.name.clone();
        let npc_role = input.npc.role.clone();
        let npc_race = input.npc.race.clone();
        let out = generate_npc_dialogue(&input);
        let lang_note = match eng.apply_dialogue_effects(&pid, &npc_id, &npc_name, &out) {
            Ok(note) => note,
            Err(err) => {
                return json!({
                    "ok": false,
                    "msg": err,
                    "view": eng.build_player_view(&pid, &pid),
                });
            }
        };
        json!({
            "ok": true,
            "npc_id": npc_id,
            "npc_name": npc_name,
            "npc_role": npc_role,
            "npc_race": npc_race,
            "reply": out.npc_reply,
            "side_overheard": out.side_overheard,
            "lang_note": lang_note,
            "view": eng.build_player_view(&pid, &pid),
        })
    })
    .await
    .unwrap_or_else(|_| json!({ "ok": false, "msg": "海風吹散了話音，請再開口一次。" }));
    Json(payload).into_response()
}

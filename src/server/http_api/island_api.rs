//! 外海大島（`Feature 6`）奇幻百戶大世界 HTTP JSON 端點。

use std::time::{SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::Query;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use crate::grid::outer_island::dialogue_llm::generate_npc_dialogue;
use crate::grid::outer_island::gazetteer::build_in_world_scroll_bundle_json;
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
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub gx: Option<f32>,
    #[serde(default)]
    pub gy: Option<f32>,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub steps: Option<u8>,
    #[serde(default)]
    pub prev_digest: Option<String>,
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

#[derive(Deserialize, Default)]
pub struct IslandScrollQuery {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub id: String,
}

pub async fn island_scroll_get(headers: HeaderMap, Query(q): Query<IslandScrollQuery>) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return unauth_resp();
    };
    let eng = global_island();
    let payload = match eng.delta.read() {
        Ok(guard) => {
            let aid = guard
                .resolve_controller(&pid)
                .unwrap_or_else(|| format!("pl:{pid}"));
            let nid = guard
                .actors
                .get(&aid)
                .map_or(eng.doc.spawn_node.as_str(), |a| a.node_id.as_str());
            let mut parts = nid.split(':');
            let _ = parts.next();
            let town_slug = match parts.next().unwrap_or("white_gull_harbor") {
                "gull_tail_port" => "gull_tail_port",
                "west_strait_ferry" => "west_strait_ferry",
                "greypine_post" => "greypine_post",
                "north_cape_beacon" => "north_cape_beacon",
                _ => "white_gull_harbor",
            };
            let mode = if q.mode == "annals" {
                "annals"
            } else {
                "stele"
            };
            let sel = (!q.id.trim().is_empty()).then_some(q.id.trim());
            build_in_world_scroll_bundle_json(&eng.doc, &guard, town_slug, mode, sel)
        }
        Err(_) => json!({ "ok": false, "msg": "四周人聲嘈雜，請稍候再讀。" }),
    };
    Json(payload).into_response()
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

pub async fn island_pulse_get(headers: HeaderMap) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return unauth_resp();
    };
    Json(global_island().pulse_meta(&pid)).into_response()
}

pub async fn island_leave_post(headers: HeaderMap) -> Response {
    if let Some(pid) = resolve_viewer_pid(&headers) {
        crate::grid::outer_island::live_tide::revoke_client_lease(&pid);
    }
    Json(json!({ "ok": true })).into_response()
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
    let prev_digest = body
        .prev_digest
        .as_deref()
        .and_then(|s| s.trim().parse::<u64>().ok());
    let payload = tokio::task::spawn_blocking(move || {
        let eng = global_island();
        let (res, view, map_sync) = eng.move_player_with_sync(
            &pid,
            body.to.trim(),
            body.gx,
            body.gy,
            body.mode.trim(),
            body.steps,
            prev_digest,
            ms,
        );
        match res {
            Ok(msg) => json!({
                "ok": true,
                "msg": msg,
                "view": view,
                "map_sync": map_sync,
            }),
            Err(err) => json!({
                "ok": false,
                "msg": err,
                "view": view,
                "map_sync": map_sync,
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
        let act_trimmed = body.action.trim();
        let res = if act_trimmed == "carriage_next" {
            Err("車船正順著官道航路穩步前行，無須手動催趕；若欲半途落腳，可向駕手示意「下一站停靠下車」。".to_string())
        } else if act_trimmed == "explore" {
            eng.explore_here(&pid)
        } else {
            eng.do_action(&pid, act_trimmed, &body.param)
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
        if let Some(peer_res) =
            eng.try_peer_direct_talk(&pid, body.npc_id.trim(), &body.message, ms)
        {
            return match peer_res {
                Ok(v) => v,
                Err(err) => json!({
                    "ok": false,
                    "msg": err,
                    "view": eng.build_player_view(&pid, &pid),
                }),
            };
        }
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
        let (lang_note, boast_callout) =
            match eng.apply_dialogue_effects(&pid, &npc_id, &npc_name, &out) {
                Ok(pair) => pair,
                Err(err) => {
                    return json!({
                        "ok": false,
                        "msg": err,
                        "view": eng.build_player_view(&pid, &pid),
                    });
                }
            };
        let combined_side = match (out.side_overheard, boast_callout) {
            (Some(a), Some(b)) => Some(format!("{a}\n{b}")),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        json!({
            "ok": true,
            "npc_id": npc_id,
            "npc_name": npc_name,
            "npc_role": npc_role,
            "npc_race": npc_race,
            "reply": out.npc_reply,
            "side_overheard": combined_side,
            "lang_note": lang_note,
            "view": eng.build_player_view(&pid, &pid),
        })
    })
    .await
    .unwrap_or_else(|_| json!({ "ok": false, "msg": "海風吹散了話音，請再開口一次。" }));
    Json(payload).into_response()
}

pub async fn island_qa_cleanup_post(headers: HeaderMap) -> Response {
    let Some(pid) = resolve_viewer_pid(&headers) else {
        return unauth_resp();
    };
    if pid != "手機驗收員" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "ok": false, "msg": "僅供本機自動化驗收收尾使用。" })),
        )
            .into_response();
    }
    let payload =
        tokio::task::spawn_blocking(move || match global_island().cleanup_qa_actor(&pid) {
            Ok(removed) => json!({ "ok": true, "removed": removed }),
            Err(err) => json!({ "ok": false, "msg": err }),
        })
        .await
        .unwrap_or_else(|_| json!({ "ok": false }));
    Json(payload).into_response()
}

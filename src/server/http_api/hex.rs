//! Hex 揭露 / 偵查 / 探索 / 移動 / 視野 / 已揭露列。

use axum::extract::{Json, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::entity::EntityKind;
use crate::hex::HexCoord;
use crate::server::hex_editor;
use crate::{db, store};

#[derive(Deserialize)]
pub struct HexPlayerRevealBody {
    pub id: String,
    pub pw: String,
    pub q: i32,
    pub r: i32,
}

#[derive(Deserialize)]
pub struct HexViewQuery {
    pub player_id: String,
}

/// GET /api/hex/view?player_id=xxx — 取得該玩家周圍視距內的六角格視野。
pub async fn hex_view(Query(q): Query<HexViewQuery>) -> impl IntoResponse {
    let (hex_q, hex_r) = if let Some(st) = store::get_store() {
        let s = st.read();
        s.get_entity(&q.player_id)
            .map(|e| (e.hex_q, e.hex_r))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };

    let (Some(q_val), Some(r_val)) = (hex_q, hex_r) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"玩家無座標"}))).into_response();
    };

    let radius = 5;
    let game_hour = -1;
    match crate::game::get_hex_area_view(q_val, r_val, radius, game_hour) {
        Ok(view) => Json(view).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /api/hex/player-reveal — 驗證玩家後：確保世界格存在，並寫入該玩家之已揭露列。
pub async fn hex_player_reveal(Json(body): Json<HexPlayerRevealBody>) -> impl IntoResponse {
    let ok = db::verify_password(&body.id, &body.pw).unwrap_or(false);
    if !ok {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"身份驗證失敗"}))).into_response();
    }
    let ent = match db::get_entity(&body.id) {
        Ok(Some(e)) => e,
        _ => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"角色不存在"}))).into_response(),
    };
    if ent.kind != EntityKind::Player {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"僅玩家角色可標記格網揭露"}))).into_response();
    }
    let coord = HexCoord::new(body.q, body.r);
    let cell_id = coord.to_cell_id();
    let (cell, world_new) = match hex_editor::ensure_world_cell_at(coord) {
        Ok(x) => x,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("世界格寫入失敗：{e}")})),
            )
                .into_response();
        }
    };
    let self_new = match db::mark_player_hex_revealed(&body.id, &cell_id) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": format!("已揭露紀錄寫入失敗：{e}")})),
            )
                .into_response();
        }
    };
    Json(serde_json::json!({
        "cell": cell,
        "cell_id": cell_id,
        "world_newly_committed": world_new,
        "self_newly_marked": self_new,
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct HexMyRevealedQuery {
    pub id: String,
    pub pw: String,
    #[serde(default = "default_reveal_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_reveal_limit() -> i64 {
    2000
}

/// GET /api/hex/my-revealed?id=&pw=&limit=&offset= — 該玩家已揭露之 cell_id 列表。
pub async fn hex_my_revealed(Query(q): Query<HexMyRevealedQuery>) -> impl IntoResponse {
    let ok = db::verify_password(&q.id, &q.pw).unwrap_or(false);
    if !ok {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"身份驗證失敗"}))).into_response();
    }
    let ent = match db::get_entity(&q.id) {
        Ok(Some(e)) => e,
        _ => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"角色不存在"}))).into_response(),
    };
    if ent.kind != EntityKind::Player {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"僅玩家可查詢"}))).into_response();
    }
    let limit = q.limit.clamp(1, 50_000);
    let offset = q.offset.max(0);
    let total = match db::count_player_hex_revealed(&q.id) {
        Ok(n) => n,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": format!("{e}")})),
            )
                .into_response();
        }
    };
    let cell_ids = match db::list_player_hex_revealed(&q.id, limit, offset) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": format!("{e}")})),
            )
                .into_response();
        }
    };
    Json(serde_json::json!({
        "entity_id": q.id,
        "total": total,
        "limit": limit,
        "offset": offset,
        "cell_ids": cell_ids,
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct HexScoutReq {
    pub player_id: String,
    pub target_q: i32,
    pub target_r: i32,
}

/// POST /api/hex/scout — 偵查黑格，生成地形並釘死。
pub async fn hex_scout(Json(req): Json<HexScoutReq>) -> impl IntoResponse {
    let (hex_q, hex_r) = if let Some(st) = store::get_store() {
        let s = st.read();
        s.get_entity(&req.player_id)
            .map(|e| (e.hex_q, e.hex_r))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };

    let (Some(pq), Some(pr)) = (hex_q, hex_r) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"玩家無座標"}))).into_response();
    };

    let player_coord = HexCoord::new(pq, pr);
    let target_coord = HexCoord::new(req.target_q, req.target_r);
    if player_coord.distance(target_coord) != 1 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"偵查目標須相鄰"}))).into_response();
    }

    match hex_editor::ensure_world_cell_at(target_coord) {
        Ok((cell, scouted)) => Json(serde_json::json!({
            "ok": true,
            "cell": cell,
            "scouted": scouted
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("偵查失敗：{e}")})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct HexExploreReq {
    pub player_id: String,
    pub target_q: i32,
    pub target_r: i32,
}

/// POST /api/hex/explore — 精探完成，解鎖精煉層（移除 fogged）。
pub async fn hex_explore(Json(req): Json<HexExploreReq>) -> impl IntoResponse {
    let (hex_q, hex_r) = if let Some(st) = store::get_store() {
        let s = st.read();
        s.get_entity(&req.player_id)
            .map(|e| (e.hex_q, e.hex_r))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };

    if hex_q != Some(req.target_q) || hex_r != Some(req.target_r) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"玩家不在目標格，無法精探"})))
            .into_response();
    }

    let target_coord = HexCoord::new(req.target_q, req.target_r);
    match hex_editor::mark_cell_explored(target_coord) {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("探索失敗：{e}")})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct HexMoveReq {
    pub player_id: String,
    pub to_q: i32,
    pub to_r: i32,
}

/// POST /api/hex/move — 跨格移動，更新玩家座標。
pub async fn hex_move(Json(req): Json<HexMoveReq>) -> impl IntoResponse {
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"})))
            .into_response();
    };

    let (pq, pr) = {
        let s = st.read();
        s.get_entity(&req.player_id)
            .map(|e| (e.hex_q, e.hex_r))
            .unwrap_or((None, None))
    };

    let (Some(pq), Some(pr)) = (pq, pr) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"玩家無座標"}))).into_response();
    };

    let player_coord = HexCoord::new(pq, pr);
    let target_coord = HexCoord::new(req.to_q, req.to_r);

    if player_coord.distance(target_coord) != 1 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "移動距離過大，禁止瞬移"})),
        )
            .into_response();
    }

    let grid = match hex_editor::get_runtime_grid() {
        Some(g) => g,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "無法取得地圖網格"})),
            )
                .into_response()
        }
    };

    match grid.get(target_coord) {
        Some(cell) => {
            if !cell.terrain.walkable() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "目標地形不可通行"})),
                )
                    .into_response();
            }
        }
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "目標格尚未揭露，禁止移入"})),
            )
                .into_response();
        }
    };

    let mut s = st.write();

    if s.get_entity(&req.player_id).is_none() {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"玩家不存在"}))).into_response();
    }

    if let Err(e) = s.set_entity_hex(&req.player_id, req.to_q, req.to_r) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("移動更新失敗：{e}")})),
        )
            .into_response();
    }

    Json(serde_json::json!({"ok": true, "q": req.to_q, "r": req.to_r})).into_response()
}

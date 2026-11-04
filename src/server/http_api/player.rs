//! 玩家查詢（房間座標 + 361 拓撲 + 時序）。

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Json, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::db;
use crate::game::{TimePeriod, game_time_now};

// ── /api/player-room ──

#[derive(Deserialize)]
pub struct PlayerRoomQuery {
    id: Option<String>,
    pw: Option<String>,
}

#[derive(Serialize)]
pub struct PlayerRoomResponse {
    player_id: String,
    room_id: String,
}

pub async fn player_room(Query(q): Query<PlayerRoomQuery>) -> impl IntoResponse {
    let (Some(player_id), Some(password)) = (q.id, q.pw) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"需提供 id 與 pw 參數"})),
        )
            .into_response();
    };
    let ok = db::verify_password(&player_id, &password).unwrap_or(false);
    if !ok {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error":"身份驗證失敗"})),
        )
            .into_response();
    }
    let room_id = match db::get_entity_room(&player_id) {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"查詢房間失敗"})),
            )
                .into_response();
        }
    };
    Json(PlayerRoomResponse { player_id, room_id }).into_response()
}

// ── /api/topology ──

#[derive(Serialize)]
struct TopoNode {
    id: String,
    name: String,
    zone: i32,
    system: String,
    inlayable: bool,
}

#[derive(Serialize)]
struct TopoEdge {
    from: String,
    to: String,
    #[serde(rename = "type")]
    edge_type: String,
    cost: f64,
}

#[derive(Serialize)]
struct TopoResponse {
    player_id: String,
    nodes: Vec<TopoNode>,
    edges: Vec<TopoEdge>,
}

const HUB_NAMES: [&str; 20] = [
    "天樞", "驚雷", "震淵", "流雲", "弦月", "曜陽", "寒霜", "淵流", "萬象", "破陣", "鎮岳", "衡定",
    "穹壁", "生息", "歸元", "神淵", "明識", "千鈞", "無相", "破極",
];

const HUB_SYSTEMS: [&str; 20] = [
    "體", "敏", "體", "敏", "敏", "氣", "氣", "氣", "氣", "氣", "體", "氣", "體", "體", "氣", "氣",
    "敏", "氣", "敏", "敏",
];

const LOGIC_NAMES: [&str; 5] = ["起", "承", "轉", "協", "合"];
const PERIPHERAL_NAMES: [&str; 12] = [
    "探", "觸", "納", "蓄", "濾", "析", "融", "衍", "律", "束", "釋", "散",
];

fn node_id(n: usize) -> String {
    format!("N{n:03}")
}

fn build_topology_nodes() -> Vec<TopoNode> {
    let mut nodes = Vec::with_capacity(361);
    nodes.push(TopoNode {
        id: "N000".into(),
        name: "生之本源".into(),
        zone: 0,
        system: String::new(),
        inlayable: false,
    });
    for i in 1..=20 {
        nodes.push(TopoNode {
            id: node_id(i),
            name: HUB_NAMES[i - 1].into(),
            zone: 1,
            system: HUB_SYSTEMS[i - 1].into(),
            inlayable: true,
        });
    }
    for i in 1..=20usize {
        let sys = HUB_SYSTEMS[i - 1];
        for j in 1..=5usize {
            let nid = 20 + 5 * (i - 1) + j;
            nodes.push(TopoNode {
                id: node_id(nid),
                name: LOGIC_NAMES[j - 1].into(),
                zone: 2,
                system: sys.into(),
                inlayable: true,
            });
        }
    }
    for i in 1..=20usize {
        let sys = HUB_SYSTEMS[i - 1];
        for s in 1..=12usize {
            let nid = 120 + 12 * (i - 1) + s;
            nodes.push(TopoNode {
                id: node_id(nid),
                name: PERIPHERAL_NAMES[s - 1].into(),
                zone: 3,
                system: sys.into(),
                inlayable: true,
            });
        }
    }
    nodes
}

fn build_topology_edges(costs: &[f64]) -> Vec<TopoEdge> {
    let mut edges = Vec::with_capacity(760);
    let mut idx = 0usize;
    for i in 1..=20usize {
        edges.push(TopoEdge {
            from: "N000".into(),
            to: node_id(i),
            edge_type: "A".into(),
            cost: costs[idx],
        });
        idx += 1;
    }
    for i in 1..=20usize {
        for j in 1..=5usize {
            let blue = 20 + 5 * (i - 1) + j;
            edges.push(TopoEdge {
                from: node_id(i),
                to: node_id(blue),
                edge_type: "B".into(),
                cost: costs[idx],
            });
            idx += 1;
        }
    }
    let blue_green_map: [[usize; 3]; 5] =
        [[1, 2, 3], [3, 4, 5], [5, 6, 7], [8, 9, 10], [10, 11, 12]];
    for i in 1..=20usize {
        for (j, green_slots) in blue_green_map.iter().enumerate() {
            let blue_id = 20 + 5 * (i - 1) + (j + 1);
            for &gs in green_slots {
                let green_id = 120 + 12 * (i - 1) + gs;
                edges.push(TopoEdge {
                    from: node_id(blue_id),
                    to: node_id(green_id),
                    edge_type: "C".into(),
                    cost: costs[idx],
                });
                idx += 1;
            }
        }
    }
    for i in 1..=20usize {
        let base = 120 + 12 * (i - 1);
        for s in 1..=12usize {
            let from = base + s;
            let next = s % 12 + 1;
            let to = base + next;
            edges.push(TopoEdge {
                from: node_id(from),
                to: node_id(to),
                edge_type: "D".into(),
                cost: costs[idx],
            });
            idx += 1;
        }
    }
    for i in 1..=20usize {
        let base = 20 + 5 * (i - 1);
        for j in 1..=5usize {
            let from = base + j;
            let next = j % 5 + 1;
            let to = base + next;
            edges.push(TopoEdge {
                from: node_id(from),
                to: node_id(to),
                edge_type: "E".into(),
                cost: costs[idx],
            });
            idx += 1;
        }
    }
    edges
}

#[derive(Deserialize)]
pub struct TopologyQuery {
    id: Option<String>,
    pw: Option<String>,
}

pub async fn topology(Query(q): Query<TopologyQuery>) -> impl IntoResponse {
    let (Some(player_id), Some(password)) = (q.id, q.pw) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"需提供 id 與 pw 參數"})),
        )
            .into_response();
    };
    let ok = db::verify_password(&player_id, &password).unwrap_or(false);
    if !ok {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error":"身份驗證失敗"})),
        )
            .into_response();
    }
    let ent = match db::get_entity(&player_id) {
        Ok(Some(e)) => e,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error":"角色不存在"})),
            )
                .into_response();
        }
    };
    let Some(seed) = ent.soul_seed else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error":"角色無 SoulSeed"})),
        )
            .into_response();
    };
    let costs = db::expand_soul_seed_to_topology_costs(seed);
    let resp = TopoResponse {
        player_id,
        nodes: build_topology_nodes(),
        edges: build_topology_edges(&costs),
    };
    Json(resp).into_response()
}

// ── /api/time ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TimeQuery {
    #[allow(dead_code)]
    player_id: Option<String>,
}

#[derive(Serialize)]
pub struct TimeResponse {
    game_hour: i32,
    game_min: i32,
    time_period: String,
    time_period_idx: i32,
    server_unix: i64,
}

pub async fn game_time(
    State(state): State<crate::server::run::AppState>,
    Query(_q): Query<TimeQuery>,
) -> impl IntoResponse {
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let (_sec, hour, min, _day) = game_time_now(
        now_unix,
        state.cfg.game_time_epoch_unix,
        state.cfg.game_time_scale,
    );
    let tp = TimePeriod::from_hour(hour);
    Json(TimeResponse {
        game_hour: hour,
        game_min: min,
        time_period: tp.label().into(),
        time_period_idx: tp.index(),
        server_unix: now_unix,
    })
    .into_response()
}

// ── /api/climate ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ClimateQuery {
    region_id: Option<String>,
}

#[derive(Serialize)]
pub struct ClimateResponse {
    region_id: String,
    event_type: String,
    prose_seed: i64,
}

pub async fn climate(Query(q): Query<ClimateQuery>) -> impl IntoResponse {
    let region_id = q.region_id.unwrap_or_default();
    if region_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"需提供 region_id 參數"})),
        )
            .into_response();
    }
    match db::get_region_climate(&region_id) {
        Ok(Some(c)) => Json(ClimateResponse {
            region_id: c.region_id,
            event_type: c.event_type,
            prose_seed: c.prose_seed,
        })
        .into_response(),
        Ok(None) => Json(ClimateResponse {
            region_id,
            event_type: "clear".into(),
            prose_seed: 0,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

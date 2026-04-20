//! 啟動 Axum HTTP／WebSocket（對齊既有 `main` + `http_routes`）。

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::header;
use axum::response::{Html, IntoResponse};
use axum::routing::{delete, get, post, put};
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use uuid::Uuid;

use crate::config::Server;

use super::handler::{handle_message, WsConnection};
use super::http_api;
use super::hub::{Hub, SEND_BUFFER_SIZE};
use super::grid_manager;
use super::hex_editor;
use super::room_editor;
use super::session::SessionStore;
use super::simulation_loop::spawn_simulation_main_loop;

#[derive(Clone)]
pub struct AppState {
    pub hub: Arc<Hub>,
    pub sessions: Arc<SessionStore>,
    pub cfg: Server,
}

/// 綁定 `cfg.port` 並服務 `/ws` 與所有 HTTP API。
pub async fn run(cfg: Server) -> anyhow::Result<()> {
    let max = cfg.max_websocket_conn.max(1) as usize;
    let hub = Arc::new(Hub::new(max));
    let sessions = Arc::new(SessionStore::new());
    let state = AppState {
        hub,
        sessions,
        cfg,
    };
    super::hex_editor::init("data/hex/grid.json");
    grid_manager::init();
    spawn_simulation_main_loop(Arc::clone(&state.sessions), state.cfg.clone());
    let port = state.cfg.port.clone();

    let app = Router::new()
        // WebSocket
        .route("/ws", get(ws_upgrade))
        // 設計常數
        .route("/api/design-constants", get(http_api::design_constants))
        // 管理：清除所有實體
        .route("/api/admin/wipe-entities", post(http_api::wipe_entities))
        // 玩家當前房間
        .route("/api/player-room", get(http_api::player_room))
        // 地圖檢視器資料
        .route("/data/rooms.json", get(http_api::rooms_data))
        // 星盤拓撲
        .route("/api/topology", get(http_api::topology))
        .route("/api/hex/view", get(http_api::hex_view))
        .route("/api/hex/player-reveal", post(http_api::hex_player_reveal))
        .route("/api/hex/my-revealed", get(http_api::hex_my_revealed))
        .route("/api/hex/scout", post(http_api::hex_scout))
        .route("/api/hex/explore", post(http_api::hex_explore))
        .route("/api/hex/move", post(http_api::hex_move))
        // 房間管理 CRUD
        .route("/api/rooms", get(http_api::list_rooms).post(http_api::create_room))
        .route("/api/rooms/{id}", get(http_api::get_room_admin).put(http_api::update_room).delete(http_api::delete_room))
        .route("/api/rooms/{id}/rename", post(http_api::rename_room))
        .route("/api/rooms/{id}/exits", post(http_api::add_exit))
        .route("/api/rooms/{from_id}/exits/{direction}", delete(http_api::remove_exit))
        // 房間心智圖編輯器
        .route("/api/room-editor/graph", get(room_editor::graph))
        .route("/api/room-editor/room", post(room_editor::create))
        .route("/api/room-editor/room/{id}", put(room_editor::update).delete(room_editor::delete))
        .route("/api/room-editor/link", post(room_editor::link_create).delete(room_editor::link_delete))
        .route("/api/room-editor/layout", put(room_editor::layout))
        .route("/api/room-editor/reload", post(room_editor::reload))
        .route("/api/room-editor/groups", get(room_editor::groups_get).post(room_editor::groups_post))
        // 地圖編輯器（六角格網）
        .route("/api/hex/grid", get(hex_editor::grid_get))
        .route("/api/hex/reveal", post(hex_editor::reveal_post))
        .route("/api/hex/reveal-region", post(hex_editor::reveal_region_post))
        .route("/api/hex/world-seed", put(hex_editor::world_seed_put))
        .route("/api/hex/cell", put(hex_editor::cell_put))
        .route("/api/hex/cells", put(hex_editor::cells_put))
        .route("/api/hex/cell/{q}/{r}", delete(hex_editor::cell_delete))
        .route("/api/hex/wall", put(hex_editor::wall_put))
        .route("/api/hex/portal", post(hex_editor::portal_post))
        .route(
            "/api/hex/transport-edge",
            post(hex_editor::transport_edge_post),
        )
        .route("/api/hex/save", post(hex_editor::save))
        .route("/api/hex/reload", post(hex_editor::reload))
        .route("/api/hex/path", get(hex_editor::path_get))
        .route("/api/hex/neighbors/{q}/{r}", get(hex_editor::neighbors_get))
        // HTML 頁面路由
        .route("/map_viewer", get(serve_map_viewer))
        .route("/star_chart", get(serve_star_chart))
        .route("/dashboard", get(serve_dashboard))
        .route("/admin", get(serve_admin))
        // Leptos 地圖編輯器（WASM）
        .nest_service("/hex-editor", ServeDir::new("editor-leptos/dist"))
        // 靜態設定 JSON（terrain_ambience 等）
        .nest_service("/data/config", ServeDir::new("data/config"))
        .with_state(state.clone())
        // 靜態檔案服務 — web/ 目錄（fallback，對齊既有 `http.FileServer`）
        .fallback_service(
            ServeDir::new("web")
                .precompressed_gzip()
        )
        .layer(
            SetResponseHeaderLayer::overriding(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("no-cache, max-age=0, must-revalidate"),
            ),
        );

    let addr: SocketAddr = format!("0.0.0.0:{port}")
        .parse()
        .map_err(|e| anyhow::anyhow!("bad port: {e}"))?;
    tracing::info!("Rust server listening on http://{addr} (WebSocket /ws)");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(st): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, st))
}

/// HTML 頁面：讀取 web/ 下的 .html 檔並回傳（對齊既有 的 http.ServeFile）。
async fn serve_html_page(filename: &str) -> impl IntoResponse {
    let path = PathBuf::from("web").join(filename);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8"),
             (header::CACHE_CONTROL, "no-cache, max-age=0, must-revalidate")],
            Html(content),
        ).into_response(),
        Err(_) => (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

async fn serve_map_viewer() -> impl IntoResponse { serve_html_page("map_viewer.html").await }
async fn serve_star_chart() -> impl IntoResponse { serve_html_page("star_chart.html").await }
async fn serve_dashboard() -> impl IntoResponse { serve_html_page("dashboard.html").await }
async fn serve_admin() -> impl IntoResponse { serve_html_page("admin.html").await }

async fn handle_socket(mut socket: WebSocket, st: AppState) {
    let conn_id = Uuid::new_v4();
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(SEND_BUFFER_SIZE);
    if !st.hub.register(conn_id, tx.clone()) {
        let _ = socket.close().await;
        return;
    }
    let (mut sink, mut stream) = socket.split();
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let text = String::from_utf8_lossy(&msg).into_owned();
            // 10 秒 send timeout——client TCP 半關時防止 write_task 無限卡
            // 卡住會使 mpsc channel 256 buffer 滿，blocking_send 全卡 → spawn_blocking thread 耗盡 → server 死鎖
            match tokio::time::timeout(
                std::time::Duration::from_secs(10),
                sink.send(Message::Text(text.into())),
            ).await {
                Ok(Ok(())) => {}
                _ => break,  // timeout / error 都中止 write side
            }
        }
    });
    let conn = Arc::new(WsConnection {
        conn_id,
        tx: tx.clone(),
        player_id: RwLock::new(None),
        sessions: st.sessions.clone(),
        hub: st.hub.clone(),
        cfg: st.cfg.clone(),
    });
    // 120 秒沒任何訊息視為殭屍連線（手機/瀏覽器半關連線不會主動送 close frame），主動砍
    // 前端 heartbeat 每 30 秒一次 ping，正常應有 pong 往返不會 timeout
    loop {
        let next = tokio::time::timeout(std::time::Duration::from_secs(120), stream.next()).await;
        let msg = match next {
            Err(_) => { tracing::info!("ws idle timeout, closing conn_id={conn_id}"); break; }
            Ok(None) => break,
            Ok(Some(Err(_))) => break,
            Ok(Some(Ok(m))) => m,
        };
        match msg {
            Message::Text(t) => {
                let c = conn.clone();
                let bytes = t.as_bytes().to_vec();
                let jh = tokio::task::spawn_blocking(move || {
                    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        handle_message(&c, &bytes);
                    }));
                    if let Err(e) = r {
                        let pmsg = if let Some(s) = e.downcast_ref::<&str>() { s.to_string() }
                            else if let Some(s) = e.downcast_ref::<String>() { s.clone() }
                            else { "unknown panic".to_string() };
                        tracing::error!("[handle_message PANIC] {pmsg}");
                    }
                }).await;
                if let Err(e) = jh { tracing::error!("[handle_message JOIN ERR] {e}"); }
            }
            Message::Binary(b) => {
                let c = conn.clone();
                let jh = tokio::task::spawn_blocking(move || {
                    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        handle_message(&c, &b);
                    }));
                    if let Err(e) = r {
                        let pmsg = if let Some(s) = e.downcast_ref::<&str>() { s.to_string() }
                            else if let Some(s) = e.downcast_ref::<String>() { s.clone() }
                            else { "unknown panic".to_string() };
                        tracing::error!("[handle_message PANIC] {pmsg}");
                    }
                }).await;
                if let Err(e) = jh { tracing::error!("[handle_message JOIN ERR] {e}"); }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    if let Ok(g) = conn.player_id.read()
        && let Some(ref pid) = *g
    {
        conn.sessions.remove_if_connection(pid, conn_id);
    }
    st.hub.unregister(conn_id);
    write_task.abort();
}

//! 啟動 Axum HTTP／WebSocket（對齊 Go `main` + `/ws`）。

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::config::Server;

use super::handler::{handle_message, WsConnection};
use super::hub::{Hub, SEND_BUFFER_SIZE};
use super::session::SessionStore;
use super::simulation_loop::spawn_simulation_main_loop;

#[derive(Clone)]
pub struct AppState {
    pub hub: Arc<Hub>,
    pub sessions: Arc<SessionStore>,
    pub cfg: Server,
}

/// 綁定 `cfg.port` 並服務 `/ws`。
pub async fn run(cfg: Server) -> anyhow::Result<()> {
    let max = cfg.max_websocket_conn.max(1) as usize;
    let hub = Arc::new(Hub::new(max));
    let sessions = Arc::new(SessionStore::new());
    let state = AppState {
        hub,
        sessions,
        cfg,
    };
    spawn_simulation_main_loop(Arc::clone(&state.sessions), state.cfg.clone());
    let port = state.cfg.port.clone();
    let app = Router::new()
        .route("/ws", get(ws_upgrade))
        .with_state(state.clone());
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
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
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
    while let Some(Ok(msg)) = stream.next().await {
        match msg {
            Message::Text(t) => {
                let c = conn.clone();
                let bytes = t.as_bytes().to_vec();
                let _ = tokio::task::spawn_blocking(move || {
                    handle_message(&c, &bytes);
                })
                .await;
            }
            Message::Binary(b) => {
                let c = conn.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    handle_message(&c, &b);
                })
                .await;
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

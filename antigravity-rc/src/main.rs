use axum::{
    extract::{DefaultBodyLimit, Multipart, ws::{Message as WsMessage, WebSocket, WebSocketUpgrade}},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use chrono::Local;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use futures_util::{SinkExt, StreamExt};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ChatMessage {
    id: String,
    role: String,
    content: String,
    timestamp: String,
    index: Option<i32>,
}

static MSG_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static BOOT_TS: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
fn next_msg_id() -> String {
    let boot = *BOOT_TS.get_or_init(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });
    let seq = MSG_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{}-{}", boot, seq)
}

#[derive(Serialize, Deserialize)]
struct ActionResult {
    status: String,
    detail: String,
}

struct AppState {
    tx: broadcast::Sender<ChatMessage>,
    history: RwLock<Vec<ChatMessage>>, // Store last N messages
}

#[tokio::main]
async fn main() {
    let upload_dir = PathBuf::from("remote_uploads");
    if !upload_dir.exists() {
        fs::create_dir_all(&upload_dir).await.unwrap();
    }

    let (tx, _rx) = broadcast::channel(100);
    let state = Arc::new(AppState { 
        tx, 
        history: RwLock::new(Vec::with_capacity(50)) 
    });

    let state_for_task = state.clone();
    tokio::spawn(async move {
        use tokio_tungstenite::tungstenite::protocol::Message;
        use futures_util::{SinkExt as _, StreamExt as _};

        let mut last_content = String::new();
        let mut ws_stream: Option<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>> = None;
        let mut cmd_id: u64 = 1;

        loop {
            // 1. Ensure we have a CDP connection
            if ws_stream.is_none() {
                match discover_and_connect().await {
                    Ok(stream) => {
                        eprintln!("[ARC] CDP connected");
                        ws_stream = Some(stream);
                    }
                    Err(e) => {
                        eprintln!("[ARC] CDP connect failed: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        continue;
                    }
                }
            }

            // 2. Send Runtime.evaluate and read response (with ID matching)
            let stream = ws_stream.as_mut().unwrap();
            cmd_id += 1;
            let expected_id = cmd_id;
            let script = get_extraction_script();
            let command = json!({ "id": expected_id, "method": "Runtime.evaluate", "params": { "expression": script, "returnByValue": true } });

            let result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                async {
                    stream.send(Message::Text(command.to_string())).await?;
                    // 用 ID 配對回覆，跳過 CDP 推送的 event
                    loop {
                        match stream.next().await {
                            Some(Ok(Message::Text(resp_text))) => {
                                let resp: serde_json::Value = match serde_json::from_str(&resp_text) {
                                    Ok(v) => v,
                                    Err(_) => continue,
                                };
                                // CDP event（有 method 無 id）→ 跳過
                                if resp.get("method").is_some() && resp.get("id").is_none() {
                                    continue;
                                }
                                // 確認是我們送出的命令的回覆
                                if resp.get("id").and_then(|v| v.as_u64()) == Some(expected_id) {
                                    return Ok::<_, anyhow::Error>(resp);
                                }
                                // 不是我們的 id → 跳過（可能是舊命令的回覆）
                                continue;
                            }
                            Some(Ok(_)) => continue, // Binary/Ping/Pong
                            Some(Err(e)) => anyhow::bail!("CDP WS error: {}", e),
                            None => anyhow::bail!("CDP WS closed"),
                        }
                    }
                }
            ).await;

            match result {
                Ok(Ok(resp)) => {
                    if let Some(res) = resp["result"]["result"]["value"].as_object() {
                        let content = res.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let _index = res.get("index").and_then(|v| v.as_i64()).map(|v| v as i32);
                        
                        if !content.trim().is_empty() && content != last_content {
                            last_content = content.to_string();

                            // AI 回覆的 index = user 訊息計數，同一輪對話共享 index
                            let mut hist = state_for_task.history.write().await;
                            let user_count = hist.iter().filter(|m| m.role == "user").count() as i32;
                            let ai_index = user_count; // 每次 user 發言後 +1，AI streaming 不變

                            let msg = ChatMessage {
                                id: next_msg_id(),
                                role: "ai".into(),
                                content: content.to_string(),
                                timestamp: Local::now().format("%H:%M").to_string(),
                                index: Some(ai_index),
                            };

                            // 覆蓋同 index 的 AI 訊息（streaming 更新），否則新增
                            let mut replaced = false;
                            for m in hist.iter_mut().rev() {
                                if m.role == "ai" && m.index == Some(ai_index) {
                                    m.content = content.to_string();
                                    m.timestamp = msg.timestamp.clone();
                                    m.id = msg.id.clone();
                                    replaced = true;
                                    break;
                                }
                                if m.role == "user" { break; }
                            }
                            if !replaced {
                                if hist.len() >= 50 { hist.remove(0); }
                                hist.push(msg.clone());
                            }

                            let _ = state_for_task.tx.send(msg);
                        }
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("[ARC] CDP command error: {} — reconnecting", e);
                    ws_stream = None;
                }
                Err(_) => {
                    eprintln!("[ARC] CDP timeout — reconnecting");
                    ws_stream = None;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        }
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/poll", get(poll_handler))
        .route("/chat", post(handle_chat))
        .fallback_service(ServeDir::new("antigravity-rc/static"))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 2026));
    println!("{}", "=".repeat(40));
    println!("Antigravity RC PRO (雙向對話版) 已啟動");
    println!("監聽地址: http://{}", addr);
    println!("{}", "=".repeat(40));

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    println!("[ARC] New WS client connected");
    let (mut sender, _) = socket.split();
    let mut rx = state.tx.subscribe();

    // Send entire history on connect — 用 block scope 確保 read lock 立即釋放
    {
        let hist = state.history.read().await;
        for cached in hist.iter() {
            if let Ok(msg_json) = serde_json::to_string(cached) {
                let _ = sender.send(WsMessage::Text(msg_json)).await;
            }
        }
    }

    while let Ok(msg) = rx.recv().await {
        if let Ok(msg_json) = serde_json::to_string(&msg) {
            if sender.send(WsMessage::Text(msg_json)).await.is_err() {
                println!("[ARC] WS client disconnected");
                break;
            }
        }
    }
}

async fn poll_handler(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    let hist = state.history.read().await;
    Json(json!({ "history": *hist }))
}

async fn handle_chat(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut text = String::new();
    let mut image_path = None;
    let mut selected_model = None;

    while let Ok(field_result) = multipart.next_field().await {
        if let Some(field) = field_result {
            let name = field.name().unwrap_or("").to_string();
            if name == "text" {
                if let Ok(data) = field.text().await {
                    text = data;
                }
            } else if name == "model" {
                if let Ok(data) = field.text().await {
                    if !data.is_empty() {
                        selected_model = Some(data);
                    }
                }
            } else if name == "file" {
                let filename = field.file_name().unwrap_or("image.png").to_string();
                let path = PathBuf::from("remote_uploads").join(format!("{}_{}", Local::now().format("%H%M%S"), filename));
                if let Ok(data) = field.bytes().await {
                    if let Ok(mut file) = File::create(&path).await {
                        let _ = file.write_all(&data).await;
                        image_path = Some(fs::canonicalize(&path).await.unwrap_or(path));
                    }
                }
            }
        } else {
            break;
        }
    }

    if text.is_empty() && image_path.is_none() {
        return Json(ActionResult { status: "error".into(), detail: "Empty message".into() });
    }

    println!("[ARC] Received message: {} (Model: {:?})", text, selected_model);

    let injection_text = if let Some(ref path) = image_path {
        format!("@{} \n\n主管指令: {}", path.display(), text)
    } else {
        text.clone()
    };

    match inject_to_cursor(&injection_text, selected_model.as_deref()).await {
        Ok(res) => {
            println!("[ARC] Injection Result: {}", res);

            let msg = ChatMessage {
                id: next_msg_id(),
                role: "user".into(),
                content: text,
                timestamp: Local::now().format("%H:%M").to_string(),
                index: None,
            };
            let mut hist = state.history.write().await;
            if hist.len() >= 50 { hist.remove(0); }
            hist.push(msg.clone());
            let _ = state.tx.send(msg);
            
            Json(ActionResult { status: "ok".into(), detail: "Injected".into() })
        }
        Err(e) => {
            eprintln!("[ARC] Injection Failed: {}", e);
            Json(ActionResult { status: "error".into(), detail: e.to_string() })
        }
    }
}

/// Discover CDP target and open a persistent WebSocket connection
async fn discover_and_connect() -> anyhow::Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>> {
    use tokio_tungstenite::connect_async;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()?;
    let targets: Vec<serde_json::Value> = client
        .get("http://127.0.0.1:9222/json")
        .send().await?
        .json().await?;

    // Match by workbench URL (CursorRemote pattern) or title
    let target = targets.into_iter()
        .find(|t| t["type"] == "page" && (
            t["url"].as_str().unwrap_or("").contains("workbench") ||
            t["title"].as_str().unwrap_or("").contains("singularity_world")
        ))
        .ok_or_else(|| anyhow::anyhow!("No IDE page target found"))?;

    let ws_url = target["webSocketDebuggerUrl"].as_str()
        .ok_or_else(|| anyhow::anyhow!("No webSocketDebuggerUrl"))?;

    eprintln!("[ARC] Connecting to: {} ({})", target["title"], ws_url);

    let (ws_stream, _) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        connect_async(ws_url)
    ).await.map_err(|_| anyhow::anyhow!("connect_async timeout"))??;

    Ok(ws_stream)
}

/// JS extraction script — 適配 Antigravity IDE 的 DOM 結構
fn get_extraction_script() -> &'static str {
    r#"
        (function() {
            try {
                // Antigravity 結構：scroll area 內，AI 正文元素統一是 class="px-2 py-1"
                const scrollArea = document.querySelector('.h-full.overflow-y-auto.min-h-0');
                if (!scrollArea) return null;

                // 找所有 AI 正文區塊（px-2 py-1），取最後一個
                const replies = scrollArea.querySelectorAll('.px-2.py-1');
                if (replies.length === 0) return null;

                const last = replies[replies.length - 1];
                const text = (last.innerText || '').trim();
                if (!text) return null;

                return { text: text, index: replies.length };
            } catch(e) { return null; }
        })();
    "#
}

async fn inject_to_cursor(content: &str, model: Option<&str>) -> anyhow::Result<serde_json::Value> {
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(3)).build()?;
    let targets: Vec<serde_json::Value> = client.get("http://127.0.0.1:9222/json").send().await?.json().await?;
    let target = targets.into_iter()
        .find(|t| t["type"] == "page" && (
            t["url"].as_str().unwrap_or("").contains("workbench") ||
            t["title"].as_str().unwrap_or("").contains("singularity_world")
        ))
        .ok_or_else(|| anyhow::anyhow!("No IDE page target found"))?;
    
    let ws_url = target["webSocketDebuggerUrl"].as_str().ok_or_else(|| anyhow::anyhow!("No WS URL"))?;
    let (mut ws_stream, _) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        connect_async(ws_url)
    ).await.map_err(|_| anyhow::anyhow!("inject connect_async timeout"))??;

    if let Some(m) = model {
        let switch_script = format!(
            r#"(function() {{
                try {{
                    function simulateClick(el) {{
                        if (!el) return;
                        const rect = el.getBoundingClientRect();
                        const x = rect.left + rect.width / 2;
                        const y = rect.top + rect.height / 2;
                        const opts = {{ bubbles: true, cancelable: true, clientX: x, clientY: y }};
                        el.dispatchEvent(new PointerEvent('pointerdown', opts));
                        el.dispatchEvent(new MouseEvent('mousedown', opts));
                        el.dispatchEvent(new MouseEvent('mouseup', opts));
                        el.dispatchEvent(new PointerEvent('pointerup', opts));
                        el.dispatchEvent(new MouseEvent('click', opts));
                        el.focus && el.focus();
                    }}

                    const modelStr = "{}";
                    // 找出目前顯示模型的按鈕
                    let targetPill = null;
                    const elements = document.querySelectorAll('div, span, button, a, [role="button"]');
                    for (let i = elements.length - 1; i >= 0; i--) {{
                        let el = elements[i];
                        let t = el.textContent || "";
                        if ((t.includes('Gemini') || t.includes('Claude') || t.includes('gpt-') || t.includes('o3-')) && 
                            el.children.length <= 2 && el.offsetParent !== null && t.length < 30) {{
                            targetPill = el;
                            break;
                        }}
                    }}

                    if (targetPill) {{
                        simulateClick(targetPill); // 打開菜單
                        setTimeout(() => {{
                            // 在菜單中尋找指定模型
                            const optionsList = document.querySelectorAll('div, span, button, [role="option"], li');
                            let targetOpt = null;
                            for (let j = optionsList.length - 1; j >= 0; j--) {{
                                let opt = optionsList[j];
                                if (opt.textContent.toLowerCase().includes(modelStr.toLowerCase()) && 
                                    opt.offsetParent !== null && 
                                    opt.textContent.length < 40 &&
                                    (opt.children.length <= 1 || opt.getAttribute('role') === 'option')) {{
                                    targetOpt = opt;
                                    break;
                                }}
                            }}
                            if (targetOpt) {{
                                simulateClick(targetOpt);
                            }}
                        }}, 300);
                    }}
                }} catch(e) {{}}
            }})();"#,
            m
        );
        let switch_cmd = json!({ "id": 99, "method": "Runtime.evaluate", "params": { "expression": switch_script } });
        let _ = ws_stream.send(Message::Text(switch_cmd.to_string())).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await; // 等待菜單點擊與渲染
    }

    // 輔助：送 CDP 命令並用 ID 配對回覆
    async fn cdp_send(
        ws: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        id: u64,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let cmd = json!({ "id": id, "method": method, "params": params });
        ws.send(Message::Text(cmd.to_string())).await?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match tokio::time::timeout_at(deadline, ws.next()).await {
                Ok(Some(Ok(Message::Text(raw)))) => {
                    if let Ok(resp) = serde_json::from_str::<serde_json::Value>(&raw) {
                        if resp.get("id").and_then(|v| v.as_u64()) == Some(id) {
                            return Ok(resp);
                        }
                    }
                }
                Ok(Some(Ok(_))) => continue,
                _ => anyhow::bail!("CDP response timeout for id {}", id),
            }
        }
    }

    // Step 1: Focus input
    let focus_script = r#"(function() {
        const input = document.querySelector('[aria-label="Message input"]');
        if (!input) return { ok: false };
        input.scrollIntoView({ block: 'center', behavior: 'instant' });
        input.focus();
        input.click();
        return { ok: true };
    })()"#;
    let resp = cdp_send(&mut ws_stream, 1, "Runtime.evaluate",
        json!({ "expression": focus_script, "returnByValue": true })).await?;
    let focused = resp["result"]["result"]["value"]["ok"].as_bool().unwrap_or(false);
    if !focused {
        anyhow::bail!("Chat input not found");
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Step 2: Ctrl+A → Backspace 清除殘留
    cdp_send(&mut ws_stream, 2, "Input.dispatchKeyEvent",
        json!({"type":"keyDown","key":"a","code":"KeyA","windowsVirtualKeyCode":65,"modifiers":2})).await?;
    cdp_send(&mut ws_stream, 3, "Input.dispatchKeyEvent",
        json!({"type":"keyUp","key":"a","code":"KeyA","windowsVirtualKeyCode":65})).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    cdp_send(&mut ws_stream, 4, "Input.dispatchKeyEvent",
        json!({"type":"keyDown","key":"Backspace","code":"Backspace","windowsVirtualKeyCode":8})).await?;
    cdp_send(&mut ws_stream, 5, "Input.dispatchKeyEvent",
        json!({"type":"keyUp","key":"Backspace","code":"Backspace","windowsVirtualKeyCode":8})).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Step 3: Insert text
    cdp_send(&mut ws_stream, 6, "Input.insertText",
        json!({"text": content})).await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Step 4: Enter 送出
    cdp_send(&mut ws_stream, 7, "Input.dispatchKeyEvent",
        json!({"type":"keyDown","key":"Enter","code":"Enter","windowsVirtualKeyCode":13})).await?;
    cdp_send(&mut ws_stream, 8, "Input.dispatchKeyEvent",
        json!({"type":"keyUp","key":"Enter","code":"Enter","windowsVirtualKeyCode":13})).await?;

    Ok(json!({"status": "injected"}))
}

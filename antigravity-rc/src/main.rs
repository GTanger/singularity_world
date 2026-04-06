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
use tokio::sync::broadcast;
use futures_util::{SinkExt, StreamExt};

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
    timestamp: String,
}

#[derive(Serialize, Deserialize)]
struct ActionResult {
    status: String,
    detail: String,
}

struct AppState {
    tx: broadcast::Sender<ChatMessage>,
}

#[tokio::main]
async fn main() {
    let upload_dir = PathBuf::from("remote_uploads");
    if !upload_dir.exists() {
        fs::create_dir_all(&upload_dir).await.unwrap();
    }

    let (tx, _rx) = broadcast::channel(100);
    let state = Arc::new(AppState { tx });

    let state_for_task = state.clone();
    tokio::spawn(async move {
        let mut last_content = String::new();
        loop {
            if let Ok(new_msgs) = poll_ide_for_responses().await {
                for msg in new_msgs {
                    if msg.content != last_content {
                        last_content = msg.content.clone();
                        let _ = state_for_task.tx.send(msg);
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        }
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
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
    let (mut sender, _) = socket.split();
    let mut rx = state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if let Ok(msg_json) = serde_json::to_string(&msg) {
            if sender.send(WsMessage::Text(msg_json)).await.is_err() {
                break;
            }
        }
    }
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

            let _ = state.tx.send(ChatMessage {
                role: "user".into(),
                content: text,
                timestamp: Local::now().format("%H:%M").to_string(),
            });
            Json(ActionResult { status: "ok".into(), detail: "Injected".into() })
        }
        Err(e) => {
            eprintln!("[ARC] Injection Failed: {}", e);
            Json(ActionResult { status: "error".into(), detail: e.to_string() })
        }
    }
}


async fn poll_ide_for_responses() -> anyhow::Result<Vec<ChatMessage>> {
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

    let client = reqwest::Client::new();
    let targets: Vec<serde_json::Value> = client.get("http://127.0.0.1:9222/json").send().await?.json().await?;
    let target = targets.into_iter().find(|t| t["title"].as_str().unwrap_or("").contains("singularity_world") && t["type"] == "page")
        .ok_or_else(|| anyhow::anyhow!("Cursor not found"))?;

    let ws_url = target["webSocketDebuggerUrl"].as_str().ok_or_else(|| anyhow::anyhow!("No active WS session"))?;
    let (mut ws_stream, _) = connect_async(ws_url).await?;

    let script = r#"
        (function() {
            try {
                let messageContainers = [];
                function scanForContainers(root) {
                    if (!root) return;
                    if (root.nodeType === Node.ELEMENT_NODE) {
                        let cn = root.className;
                        if (typeof cn === 'string') {
                            if (cn.includes('chat-bubble') || cn.includes('markdown') || cn.includes('content') || cn.includes('message')) {
                                let text = root.innerText || root.textContent || "";
                                // Exclude the input box UI area!
                                if (text.trim().length > 10 && 
                                    !text.includes('Ask anything') && 
                                    !text.includes('0 Files With Changes') &&
                                    !text.includes('Review Changes')) {
                                    messageContainers.push(root);
                                }
                            }
                        }
                    }
                    if (root.shadowRoot) scanForContainers(root.shadowRoot);
                    if (root.childNodes) {
                        for (let i = 0; i < root.childNodes.length; i++) {
                            const node = root.childNodes[i];
                            if (node.className && typeof node.className === 'string' && 
                                (node.className.includes('monaco-editor') || node.className.includes('decorationsOverviewRuler'))) continue;
                            scanForContainers(node);
                        }
                    }
                }
                
                scanForContainers(document.body);
                
                if (messageContainers.length > 0) {
                    let validTexts = messageContainers.map(el => el.innerText || el.textContent || "").map(t => t.trim());
                    let uniqueTexts = validTexts.filter((t, i, arr) => arr.indexOf(t) === i);
                    
                    let finalArr = uniqueTexts.filter(t => 
                        !t.includes('ACTIVE_MODEL') && 
                        !t.includes('Prioritizing Tool Usage') && 
                        !t.includes('CRITICAL INSTRUCTION') && 
                        t.length > 5
                    );
                    if (finalArr.length > 0) {
                        return [finalArr[finalArr.length - 1]];
                    }
                }
                return ["ERROR_NO_CONVO_FOUND"];
            } catch(e) { return [e.toString()]; }
        })();
    "#;

    let command = json!({ "id": 1, "method": "Runtime.evaluate", "params": { "expression": script, "returnByValue": true } });
    ws_stream.send(Message::Text(command.to_string())).await?;

    let mut messages = vec![];
    if let Some(Ok(Message::Text(resp_text))) = ws_stream.next().await {
        let resp: serde_json::Value = serde_json::from_str(&resp_text)?;
        if let Some(value) = resp["result"]["result"]["value"].as_array() {
            for v in value {
                if let Some(content) = v.as_str() {
                    if !content.trim().is_empty() {
                        messages.push(ChatMessage {
                            role: "ai".into(),
                            content: content.to_string(),
                            timestamp: Local::now().format("%H:%M").to_string(),
                        });
                    }
                }
            }
        } else {
            // Log what was actually returned to debug selector issues
            println!("[ARC] Polling raw value: {:?}", resp["result"]["result"]["value"]);
        }
    }
    Ok(messages)
}

async fn inject_to_cursor(content: &str, model: Option<&str>) -> anyhow::Result<serde_json::Value> {
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
    let client = reqwest::Client::new();
    let targets: Vec<serde_json::Value> = client.get("http://127.0.0.1:9222/json").send().await?.json().await?;
    let target = targets.into_iter().find(|t| t["title"].as_str().unwrap_or("").contains("singularity_world") && t["type"] == "page")
        .ok_or_else(|| anyhow::anyhow!("Cursor not found"))?;
    
    let ws_url = target["webSocketDebuggerUrl"].as_str().ok_or_else(|| anyhow::anyhow!("No WS URL"))?;
    let (mut ws_stream, _) = connect_async(ws_url).await?;

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

    let focus_script = r#"(function() {
        const inputs = document.querySelectorAll('textarea, [contenteditable="true"], [role="textbox"]');
        const chatInput = Array.from(inputs).find(i => 
            i.ariaLabel?.includes('Message input') || 
            i.getAttribute('aria-label')?.includes('Message input') ||
            i.placeholder?.includes('Ask') || 
            i.ariaLabel?.includes('Chat')
        );
        if (chatInput) {
            chatInput.focus();
            return { status: "focused", tag: chatInput.tagName };
        }
        return { status: "not_found" };
    })();"#;

    let focus_cmd = json!({ "id": 1, "method": "Runtime.evaluate", "params": { "expression": focus_script, "returnByValue": true } });
    ws_stream.send(Message::Text(focus_cmd.to_string())).await?;

    // Wait for focus to complete
    if let Some(Ok(Message::Text(_))) = ws_stream.next().await {
        // Now use CDP Input API to simulate real user typing & enter
        let insert_text_cmd = json!({
            "id": 2,
            "method": "Input.insertText",
            "params": {
                "text": content
            }
        });
        ws_stream.send(Message::Text(insert_text_cmd.to_string())).await?;
        let _ = ws_stream.next().await; // wait for insert text

        let enter_cmd = json!({
            "id": 3,
            "method": "Input.dispatchKeyEvent",
            "params": {
                "type": "keyDown",
                "windowsVirtualKeyCode": 13,
                "key": "Enter",
                "code": "Enter",
                "text": "\r",
                "unmodifiedText": "\r"
            }
        });
        ws_stream.send(Message::Text(enter_cmd.to_string())).await?;
        let _ = ws_stream.next().await; // wait for enter keyDown

        let enter_up_cmd = json!({
            "id": 4,
            "method": "Input.dispatchKeyEvent",
            "params": {
                "type": "keyUp",
                "windowsVirtualKeyCode": 13,
                "key": "Enter",
                "code": "Enter"
            }
        });
        ws_stream.send(Message::Text(enter_up_cmd.to_string())).await?;
        let _ = ws_stream.next().await; // wait for enter keyUp

        // Try clicking send button to ensure submission
        let click_script = r#"(function() {
            setTimeout(() => {
                const buttons = document.querySelectorAll('button, [role="button"], .submit-button, [aria-label*="Send"], .a-icon-send, [title*="Send"]');
                const sendBtn = Array.from(buttons).find(b => 
                    b.innerHTML.includes('svg') || 
                    (b.innerText && b.innerText.includes('Send')) || 
                    b.ariaLabel?.includes('Send') ||
                    b.className.includes('send') ||
                    b.getAttribute('title')?.includes('Send')
                );
                if (sendBtn) sendBtn.click();
            }, 100);
            return { status: "cdp_injected_and_clicked" };
        })();"#;
        let click_cmd = json!({ "id": 5, "method": "Runtime.evaluate", "params": { "expression": click_script, "returnByValue": true } });
        ws_stream.send(Message::Text(click_cmd.to_string())).await?;
        
        if let Some(Ok(Message::Text(res))) = ws_stream.next().await {
            let resp: serde_json::Value = serde_json::from_str(&res)?;
            return Ok(resp["result"]["result"]["value"].clone());
        }
    }
    
    Ok(json!({"error": "No valid injection response"}))
}

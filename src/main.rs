// 奇點世界 — Rust 後端入口。
// 啟動 HTTP/WebSocket 伺服器，載入 JSON store，啟動遊戲主迴圈。

#[tokio::main]
async fn main() {
    // 初始化日誌
    tracing_subscriber::fmt::init();
    tracing::info!("singularity_world starting");
}

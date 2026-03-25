#![forbid(unsafe_code)]
// 奇點世界 — 模組宣告。

pub mod model;
pub mod entity;
pub mod config;
pub mod event;
pub mod gametext;
pub mod store;
pub mod db;
pub mod game;
pub mod combat;
pub mod economy;
pub mod npc;
pub mod npcnpc;
pub mod ai;
pub mod world;
pub mod server;
pub mod roomcheck;

/// 啟動 WebSocket 伺服器（需先 `store::init`、建議先 `gametext::must_load`）。
pub async fn run_server(cfg: crate::config::Server) -> anyhow::Result<()> {
    server::run(cfg).await
}

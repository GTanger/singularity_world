#![forbid(unsafe_code)]
// 奇點世界 — 模組宣告。

pub mod ai;
pub mod combat;
pub mod config;
pub mod db;
pub mod economy;
pub mod entity;
pub mod event;
pub mod game;
pub mod gametext;
pub mod grid;
pub mod living_world;
pub mod model;
pub mod npc;
pub mod npcnpc;
pub mod pg;
pub mod roomcheck;
pub mod server;
pub mod store;
pub mod world;
pub mod world_state;
pub mod worldgen;

/// 啟動 WebSocket 伺服器（需先 `store::init`、建議先 `gametext::must_load`）。
pub async fn run_server(cfg: crate::config::Server) -> anyhow::Result<()> {
    server::run(cfg).await
}

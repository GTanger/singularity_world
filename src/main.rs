#![forbid(unsafe_code)]
// 白塔半島世界觀引擎 — Rust 後端入口（可選：以 Rust 跑 `/ws`；生產預設仍見 `docs/dev/README.md`）。

use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rooms = root.join("data/rooms");
    let runtime = root.join("data/runtime");
    let data = root.join("data");
    std::fs::create_dir_all(root.join("data/runtime")).ok();
    std::fs::create_dir_all(root.join("data/config")).ok();
    singularity_world::gametext::must_load();
    singularity_world::npc::narrative::load();
    let _ = singularity_world::config::load_simulation("");
    let cfg = singularity_world::config::Server::load_default();
    singularity_world::store::init(
        rooms.to_str().expect("utf-8"),
        runtime.to_str().expect("utf-8"),
        data.to_str().expect("utf-8"),
    )?;
    tracing::info!("singularity_world: 啟動滅世後的新生世界【沙鹿・竹林里】（不載入舊世白塔半島 NPC 與聚落種子）");
    let topics = root.join("data/npc_to_npc_topics.json");
    singularity_world::npc::try_load_npc_npc_topics(&topics)
        .or_else(|_| {
            singularity_world::npc::try_load_npc_npc_topics(Path::new(
                "data/npc_to_npc_topics.json",
            ))
        })
        .or_else(|_| {
            singularity_world::npc::try_load_npc_npc_topics(Path::new(
                "../data/npc_to_npc_topics.json",
            ))
        })?;
    let behaviors = root.join("data/npc_behaviors.json");
    if let Err(e) = singularity_world::npc::try_load_npc_behaviors(&behaviors) {
        tracing::warn!("npc_behaviors: {e}");
    }
    if let Err(e) = singularity_world::db::rebuild_room_graph() {
        tracing::warn!("rebuild_room_graph: {e}");
    }
    singularity_world::run_server(cfg).await
}

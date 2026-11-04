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
    if let Err(e) = singularity_world::db::seed_npcs_for_store() {
        tracing::warn!("seed npcs: {e}");
    }
    let _ = singularity_world::db::ensure_all_npcs_have_soul_seed();
    let runtime = singularity_world::store::activate_white_tower_runtime()?;
    tracing::info!(
        "white_tower_runtime: {} places, {} routes, {} goods, {} markets, {} source deals, {} upstream profiles, {} producer sites, {} work sites, {} inventory rows, {} flow lanes, {} entities migrated, {} journeys migrated, {} backups",
        runtime.runtime_places,
        runtime.runtime_routes,
        runtime.economy_goods,
        runtime.economy_markets,
        runtime.economy_trade_evidence,
        runtime.economy_upstream_supply_profiles,
        runtime.economy_producer_sites,
        runtime.economy_work_sites,
        runtime.economy_operational_inventory,
        runtime.economy_flow_lanes,
        runtime.entities_migrated,
        runtime.journeys_migrated,
        runtime.location_backups
    );
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

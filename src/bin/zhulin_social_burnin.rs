//! 沙鹿竹林活態社會 60 遊戲日純 Rust Burn-in 與四大真活硬驗收工具。
//!
//! 用法：
//! - 僅稽核不寫庫：`cargo run --release --bin zhulin-social-burnin -- --days 60`
//! - 稽核通過後寫入 PostgreSQL：`cargo run --release --bin zhulin-social-burnin -- --days 60 --write-pg`

use std::env;

use anyhow::Result;

use singularity_world::grid::SHALU_ZHULIN_WORLD_SEED;
use singularity_world::grid::substrate::runtime as substrate_runtime;
use singularity_world::grid::zhulin::social::burn_in::{
    DEFAULT_BURN_IN_DAYS, run_burn_in_and_audit,
};
use singularity_world::grid::zhulin::social::save_state_to_pg;
use singularity_world::grid::zhulin::social::sim::DEFAULT_RESIDENT_COUNT;
use singularity_world::store::sql::init_pool;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut days = DEFAULT_BURN_IN_DAYS;
    let mut count = DEFAULT_RESIDENT_COUNT;
    let mut write_pg = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--days" if i + 1 < args.len() => {
                days = args[i + 1].parse()?;
                i += 2;
            }
            "--residents" if i + 1 < args.len() => {
                count = args[i + 1].parse()?;
                i += 2;
            }
            "--write-pg" => {
                write_pg = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    // 掛載正式 SWL0 底板以進行真實 Landmass #01 有界加權 A* 硬驗收
    substrate_runtime::init_global();

    let (state, report) = run_burn_in_and_audit(SHALU_ZHULIN_WORLD_SEED, count, days)?;

    if write_pg {
        let pg_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:singularity@localhost:5432/singularity".to_string()
        });
        let pool = init_pool(&pg_url)?;
        save_state_to_pg(&pool, &state)?;
        eprintln!(
            "[zhulin-social-burnin] 已將 {} 遊戲日（{} 拍、{} 人）演化後潔淨狀態壓縮寫入 PostgreSQL zhulin_social_state。",
            days,
            state.tick_count,
            state.residents.len()
        );
    }

    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

//! 白塔半島 runtime 資料庫採用／驗證／回復工具。

use std::time::{SystemTime, UNIX_EPOCH};

use singularity_world::pg::white_tower_runtime;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let command = args.get(1).map(String::as_str).unwrap_or("check");
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:singularity@localhost:5432/singularity".to_string()
    });
    let pool = singularity_world::store::sql::init_pool(&database_url)?;
    // CLI 可能繞過正式服務啟動；只在命令邊界建一次表，避免每個世界動作重跑 DDL。
    singularity_world::pg::living_world::ensure_schema(&pool)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    match command {
        "apply" => println!(
            "{}",
            serde_json::to_string_pretty(&white_tower_runtime::ensure_and_migrate(&pool, now)?)?
        ),
        "check" => println!(
            "{}",
            serde_json::to_string_pretty(&white_tower_runtime::check_active(&pool)?)?
        ),
        "rollback" => println!(
            "{}",
            serde_json::to_string_pretty(&white_tower_runtime::rollback(&pool, now)?)?
        ),
        "gather" => {
            let entity_id = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("gather 需要 entity_id"))?;
            let place_id: String = {
                let mut conn = pool.get()?;
                conn.query_one("SELECT place_id FROM entities WHERE id=$1", &[entity_id])?
                    .get::<_, Option<String>>(0)
                    .ok_or_else(|| anyhow::anyhow!("人物沒有 place_id"))?
            };
            let gathered = singularity_world::pg::world_resource::gather(
                &pool,
                entity_id,
                &place_id,
                1,
                0,
                now,
                "acceptance_probe",
            )?;
            println!("{}", serde_json::to_string_pretty(&gathered)?);
        }
        "ledger" => {
            let (source_delta, inventory_delta, regeneration_delta) =
                singularity_world::pg::world_resource::ledger_balance(&pool)?;
            println!(
                "{}",
                serde_json::json!({
                    "source_delta": source_delta,
                    "inventory_delta": inventory_delta,
                    "regeneration_delta": regeneration_delta,
                    "transfer_balance": source_delta + inventory_delta - regeneration_delta
                })
            );
        }
        other => anyhow::bail!(
            "未知指令 {other}；可用 apply / check / rollback / gather <entity_id> / ledger"
        ),
    }
    Ok(())
}

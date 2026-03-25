use singularity_world::store::sql;
use bcrypt::{hash, DEFAULT_COST};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <entity_id> <new_password>", args[0]);
        std::process::exit(1);
    }

    let entity_id = &args[1];
    let password = &args[2];

    if password.len() < 6 {
        anyhow::bail!("Password must be at least 6 characters long.");
    }

    let pg_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:singularity@localhost:5432/singularity".to_string());

    let pool = sql::init_pool(&pg_url)?;
    let mut conn = pool.get()?;

    let password_hash = hash(password, DEFAULT_COST)?;

    conn.execute(
        "INSERT INTO auth (entity_id, password_hash) 
         VALUES ($1, $2) 
         ON CONFLICT (entity_id) DO UPDATE SET password_hash = EXCLUDED.password_hash",
        &[&entity_id, &password_hash],
    )?;

    println!("Successfully set password for user: {}", entity_id);

    Ok(())
}

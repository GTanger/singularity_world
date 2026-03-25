use std::path::Path;
use rusqlite::Connection as SqliteConnection;
use r2d2_postgres::postgres::{Client as PgClient, NoTls};

fn main() -> anyhow::Result<()> {
    let sqlite_path = "data/runtime/singularity.db";
    let pg_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:singularity@localhost:5432/singularity".to_string());

    if !Path::new(sqlite_path).exists() {
        println!("SQLite database not found at {}, skipping migration.", sqlite_path);
        return Ok(());
    }

    println!("Starting migration from {} to PostgreSQL...", sqlite_path);

    let sqlite_conn = SqliteConnection::open(sqlite_path)?;
    let mut pg_client = PgClient::connect(&pg_url, NoTls)?;

    // Ensure extension
    pg_client.execute("CREATE EXTENSION IF NOT EXISTS vector", &[])?;

    // Migrate Auth
    println!("Migrating Auth...");
    let mut stmt = sqlite_conn.prepare("SELECT entity_id, password_hash FROM auth")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (eid, hash) = row?;
        pg_client.execute(
            "INSERT INTO auth (entity_id, password_hash) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            &[&eid, &hash],
        )?;
    }

    // Migrate Event Log
    println!("Migrating Event Log...");
    let mut stmt = sqlite_conn.prepare("SELECT entity_id, event_type, payload, created_at FROM event_log")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    for row in rows {
        let (eid, etype, payload, at) = row?;
        pg_client.execute(
            "INSERT INTO event_log (entity_id, event_type, payload, created_at) VALUES ($1, $2, $3, $4)",
            &[&eid, &etype, &payload, &at],
        )?;
    }

    // Migrate Archival
    println!("Migrating Archival...");
    let mut stmt = sqlite_conn.prepare("SELECT entity_id, content, tag, created_at FROM archival")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    for row in rows {
        let (eid, content, tag, at) = row?;
        pg_client.execute(
            "INSERT INTO archival (entity_id, content, tag, created_at) VALUES ($1, $2, $3, $4)",
            &[&eid, &content, &tag, &at],
        )?;
    }

    // Migrate NPC Memories
    println!("Migrating NPC Memories...");
    let mut stmt = sqlite_conn.prepare("SELECT entity_id, subject_id, meet_count, favorability FROM npc_memories")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
            row.get::<_, i32>(3)?,
        ))
    })?;
    for row in rows {
        let (eid, sid, meet, fav) = row?;
        pg_client.execute(
            "INSERT INTO npc_memories (entity_id, subject_id, meet_count, favorability) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            &[&eid, &sid, &meet, &fav],
        )?;
    }

    // Migrate Summaries
    println!("Migrating Summaries...");
    let mut stmt = sqlite_conn.prepare("SELECT entity_id, summary FROM npc_summaries")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (eid, sum) = row?;
        pg_client.execute(
            "INSERT INTO npc_summaries (entity_id, summary) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            &[&eid, &sum],
        )?;
    }

    // Migrate NPC-NPC Summaries
    println!("Migrating NPC-NPC Summaries...");
    let mut stmt = sqlite_conn.prepare("SELECT dyad_key, summary FROM npc_npc_summaries")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (key, sum) = row?;
        pg_client.execute(
            "INSERT INTO npc_npc_summaries (dyad_key, summary) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            &[&key, &sum],
        )?;
    }

    // Migrate JSON data (Threads, Dyads, Rumors)
    migrate_json_to_pg(&mut pg_client)?;

    println!("Migration completed successfully!");
    Ok(())
}

fn migrate_json_to_pg(pg: &mut PgClient) -> anyhow::Result<()> {
    let runtime_dir = Path::new("data/runtime");
    
    // Threads
    let thread_path = runtime_dir.join("npc_thread.json");
    if thread_path.exists() {
        println!("Migrating NPC Threads from JSON...");
        let raw = std::fs::read_to_string(thread_path)?;
        let data: serde_json::Value = serde_json::from_str(&raw)?;
        if let Some(entries) = data.get("entries").and_then(|e| e.as_array()) {
            for e in entries {
                let key = e.get("thread_key").and_then(|v| v.as_str()).unwrap_or_default();
                let topic = e.get("topic_type").and_then(|v| v.as_str()).unwrap_or_default();
                let phase = e.get("phase").and_then(|v| v.as_str()).unwrap_or_default();
                let anchors = e.get("anchors").unwrap_or(&serde_json::Value::Array(vec![])).to_string();
                let turn = e.get("turn_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let cool = e.get("cooldown_until").and_then(|v| v.as_i64()).unwrap_or(0);
                let up = e.get("updated_at").and_then(|v| v.as_i64()).unwrap_or(0);
                pg.execute(
                    "INSERT INTO npc_threads (thread_key, topic_type, phase, anchors, turn_count, cooldown_until, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
                    &[&key, &topic, &phase, &anchors, &turn, &cool, &up],
                )?;
            }
        }
    }

    // Dyads
    let dyad_path = runtime_dir.join("npc_dyad.json");
    if dyad_path.exists() {
        println!("Migrating NPC Dyads from JSON...");
        let raw = std::fs::read_to_string(dyad_path)?;
        let data: serde_json::Value = serde_json::from_str(&raw)?;
        if let Some(entries) = data.get("entries").and_then(|e| e.as_array()) {
            for e in entries {
                let a_id = e.get("a_id").and_then(|v| v.as_str()).unwrap_or_default();
                let b_id = e.get("b_id").and_then(|v| v.as_str()).unwrap_or_default();
                let key = if a_id <= b_id { format!("{}|{}", a_id, b_id) } else { format!("{}|{}", b_id, a_id) };
                let fam = e.get("familiarity").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let sent = e.get("sentiment").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let tags = e.get("tags").unwrap_or(&serde_json::Value::Array(vec![])).to_string();
                let up = e.get("updated_at").and_then(|v| v.as_i64()).unwrap_or(0);
                pg.execute(
                    "INSERT INTO npc_dyads (dyad_key, a_id, b_id, familiarity, sentiment, tags, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING",
                    &[&key, &a_id, &b_id, &fam, &sent, &tags, &up],
                )?;
            }
        }
    }

    // Rumors
    let rumor_path = runtime_dir.join("npc_rumors.json");
    if rumor_path.exists() {
        println!("Migrating NPC Rumors from JSON...");
        let raw = std::fs::read_to_string(rumor_path)?;
        let data: serde_json::Value = serde_json::from_str(&raw)?;
        if let Some(entries) = data.get("entries").and_then(|e| e.as_array()) {
            for e in entries {
                let id = e.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                let text = e.get("text").and_then(|v| v.as_str()).unwrap_or_default();
                let rid = e.get("room_id").and_then(|v| v.as_str()).unwrap_or_default();
                let zone = e.get("zone").and_then(|v| v.as_str()).unwrap_or_default();
                let src = e.get("source").and_then(|v| v.as_str()).unwrap_or_default();
                let ss = e.get("source_score").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let w = e.get("weight").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let mc = e.get("mention_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let lua = e.get("last_used_at").and_then(|v| v.as_i64()).unwrap_or(0);
                let bu = e.get("blocked_until").and_then(|v| v.as_i64()).unwrap_or(0);
                let pc = e.get("penalty_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let lpa = e.get("last_penalty_at").and_then(|v| v.as_i64()).unwrap_or(0);
                let lpr = e.get("last_penalty_reason").and_then(|v| v.as_str()).unwrap_or_default();
                let up = e.get("updated_at").and_then(|v| v.as_i64()).unwrap_or(0);
                let ex = e.get("expires_at").and_then(|v| v.as_i64()).unwrap_or(0);
                pg.execute(
                    "INSERT INTO npc_rumors (id, text, room_id, zone, source, source_score, weight, mention_count, 
                                            last_used_at, blocked_until, penalty_count, last_penalty_at, 
                                            last_penalty_reason, updated_at, expires_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) ON CONFLICT DO NOTHING",
                    &[&id, &text, &rid, &zone, &src, &ss, &w, &mc, &lua, &bu, &pc, &lpa, &lpr, &up, &ex],
                )?;
            }
        }
    }

    Ok(())
}

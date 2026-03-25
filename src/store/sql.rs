use r2d2_postgres::{self, PostgresConnectionManager};
use r2d2_postgres::postgres::NoTls;
pub use r2d2::{Pool, PooledConnection};
pub use r2d2_postgres::postgres::Client as Connection;

pub type DbPool = Pool<PostgresConnectionManager<NoTls>>;

pub fn init_pool(url: &str) -> anyhow::Result<DbPool> {
    let manager = PostgresConnectionManager::new(
        url.parse()?,
        NoTls,
    );
    let pool = Pool::new(manager)?;
    
    // Create tables if they don't exist
    let mut conn = pool.get()?;
    create_tables(&mut conn)?;
    
    Ok(pool)
}

fn create_tables(conn: &mut Connection) -> anyhow::Result<()> {
    // 啟用向量擴充功能 (如果尚未啟用)
    conn.execute("CREATE EXTENSION IF NOT EXISTS vector", &[])?;

    // 帳號密碼
    conn.execute(
        "CREATE TABLE IF NOT EXISTS auth (
            entity_id TEXT PRIMARY KEY,
            password_hash TEXT NOT NULL
        )",
        &[],
    )?;

    // 事件日誌
    conn.execute(
        "CREATE TABLE IF NOT EXISTS event_log (
            id SERIAL PRIMARY KEY,
            entity_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL,
            created_at BIGINT NOT NULL
        )",
        &[],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_event_log_entity_id ON event_log(entity_id)",
        &[],
    )?;

    // 長期記憶 (Archival) - 增加 embedding 欄位
    conn.execute(
        "CREATE TABLE IF NOT EXISTS archival (
            id SERIAL PRIMARY KEY,
            entity_id TEXT NOT NULL,
            content TEXT NOT NULL,
            tag TEXT NOT NULL,
            created_at BIGINT NOT NULL,
            embedding vector(1536)
        )",
        &[],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_archival_entity_id ON archival(entity_id)",
        &[],
    )?;

    // 短期記憶 (NpcMemory)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS npc_memories (
            entity_id TEXT NOT NULL,
            subject_id TEXT NOT NULL,
            meet_count INTEGER NOT NULL DEFAULT 0,
            favorability INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (entity_id, subject_id)
        )",
        &[],
    )?;

    // 單人摘要
    conn.execute(
        "CREATE TABLE IF NOT EXISTS npc_summaries (
            entity_id TEXT PRIMARY KEY,
            summary TEXT NOT NULL
        )",
        &[],
    )?;

    // 雙人關係摘要
    conn.execute(
        "CREATE TABLE IF NOT EXISTS npc_npc_summaries (
            dyad_key TEXT PRIMARY KEY,
            summary TEXT NOT NULL
        )",
        &[],
    )?;

    // 對話線 (Threads)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS npc_threads (
            thread_key TEXT PRIMARY KEY,
            topic_type TEXT NOT NULL,
            phase TEXT NOT NULL,
            anchors TEXT NOT NULL,
            turn_count INTEGER NOT NULL DEFAULT 0,
            cooldown_until BIGINT NOT NULL DEFAULT 0,
            updated_at BIGINT NOT NULL
        )",
        &[],
    )?;

    // 關係狀態 (Dyads)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS npc_dyads (
            dyad_key TEXT PRIMARY KEY,
            a_id TEXT NOT NULL,
            b_id TEXT NOT NULL,
            familiarity INTEGER NOT NULL DEFAULT 0,
            sentiment INTEGER NOT NULL DEFAULT 0,
            tags TEXT NOT NULL,
            updated_at BIGINT NOT NULL
        )",
        &[],
    )?;

    // 傳聞 (Rumors)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS npc_rumors (
            id TEXT PRIMARY KEY,
            text TEXT NOT NULL,
            room_id TEXT NOT NULL,
            zone TEXT NOT NULL,
            source TEXT NOT NULL,
            source_score INTEGER NOT NULL DEFAULT 0,
            weight INTEGER NOT NULL DEFAULT 0,
            mention_count INTEGER NOT NULL DEFAULT 0,
            last_used_at BIGINT NOT NULL DEFAULT 0,
            blocked_until BIGINT NOT NULL DEFAULT 0,
            penalty_count INTEGER NOT NULL DEFAULT 0,
            last_penalty_at BIGINT NOT NULL DEFAULT 0,
            last_penalty_reason TEXT NOT NULL,
            updated_at BIGINT NOT NULL,
            expires_at BIGINT NOT NULL
        )",
        &[],
    )?;

    // 房間 (Rooms)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS rooms (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            zone TEXT NOT NULL,
            tags TEXT[] NOT NULL,
            objects TEXT NOT NULL -- JSON string
        )",
        &[],
    )?;

    // 出口 (Exits)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS exits (
            from_room_id TEXT NOT NULL,
            direction TEXT NOT NULL,
            to_room_id TEXT NOT NULL,
            PRIMARY KEY (from_room_id, direction)
        )",
        &[],
    )?;

    Ok(())
}

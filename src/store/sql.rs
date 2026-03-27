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
    // 實體 (Entities) — NPC 與玩家本體
    conn.execute(
        "CREATE TABLE IF NOT EXISTS entities (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL DEFAULT '',
            display_char TEXT NOT NULL DEFAULT '',
            x INTEGER NOT NULL DEFAULT 0,
            y INTEGER NOT NULL DEFAULT 0,
            move_state TEXT NOT NULL DEFAULT '',
            target_x INTEGER,
            target_y INTEGER,
            walk_or_run TEXT NOT NULL DEFAULT '',
            move_started_at BIGINT,
            vit INTEGER NOT NULL DEFAULT 0,
            qi INTEGER NOT NULL DEFAULT 0,
            dex INTEGER NOT NULL DEFAULT 0,
            magnesium INTEGER NOT NULL DEFAULT 0,
            last_observed_at BIGINT,
            created_at BIGINT NOT NULL DEFAULT 0,
            gender TEXT NOT NULL DEFAULT '',
            soul_seed BIGINT,
            display_title TEXT NOT NULL DEFAULT '',
            activated_nodes TEXT NOT NULL DEFAULT '',
            equipment_slots TEXT NOT NULL DEFAULT '',
            inventory TEXT NOT NULL DEFAULT '',
            disposition INTEGER NOT NULL DEFAULT 0,
            current_activity TEXT NOT NULL DEFAULT ''
        )",
        &[],
    )?;

    // 補欄位：current_activity（舊表可能沒有）
    let _ = conn.execute(
        "ALTER TABLE entities ADD COLUMN IF NOT EXISTS current_activity TEXT NOT NULL DEFAULT ''",
        &[],
    );

    // 實體房間對應
    conn.execute(
        "CREATE TABLE IF NOT EXISTS entity_rooms (
            entity_id TEXT PRIMARY KEY,
            room_id TEXT NOT NULL
        )",
        &[],
    )?;

    // 場所 (Venues)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS venues (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            room_ids TEXT NOT NULL DEFAULT '[]',
            max_staff INTEGER NOT NULL DEFAULT 0
        )",
        &[],
    )?;

    // 指派 (Assignments)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS assignments (
            entity_id TEXT NOT NULL,
            occupation_id TEXT NOT NULL,
            venue_id TEXT NOT NULL,
            assigned_by TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (entity_id, venue_id)
        )",
        &[],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_assignments_venue ON assignments(venue_id)",
        &[],
    )?;

    // 排班 (Schedules)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schedules (
            entity_id TEXT PRIMARY KEY,
            work_room TEXT NOT NULL,
            rest_room TEXT NOT NULL,
            shift_start INTEGER NOT NULL DEFAULT 0,
            shift_end INTEGER NOT NULL DEFAULT 0
        )",
        &[],
    )?;

    // 物品定義 (Items)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            slot TEXT NOT NULL DEFAULT '',
            item_type TEXT NOT NULL DEFAULT '',
            weight DOUBLE PRECISION NOT NULL DEFAULT 0,
            stackable INTEGER NOT NULL DEFAULT 0,
            denomination INTEGER NOT NULL DEFAULT 0,
            description TEXT NOT NULL DEFAULT ''
        )",
        &[],
    )?;

    // 世界詞典 (World Lexicon) — NPC 對話中自發產出的詞彙
    conn.execute(
        "CREATE TABLE IF NOT EXISTS world_lexicon (
            term TEXT PRIMARY KEY,
            category TEXT NOT NULL DEFAULT '',
            first_seen BIGINT NOT NULL DEFAULT 0,
            last_seen BIGINT NOT NULL DEFAULT 0,
            mention_count INTEGER NOT NULL DEFAULT 1,
            unique_pairs INTEGER NOT NULL DEFAULT 1,
            source_rooms TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'candidate',
            confirmed_by TEXT NOT NULL DEFAULT ''
        )",
        &[],
    )?;

    Ok(())
}

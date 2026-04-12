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

/// 與 `hex::Terrain` 之 serde `snake_case` 一致（供 `hex_cells` CHECK）
const HEX_TERRAIN_CHECK_IN: &str = "\
'plain','forest','forest_heavy','forest_light','mountain','hills','water','water_deep',\
'desert','swamp','tundra','grassland','jungle','urban','road','bridge','wall',\
'farm_field','farmhouse','inn','tavern','blacksmith','general_store','clinic','workshop',\
'market','guild_hall','temple','academy','library','barracks','guard_post','warehouse',\
'granary','dock','bathhouse','courthouse','jail','town_hall','bank','mint','stables',\
'caravanserai','theater','arena','observatory','alchemist','mage_tower','embassy','prison_yard'";

fn migrate_hex_schema_constraints(conn: &mut Connection) -> anyhow::Result<()> {
    conn.execute(
        "ALTER TABLE hex_cells DROP CONSTRAINT IF EXISTS hex_cells_terrain_valid",
        &[],
    )?;
    let terrain_check = format!(
        "ALTER TABLE hex_cells ADD CONSTRAINT hex_cells_terrain_valid CHECK (terrain IN ({HEX_TERRAIN_CHECK_IN}))"
    );
    conn.execute(terrain_check.as_str(), &[])?;

    conn.execute(
        "ALTER TABLE hex_barriers DROP CONSTRAINT IF EXISTS hex_barriers_dir_valid",
        &[],
    )?;
    conn.execute(
        "ALTER TABLE hex_barriers ADD CONSTRAINT hex_barriers_dir_valid CHECK (dir IN ('e','ne','nw','w','sw','se'))",
        &[],
    )?;

    conn.execute(
        "ALTER TABLE hex_transport_edges DROP CONSTRAINT IF EXISTS hex_transport_edges_mode_valid",
        &[],
    )?;
    conn.execute(
        "ALTER TABLE hex_transport_edges ADD CONSTRAINT hex_transport_edges_mode_valid CHECK (mode IN ('road','water'))",
        &[],
    )?;

    conn.execute(
        "ALTER TABLE hex_transport_edges DROP CONSTRAINT IF EXISTS hex_transport_edges_link_class_valid",
        &[],
    )?;
    conn.execute(
        "ALTER TABLE hex_transport_edges ADD CONSTRAINT hex_transport_edges_link_class_valid CHECK (link_class IN ('official','shortcut'))",
        &[],
    )?;

    Ok(())
}

/// 將舊版 `TEXT` 之 tags/objects 升級為 JSONB，並補上 CHECK。
fn migrate_hex_cells_jsonb(conn: &mut Connection) -> anyhow::Result<()> {
    let row = conn.query_opt(
        "SELECT data_type FROM information_schema.columns
         WHERE table_schema = 'public' AND table_name = 'hex_cells' AND column_name = 'tags'",
        &[],
    )?;
    let Some(row) = row else {
        return Ok(());
    };
    let dtype: String = row.get(0);
    if dtype == "jsonb" {
        migrate_hex_schema_constraints(conn)?;
        return Ok(());
    }

    conn.execute("ALTER TABLE hex_cells ALTER COLUMN tags DROP DEFAULT", &[])?;
    conn.execute(
        "ALTER TABLE hex_cells ALTER COLUMN tags TYPE jsonb USING (CASE
           WHEN trim(tags::text) = '' THEN '[]'::jsonb
           ELSE tags::jsonb
         END)",
        &[],
    )?;
    conn.execute(
        "ALTER TABLE hex_cells ALTER COLUMN tags SET DEFAULT '[]'::jsonb",
        &[],
    )?;

    conn.execute("ALTER TABLE hex_cells ALTER COLUMN objects DROP DEFAULT", &[])?;
    conn.execute(
        "ALTER TABLE hex_cells ALTER COLUMN objects TYPE jsonb USING (CASE
           WHEN trim(objects::text) = '' THEN '[]'::jsonb
           ELSE objects::jsonb
         END)",
        &[],
    )?;
    conn.execute(
        "ALTER TABLE hex_cells ALTER COLUMN objects SET DEFAULT '[]'::jsonb",
        &[],
    )?;

    migrate_hex_schema_constraints(conn)?;
    Ok(())
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
    // 補欄位：Hex 格座標 even-q（權威在 PG；NULL ＝未綁定 hex）
    let _ = conn.execute(
        "ALTER TABLE entities ADD COLUMN IF NOT EXISTS hex_q INTEGER",
        &[],
    );
    let _ = conn.execute(
        "ALTER TABLE entities ADD COLUMN IF NOT EXISTS hex_r INTEGER",
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
            description TEXT NOT NULL DEFAULT '',
            vit_bonus INTEGER NOT NULL DEFAULT 0,
            dex_bonus INTEGER NOT NULL DEFAULT 0,
            atk_bonus INTEGER NOT NULL DEFAULT 0
        )",
        &[],
    )?;
    // 相容已存在的舊表：補欄位
    for col in ["vit_bonus", "dex_bonus", "atk_bonus"] {
        let sql = format!("ALTER TABLE items ADD COLUMN IF NOT EXISTS {col} INTEGER NOT NULL DEFAULT 0");
        let _ = conn.execute(sql.as_str(), &[]);
    }

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

    // 詞元 Embedding (Word Element Embeddings) — 用於語義檢索
    conn.execute(
        "CREATE TABLE IF NOT EXISTS word_element_embeddings (
            id TEXT PRIMARY KEY,
            char TEXT NOT NULL,
            semantic TEXT NOT NULL,
            desc_text TEXT NOT NULL,
            embedding vector(1030) NOT NULL
        )",
        &[],
    )?;

    // Hex 格網：每位玩家各自「已揭露」格（cell_id = q,r 字串）；與世界格契約分表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS hex_player_reveal (
            entity_id TEXT NOT NULL,
            cell_id TEXT NOT NULL,
            revealed_at BIGINT NOT NULL DEFAULT 0,
            PRIMARY KEY (entity_id, cell_id)
        )",
        &[],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_hex_player_reveal_entity ON hex_player_reveal(entity_id)",
        &[],
    )?;

    // Hex 格網世界單例（world_seed + 時間戳）；舊版 grid_json 僅供一次性遷移
    conn.execute(
        "CREATE TABLE IF NOT EXISTS hex_world (
            id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
            grid_json TEXT NOT NULL DEFAULT '',
            updated_at BIGINT NOT NULL DEFAULT 0
        )",
        &[],
    )?;
    let _ = conn.execute(
        "ALTER TABLE hex_world ADD COLUMN IF NOT EXISTS world_seed BIGINT NOT NULL DEFAULT 0",
        &[],
    );

    // 六角格：一列一格（tags/objects 為 JSONB）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS hex_cells (
            q INT NOT NULL,
            r INT NOT NULL,
            terrain TEXT NOT NULL,
            name TEXT NOT NULL DEFAULT '',
            zone TEXT NOT NULL DEFAULT '',
            tags JSONB NOT NULL DEFAULT '[]'::jsonb,
            description TEXT NOT NULL DEFAULT '',
            objects JSONB NOT NULL DEFAULT '[]'::jsonb,
            explored BOOLEAN NOT NULL DEFAULT FALSE,
            PRIMARY KEY (q, r)
        )",
        &[],
    )?;
    // 補欄位：explored（舊表可能沒有）
    let _ = conn.execute(
        "ALTER TABLE hex_cells ADD COLUMN IF NOT EXISTS explored BOOLEAN NOT NULL DEFAULT FALSE",
        &[],
    );
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_hex_cells_terrain ON hex_cells(terrain)",
        &[],
    )?;

    // 障壁：自 coord 往 dir 單向阻斷
    conn.execute(
        "CREATE TABLE IF NOT EXISTS hex_barriers (
            q INT NOT NULL,
            r INT NOT NULL,
            dir TEXT NOT NULL,
            PRIMARY KEY (q, r, dir)
        )",
        &[],
    )?;

    // 傳送門（順序以 sort_order）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS hex_portals (
            id SERIAL PRIMARY KEY,
            sort_order INT NOT NULL DEFAULT 0,
            name TEXT NOT NULL DEFAULT '',
            from_q INT NOT NULL,
            from_r INT NOT NULL,
            to_q INT NOT NULL,
            to_r INT NOT NULL,
            bidirectional BOOLEAN NOT NULL DEFAULT TRUE,
            counts_as_official_link BOOLEAN NOT NULL DEFAULT FALSE
        )",
        &[],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_hex_portals_order ON hex_portals(sort_order)",
        &[],
    )?;

    // 明式交通邊（端點：格或聚落 id）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS hex_transport_edges (
            id SERIAL PRIMARY KEY,
            sort_order INT NOT NULL DEFAULT 0,
            edge_id TEXT,
            a_kind INT NOT NULL,
            a_q INT,
            a_r INT,
            a_settlement_id TEXT,
            b_kind INT NOT NULL,
            b_q INT,
            b_r INT,
            b_settlement_id TEXT,
            mode TEXT NOT NULL,
            operational BOOLEAN NOT NULL DEFAULT TRUE,
            link_class TEXT NOT NULL DEFAULT 'official',
            weight DOUBLE PRECISION
        )",
        &[],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_hex_transport_order ON hex_transport_edges(sort_order)",
        &[],
    )?;

    // hex_cells JSONB 升級 + terrain/dir/mode/link_class 之 CHECK（須在 hex_barriers、hex_transport_edges 建表後）
    migrate_hex_cells_jsonb(conn)?;

    Ok(())
}

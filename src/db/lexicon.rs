//! 世界詞典 (World Lexicon) — NPC 對話中自發產出的詞彙追蹤。

use crate::store;

/// 新增或更新詞典詞條。pair_key 格式為 `{a_id}|{b_id}`。
pub fn upsert_lexicon_term(term: &str, room_id: &str, pair_key: &str) {
    let term = term.trim();
    if term.is_empty() || term.chars().count() < 2 {
        return;
    }
    let now = now_unix();
    let Some(pool) = store::get_db_pool() else { return };
    let Ok(mut conn) = pool.get() else { return };

    let _ = conn.execute(
        "INSERT INTO world_lexicon (term, first_seen, last_seen, mention_count, unique_pairs, source_rooms)
         VALUES ($1, $2, $2, 1, 1, $3)
         ON CONFLICT (term) DO UPDATE SET
            last_seen = $2,
            mention_count = world_lexicon.mention_count + 1,
            source_rooms = CASE
                WHEN world_lexicon.source_rooms LIKE '%' || $3 || '%' THEN world_lexicon.source_rooms
                ELSE world_lexicon.source_rooms || ',' || $3
            END,
            unique_pairs = CASE
                WHEN world_lexicon.source_rooms LIKE '%' || $4 || '%' THEN world_lexicon.unique_pairs
                ELSE world_lexicon.unique_pairs + 1
            END",
        &[
            &term,
            &now,
            &room_id,
            &pair_key,
        ],
    );
}

/// 定期晉升候選詞條。
/// - mention_count ≥ 3 且 unique_pairs ≥ 3 → status = 'nominated'
/// - mention_count ≥ 5 且 source_rooms 含逗號（多房間）→ status = 'promoted'
pub fn promote_lexicon_candidates() {
    let Some(pool) = store::get_db_pool() else { return };
    let Ok(mut conn) = pool.get() else { return };

    // candidate → nominated
    let _ = conn.execute(
        "UPDATE world_lexicon SET status = 'nominated'
         WHERE status = 'candidate'
           AND mention_count >= 3
           AND unique_pairs >= 3",
        &[],
    );

    // nominated → promoted (cross-room: source_rooms has comma = multiple rooms)
    let _ = conn.execute(
        "UPDATE world_lexicon SET status = 'promoted'
         WHERE status = 'nominated'
           AND mention_count >= 5
           AND source_rooms LIKE '%,%'",
        &[],
    );
}

/// 衰減：last_seen 超過 48 小時且 status = 'candidate' → 刪除。
pub fn decay_lexicon() {
    let Some(pool) = store::get_db_pool() else { return };
    let Ok(mut conn) = pool.get() else { return };
    let cutoff = now_unix() - 48 * 3600;
    let _ = conn.execute(
        "DELETE FROM world_lexicon WHERE status = 'candidate' AND last_seen < $1",
        &[&cutoff],
    );
}

/// 列出所有 nominated 或 promoted 的詞條（供日報提煉用）。
pub fn list_notable_terms() -> Vec<LexiconEntry> {
    let Some(pool) = store::get_db_pool() else { return vec![] };
    let Ok(mut conn) = pool.get() else { return vec![] };
    let rows = match conn.query(
        "SELECT term, category, mention_count, unique_pairs, source_rooms, status
         FROM world_lexicon
         WHERE status IN ('nominated', 'promoted')
         ORDER BY mention_count DESC
         LIMIT 100",
        &[],
    ) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    rows.iter()
        .map(|r| LexiconEntry {
            term: r.get::<_, String>(0),
            category: r.get::<_, String>(1),
            mention_count: r.get::<_, i32>(2),
            unique_pairs: r.get::<_, i32>(3),
            source_rooms: r.get::<_, String>(4),
            status: r.get::<_, String>(5),
        })
        .collect()
}

pub struct LexiconEntry {
    pub term: String,
    pub category: String,
    pub mention_count: i32,
    pub unique_pairs: i32,
    pub source_rooms: String,
    pub status: String,
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

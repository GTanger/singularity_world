//! NPC 傳聞 PG 讀寫——從 Store lock 遷出，走 writer queue。

use crate::store::NpcRumor;
use crate::store::sql::DbPool;

/// 批量 upsert 傳聞（writer thread 內呼叫，不持 store lock）。
pub fn upsert_batch(pool: &DbPool, rumors: &[NpcRumor]) -> anyhow::Result<()> {
    if rumors.is_empty() {
        return Ok(());
    }
    let mut conn = pool.get()?;
    for r in rumors {
        conn.execute(
            "INSERT INTO npc_rumors (id, text, room_id, zone, source, source_score, weight, mention_count,
                                    last_used_at, blocked_until, penalty_count, last_penalty_at,
                                    last_penalty_reason, updated_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT(id) DO UPDATE SET
                text=EXCLUDED.text, room_id=EXCLUDED.room_id, zone=EXCLUDED.zone, source=EXCLUDED.source,
                source_score=EXCLUDED.source_score, weight=EXCLUDED.weight, mention_count=EXCLUDED.mention_count,
                last_used_at=EXCLUDED.last_used_at, blocked_until=EXCLUDED.blocked_until,
                penalty_count=EXCLUDED.penalty_count, last_penalty_at=EXCLUDED.last_penalty_at,
                last_penalty_reason=EXCLUDED.last_penalty_reason, updated_at=EXCLUDED.updated_at,
                expires_at=EXCLUDED.expires_at",
            &[&r.id, &r.text, &r.room_id, &r.zone, &r.source, &r.source_score, &r.weight, &r.mention_count,
              &r.last_used_at, &r.blocked_until, &r.penalty_count, &r.last_penalty_at,
              &r.last_penalty_reason, &r.updated_at, &r.expires_at],
        )?;
    }
    Ok(())
}

/// 批量刪除過期傳聞（by id）。
pub fn delete_batch(pool: &DbPool, ids: &[String]) -> anyhow::Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut conn = pool.get()?;
    for id in ids {
        conn.execute("DELETE FROM npc_rumors WHERE id = $1", &[id])?;
    }
    Ok(())
}

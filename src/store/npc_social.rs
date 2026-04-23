//! store NPC 社交 — memory / summary / thread / dyad 的讀寫。
//! 讀取同步查 PG（帶記憶體 fallback），寫入走 writer queue + JSON 備份。

use super::{dyad_key, NpcDyad, NpcMemory, NpcThread, Store};

impl Store {
    // ── NPC Memory ──

    pub fn get_npc_memory(&self, entity_id: &str, subject_id: &str) -> Option<NpcMemory> {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
            && let Ok(Some(row)) = conn.query_opt(
                "SELECT meet_count, favorability FROM npc_memories WHERE entity_id = $1 AND subject_id = $2",
                &[&entity_id, &subject_id],
            )
        {
            return Some(NpcMemory {
                entity_id: entity_id.to_string(),
                subject_id: subject_id.to_string(),
                meet_count: row.get(0),
                favorability: row.get(1),
            });
        }
        None
    }

    pub fn record_meet(&mut self, entity_id: &str, subject_id: &str) -> anyhow::Result<()> {
        crate::pg::writer::submit(crate::pg::writer::WriteOp::RecordMeet {
            entity_id: entity_id.to_string(),
            subject_id: subject_id.to_string(),
        });
        Ok(())
    }

    pub fn adjust_favorability(
        &mut self,
        entity_id: &str,
        subject_id: &str,
        delta: i32,
    ) -> anyhow::Result<()> {
        let old_mem = self.get_npc_memory(entity_id, subject_id);
        let old_fav = old_mem.map_or(0, |m| m.favorability);
        let new_fav = (old_fav + delta).clamp(-100, 100);
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetFavorability {
            entity_id: entity_id.to_string(),
            subject_id: subject_id.to_string(),
            new_fav,
        });
        Ok(())
    }

    // ── NPC Summaries ──

    pub fn get_npc_summary(&self, entity_id: &str) -> String {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
            && let Ok(Some(row)) = conn.query_opt(
                "SELECT summary FROM npc_summaries WHERE entity_id = $1",
                &[&entity_id],
            )
        {
            return row.get(0);
        }
        String::new()
    }

    pub fn set_npc_summary(&mut self, entity_id: &str, summary: &str) -> anyhow::Result<()> {
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetNpcSummary {
            entity_id: entity_id.to_string(),
            summary: summary.to_string(),
        });
        Ok(())
    }

    pub fn get_npc_npc_summary(&self, id_a: &str, id_b: &str) -> String {
        let key = dyad_key(id_a, id_b);
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
            && let Ok(Some(row)) = conn.query_opt(
                "SELECT summary FROM npc_npc_summaries WHERE dyad_key = $1",
                &[&key],
            )
        {
            return row.get(0);
        }
        String::new()
    }

    pub fn set_npc_npc_summary(
        &mut self,
        id_a: &str,
        id_b: &str,
        summary: &str,
    ) -> anyhow::Result<()> {
        let key = dyad_key(id_a, id_b);
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetNpcNpcSummary {
            dyad_key: key,
            summary: summary.to_string(),
        });
        Ok(())
    }

    // ── NPC Threads ──

    pub fn get_npc_thread(&self, id_a: &str, id_b: &str) -> Option<NpcThread> {
        let key = dyad_key(id_a, id_b);
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
            && let Ok(Some(row)) = conn.query_opt(
                "SELECT topic_type, phase, anchors, turn_count, cooldown_until, updated_at \
                 FROM npc_threads WHERE thread_key = $1",
                &[&key],
            )
        {
            let anchors_raw: String = row.get(2);
            let anchors: Vec<String> = serde_json::from_str(&anchors_raw).unwrap_or_default();
            return Some(NpcThread {
                thread_key: key,
                topic_type: row.get(0),
                phase: row.get(1),
                anchors,
                turn_count: row.get(3),
                cooldown_until: row.get(4),
                updated_at: row.get(5),
            });
        }
        self.npc_threads.get(&key).cloned()
    }

    pub fn set_npc_thread(
        &mut self,
        id_a: &str,
        id_b: &str,
        t: NpcThread,
    ) -> anyhow::Result<()> {
        let key = dyad_key(id_a, id_b);
        let anchors_raw = serde_json::to_string(&t.anchors).unwrap_or_else(|_| "[]".to_string());
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetNpcThread {
            key: key.clone(),
            topic_type: t.topic_type.clone(),
            phase: t.phase.clone(),
            anchors_raw,
            turn_count: t.turn_count,
            cooldown_until: t.cooldown_until,
            updated_at: t.updated_at,
        });
        self.npc_threads.insert(key, t);
        self.persist_npc_threads()
    }

    pub fn delete_npc_thread(&mut self, id_a: &str, id_b: &str) -> anyhow::Result<()> {
        let key = dyad_key(id_a, id_b);
        crate::pg::writer::submit(crate::pg::writer::WriteOp::DeleteNpcThread { key: key.clone() });
        self.npc_threads.remove(&key);
        self.persist_npc_threads()
    }

    // ── NPC Dyads ──

    pub fn get_npc_dyad(&self, id_a: &str, id_b: &str) -> Option<NpcDyad> {
        let key = dyad_key(id_a, id_b);
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
            && let Ok(Some(row)) = conn.query_opt(
                "SELECT a_id, b_id, familiarity, sentiment, tags, updated_at \
                 FROM npc_dyads WHERE dyad_key = $1",
                &[&key],
            )
        {
            let tags_raw: String = row.get(4);
            let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
            return Some(NpcDyad {
                a_id: row.get(0),
                b_id: row.get(1),
                familiarity: row.get(2),
                sentiment: row.get(3),
                tags,
                updated_at: row.get(5),
            });
        }
        self.npc_dyads.get(&key).cloned()
    }

    pub fn set_npc_dyad(&mut self, id_a: &str, id_b: &str, d: NpcDyad) -> anyhow::Result<()> {
        let key = dyad_key(id_a, id_b);
        let tags_raw = serde_json::to_string(&d.tags).unwrap_or_else(|_| "[]".to_string());
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetNpcDyad {
            key: key.clone(),
            a_id: d.a_id.clone(),
            b_id: d.b_id.clone(),
            familiarity: d.familiarity,
            sentiment: d.sentiment,
            tags_raw,
            updated_at: d.updated_at,
        });
        self.npc_dyads.insert(key, d);
        self.persist_npc_dyads()
    }
}

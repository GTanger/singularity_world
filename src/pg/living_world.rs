//! 人物連續性、共同行動收據與來源層級的 PostgreSQL 權威資料。

use std::collections::HashMap;

use r2d2_postgres::postgres::{GenericClient, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::living_world::{ActorKind, SourceRecord, builtin_contract, shared_policy_key};
use crate::store::sql::DbPool;
use crate::world::place::WHITE_TOWER_WORLD_ID;

const ALL_GATHER_CORRECTION_ID: &str = "living_world_v1_all_gather_correction";


/// 判斷物品是否為武器或工器具（排斥消耗品、食材、茶水、藥材、素材等）
pub fn is_weapon_or_tool_item(item_id: &str, item_name: &str) -> bool {
    let id_lower = item_id.to_lowercase();
    let name_lower = item_name.to_lowercase();

    // 排除名單：消耗品、食品、茶、藥、生肉、素材殘渣、空白手札
    const EXCLUDE_SUBSTRINGS: &[&str] = &[
        "茶", "湯", "肉", "餅", "果", "藥", "散", "膏", "水", "漿", "手札",
        "corpse_", "mat_", "blank_manual", "food", "potion", "herb", "ingot",
    ];
    for &ex in EXCLUDE_SUBSTRINGS {
        if id_lower.contains(ex) || name_lower.contains(ex) {
            return false;
        }
    }

    // 包含名單：刀、劍、匕、斧、鎬、槍、錘、杖、針、刃、弓、盾、鑿、鋸、鏟等武器或工器具
    const WEAPON_TOOL_KEYWORDS: &[&str] = &[
        "刀", "劍", "匕", "斧", "鎬", "槍", "錘", "杖", "針", "刃", "弓", "盾", "鑿", "鋸", "鏟",
        "knife", "blade", "sword", "axe", "pick", "hammer", "spear", "rod", "staff", "bow", "shield", "tool", "weapon",
    ];
    WEAPON_TOOL_KEYWORDS.iter().any(|&kw| id_lower.contains(kw) || name_lower.contains(kw))
}

/// 判斷物品是否為具有刃口或尖銳端之器物（刀、劍、匕、斧、鎬、槍、刃、鑿等）
pub fn is_bladed_vessel_item(item_id: &str, item_name: &str) -> bool {
    let id_lower = item_id.to_lowercase();
    let name_lower = item_name.to_lowercase();

    // 排除名單
    const EXCLUDE_SUBSTRINGS: &[&str] = &[
        "茶", "湯", "肉", "餅", "果", "藥", "散", "膏", "水", "漿", "手札",
        "corpse_", "mat_", "blank_manual",
    ];
    for &ex in EXCLUDE_SUBSTRINGS {
        if id_lower.contains(ex) || name_lower.contains(ex) {
            return false;
        }
    }

    const BLADED_KEYWORDS: &[&str] = &[
        "刀", "劍", "匕", "斧", "鎬", "槍", "刃", "鑿",
        "knife", "blade", "sword", "axe", "pick", "spear", "dagger",
    ];
    BLADED_KEYWORDS.iter().any(|&kw| id_lower.contains(kw) || name_lower.contains(kw))
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResidentContinuity {
    pub entity_id: String,
    pub control_mode: String,
    pub life_status: String,
    pub current_intent: String,
    pub intent_reason: String,
    pub destination_place_id: Option<String>,
    pub blocked_by: Option<String>,
    pub intent_started_at: i64,
    pub next_wake_at: Option<i64>,
    pub last_action_id: Option<String>,
    pub revision: i64,
    pub updated_at: i64,
    pub source_kind: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResidentObligation {
    pub obligation_id: String,
    pub entity_id: String,
    pub obligation_kind: String,
    pub subject_ref: String,
    pub status: String,
    pub due_at: Option<i64>,
    pub source_ref: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationshipSummary {
    pub familiarity_score: i32,
    pub balance_score: i32,
    pub active_kinds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileReport {
    pub character_count: i64,
    pub autonomous_count: i64,
    pub continuity_count: i64,
    pub obligations_from_assignments: i64,
    pub relationships_from_encounters: i64,
    pub corrected_all_gather_rows: i64,
    pub source_record_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivingWorldAudit {
    pub character_count: i64,
    pub npc_count: i64,
    pub continuity_count: i64,
    pub autonomous_count: i64,
    pub obligation_count: i64,
    pub relationship_count: i64,
    pub knowledge_count: i64,
    pub action_receipt_count: i64,
    pub source_record_count: i64,
    pub distinct_npc_activities: i64,
    pub gathering_npc_count: i64,
    pub trade_count: i64,
    pub trade_receipt_count: i64,
    pub unbalanced_trade_count: i64,
    pub work_count: i64,
    pub work_receipt_count: i64,
    pub unbalanced_work_count: i64,
}

impl LivingWorldAudit {
    #[must_use]
    pub fn all_npcs_gathering(&self) -> bool {
        self.npc_count > 0 && self.gathering_npc_count == self.npc_count
    }
}

#[derive(Debug, Clone)]
pub struct ActionDraft<'a> {
    pub actor_id: &'a str,
    pub action_key: &'a str,
    pub target_ref: &'a str,
    pub place_id: &'a str,
    pub preconditions: Value,
    pub effects: Value,
    pub outcome: &'a str,
    pub source_ref: &'a str,
    pub occurred_at: i64,
}

pub fn ensure_schema(pool: &DbPool) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    ensure_schema_on(&mut *conn)
}

pub fn ensure_schema_on(client: &mut impl GenericClient) -> anyhow::Result<()> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS resident_continuity (
            entity_id TEXT PRIMARY KEY,
            control_mode TEXT NOT NULL CHECK(control_mode IN ('autonomous','player')),
            life_status TEXT NOT NULL CHECK(life_status IN ('alive','dead')),
            current_intent TEXT NOT NULL,
            intent_reason TEXT NOT NULL,
            destination_place_id TEXT,
            blocked_by TEXT,
            intent_started_at BIGINT NOT NULL,
            next_wake_at BIGINT,
            last_action_id TEXT,
            revision BIGINT NOT NULL DEFAULT 0,
            updated_at BIGINT NOT NULL,
            source_kind TEXT NOT NULL,
            source_ref TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_resident_continuity_due
        ON resident_continuity(next_wake_at,entity_id)
        WHERE control_mode='autonomous' AND life_status='alive';

        CREATE TABLE IF NOT EXISTS resident_obligations (
            obligation_id TEXT PRIMARY KEY,
            entity_id TEXT NOT NULL,
            obligation_kind TEXT NOT NULL,
            subject_ref TEXT NOT NULL,
            status TEXT NOT NULL,
            due_at BIGINT,
            source_ref TEXT NOT NULL,
            created_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_resident_obligations_entity
        ON resident_obligations(entity_id,status);

        CREATE TABLE IF NOT EXISTS resident_knowledge (
            entity_id TEXT NOT NULL,
            subject_ref TEXT NOT NULL,
            predicate TEXT NOT NULL,
            value TEXT NOT NULL,
            certainty INTEGER NOT NULL CHECK(certainty BETWEEN 0 AND 100),
            source_ref TEXT NOT NULL,
            learned_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            PRIMARY KEY(entity_id,subject_ref,predicate)
        );

        CREATE TABLE IF NOT EXISTS resident_relationships (
            entity_id TEXT NOT NULL,
            other_entity_id TEXT NOT NULL,
            relation_kind TEXT NOT NULL,
            evidence_count INTEGER NOT NULL DEFAULT 0 CHECK(evidence_count >= 0),
            source_ref TEXT NOT NULL,
            last_interaction_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            PRIMARY KEY(entity_id,other_entity_id,relation_kind),
            CHECK(entity_id <> other_entity_id)
        );

        CREATE TABLE IF NOT EXISTS world_action_receipts (
            action_id TEXT PRIMARY KEY,
            world_id TEXT NOT NULL,
            actor_id TEXT NOT NULL,
            actor_kind TEXT NOT NULL CHECK(actor_kind IN ('player','npc')),
            policy_key TEXT NOT NULL,
            action_key TEXT NOT NULL,
            target_ref TEXT NOT NULL,
            place_id TEXT NOT NULL,
            preconditions TEXT NOT NULL,
            effects TEXT NOT NULL,
            outcome TEXT NOT NULL,
            source_ref TEXT NOT NULL,
            occurred_at BIGINT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_world_action_receipts_actor
        ON world_action_receipts(actor_id,occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_world_action_receipts_target
        ON world_action_receipts(target_ref,occurred_at DESC);

        CREATE TABLE IF NOT EXISTS world_fact_registry (
            fact_id TEXT PRIMARY KEY,
            subject_ref TEXT NOT NULL,
            fact_key TEXT NOT NULL,
            fact_value TEXT NOT NULL,
            source_tier TEXT NOT NULL CHECK(source_tier IN ('U','A','W','G','R')),
            source_ref TEXT NOT NULL,
            status TEXT NOT NULL,
            canon BOOLEAN NOT NULL,
            runtime_authority BOOLEAN NOT NULL,
            runtime_projection TEXT NOT NULL,
            updated_at BIGINT NOT NULL,
            CHECK(NOT canon OR source_tier IN ('U','A')),
            CHECK(NOT (source_tier='R' AND runtime_authority)),
            CHECK(NOT runtime_authority OR runtime_projection <> '')
        );

        CREATE TABLE IF NOT EXISTS living_world_migrations (
            id TEXT PRIMARY KEY,
            applied_at BIGINT NOT NULL,
            affected_count BIGINT NOT NULL
        );",
    )?;
    Ok(())
}

/// 只從已有角色、旅程、指派與相遇證據建立連續資料；不虛構家庭、職業或承諾。
pub fn reconcile_existing_characters(
    pool: &DbPool,
    now_unix: i64,
) -> anyhow::Result<ReconcileReport> {
    let contract = builtin_contract()?;
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    ensure_schema_on(&mut tx)?;

    tx.execute(
        "INSERT INTO resident_continuity
         (entity_id,control_mode,life_status,current_intent,intent_reason,
          destination_place_id,blocked_by,intent_started_at,next_wake_at,last_action_id,
          revision,updated_at,source_kind,source_ref)
         SELECT e.id,
                CASE WHEN e.kind='npc' THEN 'autonomous' ELSE 'player' END,
                CASE WHEN e.vit>0 THEN 'alive' ELSE 'dead' END,
                CASE
                  WHEN e.kind='player' THEN 'player_directed'
                  WHEN e.current_activity='正在採集' AND e.magnesium<50
                       AND COALESCE(e.inventory,'[]') NOT IN ('','[]') THEN 'reassess'
                  ELSE COALESCE(NULLIF(j.reason,''),'idle')
                END,
                CASE
                  WHEN e.kind='player' THEN '等待人物本人輸入'
                  WHEN e.current_activity='正在採集' AND e.magnesium<50
                       AND COALESCE(e.inventory,'[]') NOT IN ('','[]')
                    THEN '既有採集硬優先忽略了持物，重新依全部事實決定'
                  ELSE COALESCE(NULLIF(j.consequence,''),'由既有執行狀態建立')
                END,
                j.destination_place_id,NULL,COALESCE(j.updated_at,$1),
                CASE WHEN e.kind='npc' AND e.vit>0
                     THEN $1 + mod(abs(hashtext(e.id)::bigint),30) ELSE NULL END,
                NULL,0,$1,'runtime_reconciliation','living_world_v1'
         FROM entities e LEFT JOIN entity_journeys j ON j.entity_id=e.id
         WHERE e.kind IN ('npc','player')
         ON CONFLICT(entity_id) DO UPDATE SET
           control_mode=EXCLUDED.control_mode,life_status=EXCLUDED.life_status,
           next_wake_at=CASE
             WHEN EXCLUDED.control_mode='player' OR EXCLUDED.life_status='dead' THEN NULL
             ELSE resident_continuity.next_wake_at END,
           updated_at=EXCLUDED.updated_at",
        &[&now_unix],
    )?;

    let correction_claimed = tx.execute(
        "INSERT INTO living_world_migrations(id,applied_at,affected_count)
         VALUES ($1,$2,0) ON CONFLICT(id) DO NOTHING",
        &[&ALL_GATHER_CORRECTION_ID, &now_unix],
    )?;
    let corrected = if correction_claimed == 1 {
        let changed = tx.execute(
            "UPDATE entities SET current_activity='正在重新安排手邊的事'
             WHERE kind='npc' AND vit>0 AND current_activity='正在採集'
               AND magnesium<50 AND COALESCE(inventory,'[]') NOT IN ('','[]')",
            &[],
        )? as i64;
        tx.execute(
            "UPDATE resident_continuity c SET current_intent='reassess',
                 intent_reason='既有採集硬優先忽略了持物，重新依全部事實決定',
                 destination_place_id=NULL,blocked_by=NULL,intent_started_at=$1,
                 next_wake_at=$1 + mod(abs(hashtext(c.entity_id)::bigint),30),
                 revision=revision+1,updated_at=$1,source_kind='runtime_correction',
                 source_ref=$2
             FROM entities e WHERE e.id=c.entity_id AND e.kind='npc' AND e.vit>0
               AND e.current_activity='正在重新安排手邊的事'",
            &[&now_unix, &ALL_GATHER_CORRECTION_ID],
        )?;
        tx.execute(
            "UPDATE living_world_migrations SET affected_count=$1 WHERE id=$2",
            &[&changed, &ALL_GATHER_CORRECTION_ID],
        )?;
        changed
    } else {
        0
    };

    tx.execute(
        "INSERT INTO resident_obligations
         (obligation_id,entity_id,obligation_kind,subject_ref,status,due_at,
          source_ref,created_at,updated_at)
         SELECT 'assignment:'||a.entity_id||':'||a.venue_id,a.entity_id,
                'employment',a.venue_id,'active',NULL,
                'assignments:'||a.occupation_id,$1,$1
         FROM assignments a
         ON CONFLICT(obligation_id) DO UPDATE SET status='active',updated_at=EXCLUDED.updated_at",
        &[&now_unix],
    )?;
    tx.execute(
        "UPDATE resident_obligations SET status='active', updated_at=$1 WHERE status='open'",
        &[&now_unix],
    )?;
    tx.execute(
        "UPDATE resident_obligations SET status='completed', updated_at=$1 WHERE status='resolved'",
        &[&now_unix],
    )?;

    tx.execute(
        "INSERT INTO resident_relationships
         (entity_id,other_entity_id,relation_kind,evidence_count,source_ref,
          last_interaction_at,updated_at)
         SELECT m.entity_id,m.subject_id,'acquaintance',GREATEST(m.meet_count,0),
                'npc_memories.meet_count',$1,$1
         FROM npc_memories m
         WHERE m.entity_id<>m.subject_id AND m.meet_count>0
         ON CONFLICT(entity_id,other_entity_id,relation_kind) DO UPDATE SET
           evidence_count=GREATEST(resident_relationships.evidence_count,
                                   EXCLUDED.evidence_count),
           updated_at=EXCLUDED.updated_at",
        &[&now_unix],
    )?;
    for record in &contract.source_records {
        upsert_source_record(&mut tx, record, now_unix)?;
    }

    let character_count = count_one(
        &mut tx,
        "SELECT COUNT(*)::bigint FROM entities WHERE kind IN ('npc','player')",
    )?;
    let autonomous_count = count_one(
        &mut tx,
        "SELECT COUNT(*)::bigint FROM resident_continuity WHERE control_mode='autonomous'",
    )?;
    let continuity_count = count_one(&mut tx, "SELECT COUNT(*)::bigint FROM resident_continuity")?;
    let obligations_from_assignments = count_one(
        &mut tx,
        "SELECT COUNT(*)::bigint FROM resident_obligations WHERE source_ref LIKE 'assignments:%'",
    )?;
    let relationships_from_encounters = count_one(
        &mut tx,
        "SELECT COUNT(*)::bigint FROM resident_relationships WHERE source_ref='npc_memories.meet_count'",
    )?;
    let source_record_count =
        count_one(&mut tx, "SELECT COUNT(*)::bigint FROM world_fact_registry")?;
    tx.commit()?;
    Ok(ReconcileReport {
        character_count,
        autonomous_count,
        continuity_count,
        obligations_from_assignments,
        relationships_from_encounters,
        corrected_all_gather_rows: corrected,
        source_record_count,
    })
}

pub fn load_autonomous(pool: &DbPool) -> anyhow::Result<HashMap<String, ResidentContinuity>> {
    let mut conn = pool.get()?;
    let rows = conn.query(
        "SELECT entity_id,control_mode,life_status,current_intent,intent_reason,
                destination_place_id,blocked_by,intent_started_at,next_wake_at,last_action_id,
                revision,updated_at,source_kind,source_ref
         FROM resident_continuity
         WHERE control_mode='autonomous' AND life_status='alive' ORDER BY entity_id",
        &[],
    )?;
    Ok(rows
        .into_iter()
        .map(continuity_from_row)
        .map(|state| (state.entity_id.clone(), state))
        .collect())
}

pub fn upsert_continuity(pool: &DbPool, state: &ResidentContinuity) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    conn.execute(
        "INSERT INTO resident_continuity
         (entity_id,control_mode,life_status,current_intent,intent_reason,
          destination_place_id,blocked_by,intent_started_at,next_wake_at,last_action_id,
          revision,updated_at,source_kind,source_ref)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
         ON CONFLICT(entity_id) DO UPDATE SET
           control_mode=EXCLUDED.control_mode,life_status=EXCLUDED.life_status,
           current_intent=EXCLUDED.current_intent,intent_reason=EXCLUDED.intent_reason,
           destination_place_id=EXCLUDED.destination_place_id,blocked_by=EXCLUDED.blocked_by,
           intent_started_at=EXCLUDED.intent_started_at,next_wake_at=EXCLUDED.next_wake_at,
           revision=resident_continuity.revision+1,updated_at=EXCLUDED.updated_at,
           source_kind=EXCLUDED.source_kind,source_ref=EXCLUDED.source_ref",
        &[
            &state.entity_id,
            &state.control_mode,
            &state.life_status,
            &state.current_intent,
            &state.intent_reason,
            &state.destination_place_id,
            &state.blocked_by,
            &state.intent_started_at,
            &state.next_wake_at,
            &state.last_action_id,
            &state.revision,
            &state.updated_at,
            &state.source_kind,
            &state.source_ref,
        ],
    )?;
    Ok(())
}

/// 與世界狀態同一交易寫入；actor_kind 只作證據，不參與規則分流。
pub fn record_action_in_tx(
    tx: &mut Transaction<'_>,
    draft: ActionDraft<'_>,
) -> anyhow::Result<String> {
    let row = tx.query_one("SELECT kind FROM entities WHERE id=$1", &[&draft.actor_id])?;
    let kind: String = row.get(0);
    let actor_kind = match kind.as_str() {
        "player" => ActorKind::Player,
        "npc" => ActorKind::Npc,
        _ => anyhow::bail!("非人物實體不可使用共同行動入口：{}", draft.actor_id),
    };
    let policy_key = shared_policy_key(actor_kind, draft.action_key)?;
    let action_id = Uuid::new_v4().to_string();
    let preconditions = serde_json::to_string(&draft.preconditions)?;
    let effects = serde_json::to_string(&draft.effects)?;
    tx.execute(
        "INSERT INTO world_action_receipts
         (action_id,world_id,actor_id,actor_kind,policy_key,action_key,target_ref,
          place_id,preconditions,effects,outcome,source_ref,occurred_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        &[
            &action_id,
            &WHITE_TOWER_WORLD_ID,
            &draft.actor_id,
            &actor_kind.as_str(),
            &policy_key,
            &draft.action_key,
            &draft.target_ref,
            &draft.place_id,
            &preconditions,
            &effects,
            &draft.outcome,
            &draft.source_ref,
            &draft.occurred_at,
        ],
    )?;
    tx.execute(
        "UPDATE resident_continuity SET last_action_id=$1,revision=revision+1,
         updated_at=$2 WHERE entity_id=$3",
        &[&action_id, &draft.occurred_at, &draft.actor_id],
    )?;
    Ok(action_id)
}

/// 便捷函式：嘗試取得全域 DbPool
pub fn try_pool() -> Option<std::sync::Arc<DbPool>> {
    crate::pg::pool()
}

pub fn record_encounter(
    pool: &DbPool,
    actor_id: &str,
    other_id: &str,
    place_id: &str,
    now_unix: i64,
) -> anyhow::Result<()> {
    anyhow::ensure!(actor_id != other_id, "人物不可與自己相遇");
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    for (entity_id, peer_id) in [(actor_id, other_id), (other_id, actor_id)] {
        tx.execute(
            "INSERT INTO resident_relationships
             (entity_id,other_entity_id,relation_kind,evidence_count,source_ref,
              last_interaction_at,updated_at)
             VALUES ($1,$2,'acquaintance',1,'runtime:co_presence',$3,$3)
             ON CONFLICT(entity_id,other_entity_id,relation_kind) DO UPDATE SET
               evidence_count=resident_relationships.evidence_count+1,
               source_ref='runtime:co_presence',last_interaction_at=EXCLUDED.last_interaction_at,
               updated_at=EXCLUDED.updated_at",
            &[&entity_id, &peer_id, &now_unix],
        )?;
        remember_fact(
            &mut tx,
            entity_id,
            peer_id,
            "last_seen_at",
            place_id,
            100,
            "runtime:co_presence",
            now_unix,
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn open_obligation(
    pool: &DbPool,
    entity_id: &str,
    obligation_kind: &str,
    subject_ref: &str,
    due_at: Option<i64>,
    source_ref: &str,
    now_unix: i64,
) -> anyhow::Result<String> {
    let mut conn = pool.get()?;
    let existing = conn.query_opt(
        "SELECT obligation_id FROM resident_obligations WHERE entity_id = $1 AND obligation_kind = $2 AND status IN ('active', 'open') LIMIT 1",
        &[&entity_id, &obligation_kind],
    )?;
    if let Some(row) = existing {
        let ob_id: String = row.get(0);
        conn.execute(
            "UPDATE resident_obligations SET subject_ref = $1, status = 'active', due_at = $2, source_ref = $3, updated_at = $4 WHERE obligation_id = $5",
            &[&subject_ref, &due_at, &source_ref, &now_unix, &ob_id],
        )?;
        Ok(ob_id)
    } else {
        let ob_id = format!("ob:{}:{}:{}", entity_id, obligation_kind, now_unix);
        conn.execute(
            "INSERT INTO resident_obligations (obligation_id, entity_id, obligation_kind, subject_ref, status, due_at, source_ref, created_at, updated_at) VALUES ($1, $2, $3, $4, 'active', $5, $6, $7, $8)",
            &[&ob_id, &entity_id, &obligation_kind, &subject_ref, &due_at, &source_ref, &now_unix, &now_unix],
        )?;
        Ok(ob_id)
    }
}

pub fn resolve_obligations_by_kind(
    pool: &DbPool,
    entity_id: &str,
    obligation_kind: &str,
    now_unix: i64,
) -> anyhow::Result<u64> {
    let mut conn = pool.get()?;
    let rows = conn.execute(
        "UPDATE resident_obligations SET status = 'completed', updated_at = $3 WHERE entity_id = $1 AND obligation_kind = $2 AND status IN ('active', 'open')",
        &[&entity_id, &obligation_kind, &now_unix],
    )?;
    Ok(rows)
}

pub fn mark_entity_dead(
    pool: &DbPool,
    entity_id: &str,
    source_ref: &str,
    now_unix: i64,
) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    tx.execute(
        "UPDATE resident_continuity
         SET life_status = 'dead', current_intent = 'dead', intent_reason = '已身亡',
             destination_place_id = NULL, blocked_by = NULL, next_wake_at = NULL,
             revision = revision + 1, updated_at = $2, source_kind = 'combat', source_ref = $3
         WHERE entity_id = $1",
        &[&entity_id, &now_unix, &source_ref],
    )?;
    tx.execute(
        "UPDATE resident_obligations SET status = 'failed', updated_at = $2
         WHERE entity_id = $1 AND status IN ('active', 'open')",
        &[&entity_id, &now_unix],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn list_open_obligations_for_entity(
    pool: &DbPool,
    entity_id: &str,
) -> anyhow::Result<Vec<ResidentObligation>> {
    let mut conn = pool.get()?;
    let rows = conn.query(
        "SELECT obligation_id, entity_id, obligation_kind, subject_ref, status, due_at, source_ref, created_at, updated_at
         FROM resident_obligations
         WHERE entity_id = $1 AND status IN ('active', 'open')
         ORDER BY updated_at DESC",
        &[&entity_id],
    )?;
    Ok(rows.into_iter().map(obligation_from_row).collect())
}

pub fn list_open_obligations_at_place(
    pool: &DbPool,
    place_id: &str,
) -> anyhow::Result<Vec<ResidentObligation>> {
    let mut conn = pool.get()?;
    let pattern = format!("%{}%", place_id);
    let rows = conn.query(
        "SELECT obligation_id, entity_id, obligation_kind, subject_ref, status, due_at, source_ref, created_at, updated_at
         FROM resident_obligations
         WHERE (subject_ref = $1 OR source_ref LIKE $2 OR entity_id IN (SELECT entity_id FROM entity_rooms WHERE room_id = $1))
           AND status IN ('active', 'open')
         ORDER BY updated_at DESC",
        &[&place_id, &pattern],
    )?;
    Ok(rows.into_iter().map(obligation_from_row).collect())
}

pub fn record_relationship_delta(
    pool: &DbPool,
    entity_id: &str,
    other_entity_id: &str,
    relation_kind: &str,
    delta_evidence: i32,
    source_ref: &str,
    now_unix: i64,
) -> anyhow::Result<()> {
    if entity_id.trim().is_empty() || other_entity_id.trim().is_empty() || entity_id == other_entity_id {
        return Ok(());
    }
    let mut conn = pool.get()?;
    conn.execute(
        "INSERT INTO resident_relationships
         (entity_id, other_entity_id, relation_kind, evidence_count, source_ref, last_interaction_at, updated_at)
         VALUES ($1, $2, $3, GREATEST(0, $4), $5, $6, $6)
         ON CONFLICT (entity_id, other_entity_id, relation_kind) DO UPDATE SET
           evidence_count = GREATEST(0, resident_relationships.evidence_count + $4),
           source_ref = EXCLUDED.source_ref,
           last_interaction_at = EXCLUDED.last_interaction_at,
           updated_at = EXCLUDED.updated_at",
        &[&entity_id, &other_entity_id, &relation_kind, &delta_evidence, &source_ref, &now_unix],
    )?;
    Ok(())
}
pub fn upsert_relationship(
    pool: &DbPool,
    entity_id: &str,
    other_entity_id: &str,
    relation_kind: &str,
    delta_evidence: i32,
    initial_evidence: i32,
    source_ref: &str,
    now_unix: i64,
) -> anyhow::Result<()> {
    if entity_id.trim().is_empty() || other_entity_id.trim().is_empty() || entity_id == other_entity_id {
        return Ok(());
    }
    let mut conn = pool.get()?;
    conn.execute(
        "INSERT INTO resident_relationships
         (entity_id, other_entity_id, relation_kind, evidence_count, source_ref, last_interaction_at, updated_at)
         VALUES ($1, $2, $3, GREATEST(0, $4), $5, $6, $6)
         ON CONFLICT (entity_id, other_entity_id, relation_kind) DO UPDATE SET
           evidence_count = GREATEST(0, resident_relationships.evidence_count + $7),
           source_ref = EXCLUDED.source_ref,
           last_interaction_at = EXCLUDED.last_interaction_at,
           updated_at = EXCLUDED.updated_at",
        &[&entity_id, &other_entity_id, &relation_kind, &initial_evidence, &source_ref, &now_unix, &delta_evidence],
    )?;
    Ok(())
}


pub fn get_relationship_summary(
    pool: &DbPool,
    entity_id: &str,
    other_entity_id: &str,
) -> anyhow::Result<RelationshipSummary> {
    let mut conn = pool.get()?;
    let rows = conn.query(
        "SELECT relation_kind, evidence_count FROM resident_relationships WHERE entity_id = $1 AND other_entity_id = $2",
        &[&entity_id, &other_entity_id],
    )?;
    let mut active_kinds = Vec::new();
    let mut counts: HashMap<String, i32> = HashMap::new();
    for row in rows {
        let kind: String = row.get(0);
        let count: i32 = row.get(1);
        if count > 0 {
            active_kinds.push(kind.clone());
            counts.insert(kind, count);
        }
    }
    let get = |k: &str| *counts.get(k).unwrap_or(&0);
    let familiarity_score = get("acquaintance") + get("met") + get("talked") + get("traded") + get("helped") + get("borrowed");
    let balance_score = 2 * get("helped") + get("traded") + get("talked") - 3 * get("grudge") - 2 * get("debt") - 4 * get("hostile");
    Ok(RelationshipSummary {
        familiarity_score,
        balance_score,
        active_kinds,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn record_player_world_action(
    pool: &DbPool,
    actor_id: &str,
    action_key: &str,
    target_ref: &str,
    place_id: &str,
    outcome: &str,
    effects: serde_json::Value,
    now_unix: i64,
) -> anyhow::Result<String> {
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let draft = ActionDraft {
        actor_id,
        action_key,
        target_ref,
        place_id,
        preconditions: serde_json::json!({}),
        effects,
        outcome,
        source_ref: "runtime:player_action",
        occurred_at: now_unix,
    };
    let action_id = record_action_in_tx(&mut tx, draft)?;
    tx.commit()?;
    Ok(action_id)
}

fn obligation_from_row(row: r2d2_postgres::postgres::Row) -> ResidentObligation {
    ResidentObligation {
        obligation_id: row.get(0),
        entity_id: row.get(1),
        obligation_kind: row.get(2),
        subject_ref: row.get(3),
        status: row.get(4),
        due_at: row.get(5),
        source_ref: row.get(6),
        created_at: row.get(7),
        updated_at: row.get(8),
    }
}

pub fn audit(pool: &DbPool) -> anyhow::Result<LivingWorldAudit> {
    let mut conn = pool.get()?;
    ensure_schema_on(&mut *conn)?;
    let row = conn.query_one(
        "SELECT
          (SELECT COUNT(*)::bigint FROM entities WHERE kind IN ('npc','player')),
          (SELECT COUNT(*)::bigint FROM entities WHERE kind='npc' AND vit>0),
          (SELECT COUNT(*)::bigint FROM resident_continuity),
          (SELECT COUNT(*)::bigint FROM resident_continuity
             WHERE control_mode='autonomous' AND life_status='alive'),
          (SELECT COUNT(*)::bigint FROM resident_obligations WHERE status='active'),
          (SELECT COUNT(*)::bigint FROM resident_relationships),
          (SELECT COUNT(*)::bigint FROM resident_knowledge),
          (SELECT COUNT(*)::bigint FROM world_action_receipts),
          (SELECT COUNT(*)::bigint FROM world_fact_registry),
          (SELECT COUNT(DISTINCT current_activity)::bigint FROM entities
             WHERE kind='npc' AND vit>0),
          (SELECT COUNT(*)::bigint FROM entities
             WHERE kind='npc' AND vit>0 AND current_activity='正在採集'),
          (SELECT COUNT(*)::bigint FROM world_trade_ledger),
          (SELECT COUNT(*)::bigint FROM world_action_receipts WHERE action_key='trade'),
          (SELECT COUNT(*)::bigint FROM world_trade_ledger
             WHERE buyer_mg_before+seller_mg_before<>buyer_mg_after+seller_mg_after
                OR buyer_quantity_before+seller_quantity_before<>
                   buyer_quantity_after+seller_quantity_after),
          (SELECT COUNT(*)::bigint FROM world_work_ledger),
          (SELECT COUNT(*)::bigint FROM world_action_receipts WHERE action_key='work'),
          (SELECT COUNT(*)::bigint FROM world_work_ledger
             WHERE entity_mg_before+treasury_mg_before<>
                   entity_mg_after+treasury_mg_after)",
        &[],
    )?;
    Ok(LivingWorldAudit {
        character_count: row.get(0),
        npc_count: row.get(1),
        continuity_count: row.get(2),
        autonomous_count: row.get(3),
        obligation_count: row.get(4),
        relationship_count: row.get(5),
        knowledge_count: row.get(6),
        action_receipt_count: row.get(7),
        source_record_count: row.get(8),
        distinct_npc_activities: row.get(9),
        gathering_npc_count: row.get(10),
        trade_count: row.get(11),
        trade_receipt_count: row.get(12),
        unbalanced_trade_count: row.get(13),
        work_count: row.get(14),
        work_receipt_count: row.get(15),
        unbalanced_work_count: row.get(16),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn remember_fact(
    client: &mut impl GenericClient,
    entity_id: &str,
    subject_ref: &str,
    predicate: &str,
    value: &str,
    certainty: i32,
    source_ref: &str,
    now_unix: i64,
) -> anyhow::Result<()> {
    client.execute(
        "INSERT INTO resident_knowledge
         (entity_id,subject_ref,predicate,value,certainty,source_ref,learned_at,updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$7)
         ON CONFLICT(entity_id,subject_ref,predicate) DO UPDATE SET
           value=EXCLUDED.value,certainty=EXCLUDED.certainty,source_ref=EXCLUDED.source_ref,
           updated_at=EXCLUDED.updated_at",
        &[
            &entity_id,
            &subject_ref,
            &predicate,
            &value,
            &certainty,
            &source_ref,
            &now_unix,
        ],
    )?;
    Ok(())
}

fn upsert_source_record(
    client: &mut impl GenericClient,
    record: &SourceRecord,
    now_unix: i64,
) -> anyhow::Result<()> {
    record.validate()?;
    client.execute(
        "INSERT INTO world_fact_registry
         (fact_id,subject_ref,fact_key,fact_value,source_tier,source_ref,status,
          canon,runtime_authority,runtime_projection,updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
         ON CONFLICT(fact_id) DO UPDATE SET
           subject_ref=EXCLUDED.subject_ref,fact_key=EXCLUDED.fact_key,
           fact_value=EXCLUDED.fact_value,source_tier=EXCLUDED.source_tier,
           source_ref=EXCLUDED.source_ref,status=EXCLUDED.status,canon=EXCLUDED.canon,
           runtime_authority=EXCLUDED.runtime_authority,
           runtime_projection=EXCLUDED.runtime_projection,updated_at=EXCLUDED.updated_at",
        &[
            &record.fact_id,
            &record.subject_ref,
            &record.fact_key,
            &record.fact_value,
            &record.source_tier.as_str(),
            &record.source_ref,
            &record.status,
            &record.canon,
            &record.runtime_authority,
            &record.runtime_projection,
            &now_unix,
        ],
    )?;
    Ok(())
}

fn continuity_from_row(row: r2d2_postgres::postgres::Row) -> ResidentContinuity {
    ResidentContinuity {
        entity_id: row.get(0),
        control_mode: row.get(1),
        life_status: row.get(2),
        current_intent: row.get(3),
        intent_reason: row.get(4),
        destination_place_id: row.get(5),
        blocked_by: row.get(6),
        intent_started_at: row.get(7),
        next_wake_at: row.get(8),
        last_action_id: row.get(9),
        revision: row.get(10),
        updated_at: row.get(11),
        source_kind: row.get(12),
        source_ref: row.get(13),
    }
}

fn count_one(client: &mut impl GenericClient, query: &str) -> anyhow::Result<i64> {
    Ok(client.query_one(query, &[])?.get(0))
}

// ═══════════════════════════════════════════════════════════════════════════
// 四大進階閉環之 PostgreSQL 權威持久層（部位傷勢與瘴毒、地面遺落物、功法手札、山海獸殘軀）
// ═══════════════════════════════════════════════════════════════════════════

use std::sync::{LazyLock, RwLock, atomic::{AtomicU64, Ordering}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AfflictionKind {
    HeadConcussion, // 頭面震盪
    TorsoInternal,  // 胸脅內傷
    ArmSprain,      // 腕肘挫傷
    LegSprain,      // 下盤扭傷
    Miasma,         // 瘴毒入體（非部位環境傷患，須以藥圃草藥湯劑拔除）
}

impl AfflictionKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::HeadConcussion => "head",
            Self::TorsoInternal => "torso",
            Self::ArmSprain => "arm",
            Self::LegSprain => "leg",
            Self::Miasma => "miasma",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "head" => Some(Self::HeadConcussion),
            "torso" => Some(Self::TorsoInternal),
            "arm" => Some(Self::ArmSprain),
            "leg" => Some(Self::LegSprain),
            "miasma" => Some(Self::Miasma),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::HeadConcussion => "頭面震盪",
            Self::TorsoInternal => "胸脅內傷",
            Self::ArmSprain => "腕肘挫傷",
            Self::LegSprain => "下盤扭傷",
            Self::Miasma => "瘴毒入體",
        }
    }

    pub fn symptom_prose(self) -> &'static str {
        match self {
            Self::HeadConcussion => "耳鳴目眩，凝神推演與識海專注略顯滯澀",
            Self::TorsoInternal => "胸脅氣血翻湧，周天運氣與發力受阻",
            Self::ArmSprain => "腕肘筋骨挫痛，運使手中器物與百工手藝略打折扣",
            Self::LegSprain => "下盤經絡扭傷，跋涉移步與閃轉騰挪較平時吃力",
            Self::Miasma => "濕熱瘴氣鬱結肺腑，須藉草藥湯劑或藥圃敷貼方可拔除",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribedManualRecord {
    pub manual_hash: String,
    pub author_id: String,
    pub author_name: String,
    pub formal_name: String,
    pub slot_label: String,
    pub carrier_code: String,
    pub word_ids: Vec<String>,
    pub word_chars: Vec<String>,
    pub origin_basis: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeastCorpseRecord {
    pub corpse_item_id: String,
    pub creature_title: String,
    pub primary_loot_id: String,
    pub primary_loot_name: String,
    pub shanhai_note: String,
    pub tier: u8,
    pub fresh_until: i64,
}

static AFFLICTION_CACHE: LazyLock<RwLock<HashMap<String, Vec<AfflictionKind>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static GROUND_DROPS_CACHE: LazyLock<RwLock<HashMap<(i32, i32), Vec<(String, String, i32)>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static MANUALS_CACHE: LazyLock<RwLock<HashMap<String, TranscribedManualRecord>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static CORPSES_CACHE: LazyLock<RwLock<HashMap<String, BeastCorpseRecord>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static CORPSE_SEQ: AtomicU64 = AtomicU64::new(1000);
static PG_AUX_TABLES_READY: LazyLock<RwLock<bool>> = LazyLock::new(|| RwLock::new(false));

fn ensure_aux_pg_tables() {
    if *PG_AUX_TABLES_READY.read().unwrap() {
        return;
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.batch_execute(
            "CREATE TABLE IF NOT EXISTS entity_body_injuries (
                entity_id TEXT NOT NULL,
                affliction_code TEXT NOT NULL,
                source_ref TEXT NOT NULL DEFAULT '',
                created_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, affliction_code)
            );
            CREATE TABLE IF NOT EXISTS ground_dropped_items (
                drop_id TEXT PRIMARY KEY,
                gx INTEGER NOT NULL,
                gy INTEGER NOT NULL,
                owner_id TEXT NOT NULL,
                item_id TEXT NOT NULL,
                item_name TEXT NOT NULL,
                qty INTEGER NOT NULL,
                claimed BOOLEAN NOT NULL DEFAULT FALSE,
                created_at BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS transcribed_manuals (
                manual_hash TEXT PRIMARY KEY,
                author_id TEXT NOT NULL,
                author_name TEXT NOT NULL,
                formal_name TEXT NOT NULL,
                slot_label TEXT NOT NULL,
                carrier_code TEXT NOT NULL,
                word_ids_json TEXT NOT NULL,
                word_chars_json TEXT NOT NULL,
                origin_basis TEXT NOT NULL,
                created_at BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS beast_corpse_instances (
                corpse_item_id TEXT PRIMARY KEY,
                creature_title TEXT NOT NULL,
                primary_loot_id TEXT NOT NULL,
                primary_loot_name TEXT NOT NULL,
                shanhai_note TEXT NOT NULL,
                tier SMALLINT NOT NULL DEFAULT 1,
                fresh_until BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS entity_breath_habits (
                entity_id TEXT PRIMARY KEY,
                dominant_code TEXT NOT NULL DEFAULT 'forest_long',
                forge_score INTEGER NOT NULL DEFAULT 0,
                forest_score INTEGER NOT NULL DEFAULT 0,
                tide_score INTEGER NOT NULL DEFAULT 0,
                chamber_score INTEGER NOT NULL DEFAULT 0,
                battle_score INTEGER NOT NULL DEFAULT 0,
                updated_at BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS entity_vessel_states (
                entity_id TEXT NOT NULL,
                item_id TEXT NOT NULL,
                use_count INTEGER NOT NULL DEFAULT 0,
                wear_points INTEGER NOT NULL DEFAULT 0,
                condition_code TEXT NOT NULL DEFAULT 'sharp',
                updated_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, item_id)
            );
            CREATE TABLE IF NOT EXISTS entity_lineage_imprints (
                entity_id TEXT NOT NULL,
                mentor_name TEXT NOT NULL,
                imprint_title TEXT NOT NULL,
                imprint_prose TEXT NOT NULL,
                interaction_count INTEGER NOT NULL DEFAULT 1,
                manual_gifted BOOLEAN NOT NULL DEFAULT FALSE,
                updated_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, mentor_name)
            );"
        );
        if let Ok(rows) = client.query(
            "SELECT manual_hash, author_id, author_name, formal_name, slot_label, carrier_code, word_ids_json, word_chars_json, origin_basis, created_at FROM transcribed_manuals",
            &[],
        ) {
            let mut guard = MANUALS_CACHE.write().unwrap();
            for r in rows {
                let manual_hash: String = r.get(0);
                let word_ids: Vec<String> = serde_json::from_str(r.get::<_, &str>(6)).unwrap_or_default();
                let word_chars: Vec<String> = serde_json::from_str(r.get::<_, &str>(7)).unwrap_or_default();
                guard.insert(
                    manual_hash.clone(),
                    TranscribedManualRecord {
                        manual_hash,
                        author_id: r.get(1),
                        author_name: r.get(2),
                        formal_name: r.get(3),
                        slot_label: r.get(4),
                        carrier_code: r.get(5),
                        word_ids,
                        word_chars,
                        origin_basis: r.get(8),
                        created_at: r.get(9),
                    },
                );
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, affliction_code FROM entity_body_injuries",
            &[],
        ) {
            let mut guard = AFFLICTION_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let code: String = r.get(1);
                if let Some(kind) = AfflictionKind::from_code(&code) {
                    let list = guard.entry(eid).or_default();
                    if !list.contains(&kind) {
                        list.push(kind);
                    }
                }
            }
        }
        if let Ok(rows) = client.query(
            "SELECT gx, gy, item_id, item_name, qty FROM ground_dropped_items WHERE claimed = FALSE",
            &[],
        ) {
            let mut guard = GROUND_DROPS_CACHE.write().unwrap();
            for r in rows {
                let gx: i32 = r.get(0);
                let gy: i32 = r.get(1);
                let item_id: String = r.get(2);
                let item_name: String = r.get(3);
                let qty: i32 = r.get(4);
                guard.entry((gx, gy)).or_default().push((item_id, item_name, qty));
            }
        }
        if let Ok(rows) = client.query(
            "SELECT corpse_item_id, creature_title, primary_loot_id, primary_loot_name, shanhai_note, tier, fresh_until FROM beast_corpse_instances",
            &[],
        ) {
            let mut guard = CORPSES_CACHE.write().unwrap();
            for r in rows {
                let cid: String = r.get(0);
                let tier_i16: i16 = r.get(5);
                guard.insert(
                    cid.clone(),
                    BeastCorpseRecord {
                        corpse_item_id: cid,
                        creature_title: r.get(1),
                        primary_loot_id: r.get(2),
                        primary_loot_name: r.get(3),
                        shanhai_note: r.get(4),
                        tier: tier_i16.max(1) as u8,
                        fresh_until: r.get(6),
                    },
                );
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, dominant_code, forge_score, forest_score, tide_score, chamber_score, battle_score, updated_at FROM entity_breath_habits",
            &[],
        ) {
            let mut guard = BREATH_HABIT_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let code: String = r.get(1);
                guard.insert(
                    eid.clone(),
                    BreathHabitRecord {
                        entity_id: eid,
                        dominant: BreathHabitKind::from_code(&code),
                        forge_score: r.get(2),
                        forest_score: r.get(3),
                        tide_score: r.get(4),
                        chamber_score: r.get(5),
                        battle_score: r.get(6),
                        updated_at: r.get(7),
                    },
                );
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, item_id, use_count, wear_points, condition_code, updated_at FROM entity_vessel_states",
            &[],
        ) {
            let mut guard = VESSEL_STATE_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let iid: String = r.get(1);
                let cond_code: String = r.get(4);
                guard.insert(
                    (eid.clone(), iid.clone()),
                    VesselStateRecord {
                        entity_id: eid,
                        item_id: iid,
                        use_count: r.get(2),
                        wear_points: r.get(3),
                        condition: VesselCondition::from_code(&cond_code),
                        updated_at: r.get(5),
                    },
                );
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, mentor_name, imprint_title, imprint_prose, interaction_count, manual_gifted, updated_at FROM entity_lineage_imprints",
            &[],
        ) {
            let mut guard = LINEAGE_IMPRINT_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let mentor: String = r.get(1);
                let list = guard.entry(eid.clone()).or_default();
                if !list.iter().any(|x| x.mentor_name == mentor) {
                    list.push(LineageImprintRecord {
                        entity_id: eid,
                        mentor_name: mentor,
                        imprint_title: r.get(2),
                        imprint_prose: r.get(3),
                        interaction_count: r.get(4),
                        manual_gifted: r.get(5),
                        updated_at: r.get(6),
                    });
                }
            }
        }
        *PG_AUX_TABLES_READY.write().unwrap() = true;
    }
}

pub fn record_entity_affliction(
    entity_id: &str,
    kind: AfflictionKind,
    source_ref: &str,
    now_unix: i64,
) {
    ensure_aux_pg_tables();
    {
        let mut guard = AFFLICTION_CACHE.write().unwrap();
        let list = guard.entry(entity_id.to_string()).or_default();
        if !list.contains(&kind) {
            list.push(kind);
        }
    }
    if let Some(pool) = crate::pg::pool() {
        let _ = open_obligation(
            &pool,
            entity_id,
            "treat_injury",
            kind.code(),
            Some(now_unix + 7200),
            source_ref,
            now_unix,
        );
        if let Ok(mut client) = pool.get() {
            let _ = client.execute(
                "INSERT INTO entity_body_injuries (entity_id, affliction_code, source_ref, created_at)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (entity_id, affliction_code) DO UPDATE SET source_ref = EXCLUDED.source_ref, created_at = EXCLUDED.created_at",
                &[&entity_id, &kind.code(), &source_ref, &now_unix],
            );
        }
    }
}

pub fn get_entity_afflictions(entity_id: &str) -> Vec<AfflictionKind> {
    ensure_aux_pg_tables();
    AFFLICTION_CACHE
        .read()
        .unwrap()
        .get(entity_id)
        .cloned()
        .unwrap_or_default()
}

pub fn clear_entity_afflictions(
    entity_id: &str,
    include_miasma: bool,
    now_unix: i64,
) -> Vec<AfflictionKind> {
    ensure_aux_pg_tables();
    let mut removed = Vec::new();
    let all_cleared = {
        let mut guard = AFFLICTION_CACHE.write().unwrap();
        if let Some(list) = guard.get_mut(entity_id) {
            let mut kept = Vec::new();
            for item in list.drain(..) {
                if item == AfflictionKind::Miasma && !include_miasma {
                    kept.push(item);
                } else {
                    removed.push(item);
                }
            }
            let has_remaining = !kept.is_empty();
            *list = kept;
            !has_remaining
        } else {
            true
        }
    };
    if !removed.is_empty()
        && let Some(pool) = crate::pg::pool()
    {
        if all_cleared {
            let _ = resolve_obligations_by_kind(&pool, entity_id, "treat_injury", now_unix);
        }
        if let Ok(mut client) = pool.get() {
            if include_miasma {
                let _ = client.execute(
                    "DELETE FROM entity_body_injuries WHERE entity_id = $1",
                    &[&entity_id],
                );
            } else {
                let _ = client.execute(
                    "DELETE FROM entity_body_injuries WHERE entity_id = $1 AND affliction_code <> 'miasma'",
                    &[&entity_id],
                );
            }
        }
    }
    removed
}

pub fn record_ground_dropped_items(
    gx: i32,
    gy: i32,
    owner_id: &str,
    items: &[(String, String, i32)],
    now_unix: i64,
) {
    if items.is_empty() {
        return;
    }
    ensure_aux_pg_tables();
    {
        let mut guard = GROUND_DROPS_CACHE.write().unwrap();
        let entry = guard.entry((gx, gy)).or_default();
        for (id, name, qty) in items {
            entry.push((id.clone(), name.clone(), *qty));
        }
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        for (id, name, qty) in items {
            let drop_id = Uuid::new_v4().to_string();
            let _ = client.execute(
                "INSERT INTO ground_dropped_items (drop_id, gx, gy, owner_id, item_id, item_name, qty, claimed, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, FALSE, $8)",
                &[&drop_id, &gx, &gy, &owner_id, id, name, qty, &now_unix],
            );
        }
    }
}

pub fn claim_ground_dropped_items(gx: i32, gy: i32) -> Vec<(String, String, i32)> {
    ensure_aux_pg_tables();
    let claimed = {
        let mut guard = GROUND_DROPS_CACHE.write().unwrap();
        guard.remove(&(gx, gy)).unwrap_or_default()
    };
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "UPDATE ground_dropped_items SET claimed = TRUE WHERE gx = $1 AND gy = $2 AND claimed = FALSE",
            &[&gx, &gy],
        );
    }
    claimed
}

pub fn peek_ground_dropped_items(gx: i32, gy: i32) -> Vec<(String, String, i32)> {
    ensure_aux_pg_tables();
    GROUND_DROPS_CACHE
        .read()
        .unwrap()
        .get(&(gx, gy))
        .cloned()
        .unwrap_or_default()
}

pub fn snapshot_all_ground_dropped_items() -> std::collections::HashMap<(i32, i32), Vec<(String, String, i32)>> {
    ensure_aux_pg_tables();
    GROUND_DROPS_CACHE.read().unwrap().clone()
}

pub fn save_transcribed_manual(record: &TranscribedManualRecord) {
    ensure_aux_pg_tables();
    {
        let mut guard = MANUALS_CACHE.write().unwrap();
        guard.insert(record.manual_hash.clone(), record.clone());
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let word_ids_json = serde_json::to_string(&record.word_ids).unwrap_or_else(|_| "[]".into());
        let word_chars_json =
            serde_json::to_string(&record.word_chars).unwrap_or_else(|_| "[]".into());
        let _ = client.execute(
            "INSERT INTO transcribed_manuals
               (manual_hash, author_id, author_name, formal_name, slot_label, carrier_code, word_ids_json, word_chars_json, origin_basis, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (manual_hash) DO UPDATE SET
               formal_name = EXCLUDED.formal_name,
               origin_basis = EXCLUDED.origin_basis",
            &[
                &record.manual_hash,
                &record.author_id,
                &record.author_name,
                &record.formal_name,
                &record.slot_label,
                &record.carrier_code,
                &word_ids_json,
                &word_chars_json,
                &record.origin_basis,
                &record.created_at,
            ],
        );
    }
}

pub fn get_transcribed_manual(manual_hash_or_item_id: &str) -> Option<TranscribedManualRecord> {
    ensure_aux_pg_tables();
    let key = manual_hash_or_item_id
        .strip_prefix("manual_")
        .unwrap_or(manual_hash_or_item_id);
    MANUALS_CACHE.read().unwrap().get(key).cloned()
}

pub fn register_beast_corpse(
    creature_title: &str,
    primary_loot_id: &str,
    primary_loot_name: &str,
    shanhai_note: &str,
    tier: u8,
    now_unix: i64,
) -> BeastCorpseRecord {
    ensure_aux_pg_tables();
    let seq = CORPSE_SEQ.fetch_add(1, Ordering::Relaxed);
    let corpse_item_id = format!("corpse_{}_{}", primary_loot_id.trim_start_matches("wt_"), seq + (now_unix.unsigned_abs() % 10000));
    let fresh_until = now_unix + 1800; // 30 分鐘內靈韻未散可整軀剝名
    let rec = BeastCorpseRecord {
        corpse_item_id: corpse_item_id.clone(),
        creature_title: creature_title.to_string(),
        primary_loot_id: primary_loot_id.to_string(),
        primary_loot_name: primary_loot_name.to_string(),
        shanhai_note: shanhai_note.to_string(),
        tier: tier.max(1),
        fresh_until,
    };
    {
        let mut guard = CORPSES_CACHE.write().unwrap();
        guard.insert(corpse_item_id.clone(), rec.clone());
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let tier_i16 = rec.tier as i16;
        let _ = client.execute(
            "INSERT INTO beast_corpse_instances
               (corpse_item_id, creature_title, primary_loot_id, primary_loot_name, shanhai_note, tier, fresh_until)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (corpse_item_id) DO NOTHING",
            &[
                &rec.corpse_item_id,
                &rec.creature_title,
                &rec.primary_loot_id,
                &rec.primary_loot_name,
                &rec.shanhai_note,
                &tier_i16,
                &rec.fresh_until,
            ],
        );
    }
    rec
}

pub fn get_beast_corpse(corpse_item_id: &str) -> Option<BeastCorpseRecord> {
    ensure_aux_pg_tables();
    CORPSES_CACHE.read().unwrap().get(corpse_item_id).cloned()
}

pub fn consume_beast_corpse(corpse_item_id: &str) {
    ensure_aux_pg_tables();
    CORPSES_CACHE.write().unwrap().remove(corpse_item_id);
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "DELETE FROM beast_corpse_instances WHERE corpse_item_id = $1",
            &[&corpse_item_id],
        );
    }
}

// ============================================================================
// 5. 自然養成之「內息底色（吐納習慣）」與「人器磨合 × 鋒刃器況」與「師承烙印」
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BreathHabitKind {
    /// 鍛爐沉息：常近爐火與重錘鍛打養成，吐納沉厚耐燥、蓄力剛猛
    ForgeHeavy,
    /// 山林長息：常年跋涉山林溪谷與狩獵採集養成，吐納綿長輕細、回氣輕快
    ForestLong,
    /// 海潮綿息：常處海濱潮汐、河堰水輪或舟楫勞作養成，氣息綿延不絕、耐受久戰
    TideContinuous,
    /// 靜室清息：常於靜室抄錄研讀、品茗調息或醫館搭脈養成，神思澄明、經脈繞路順暢
    ChamberClear,
    /// 百戰烈息：常歷生死交鋒與兵刃硬撼養成，氣血激盪如火、架勢重整迅疾
    BattleFierce,
}

impl BreathHabitKind {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ForgeHeavy => "forge_heavy",
            Self::ForestLong => "forest_long",
            Self::TideContinuous => "tide_continuous",
            Self::ChamberClear => "chamber_clear",
            Self::BattleFierce => "battle_fierce",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "forge_heavy" => Self::ForgeHeavy,
            "tide_continuous" => Self::TideContinuous,
            "chamber_clear" => Self::ChamberClear,
            "battle_fierce" => Self::BattleFierce,
            _ => Self::ForestLong,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::ForgeHeavy => "鍛爐沉息",
            Self::ForestLong => "山林長息",
            Self::TideContinuous => "海潮綿息",
            Self::ChamberClear => "靜室清息",
            Self::BattleFierce => "百戰烈息",
        }
    }

    pub fn somatic_description(&self) -> &'static str {
        match self {
            Self::ForgeHeavy => "常年近火揮錘、運使沉重器物所養成的吐納習慣。吸氣短促入腹、呼氣沉厚如風箱鼓盪，耐受燥熱與重負荷，重擊發勁時氣息格外凝實。",
            Self::ForestLong => "常年穿行山林草甸、涉水追蹤所養成的吐納習慣。氣息細長綿遠、起落無聲，跋涉與閃轉後回氣輕快，耳目對風吹草動尤為敏銳。",
            Self::TideContinuous => "常年順應潮汐水勢、於河海舟楫與水磨間勞作所養成的吐納習慣。氣機如潮汐疊浪、後勁綿延不絕，久戰之下氣脈不易潰散。",
            Self::ChamberClear => "常年靜心著譜研讀、品茗調息與體察氣脈所養成的吐納習慣。吐納勻淨澄明，體內氣機遇阻時能自然尋隙繞行，運轉字印更為省力。",
            Self::BattleFierce => "在連番貼身交鋒與兵刃硬撼中磨礪出的吐納習慣。臨敵時氣血如烘爐驟燃，受創或架勢動搖時能憑一口烈息迅速穩住下盤。",
        }
    }

    pub fn combat_rhythm_phrase(&self) -> &'static str {
        match self {
            Self::ForgeHeavy => "吐納沉如風箱鼓火，勁力自腰胯沉穩催發",
            Self::ForestLong => "呼吸綿長無聲，換氣間身形輕靈如林間游鹿",
            Self::TideContinuous => "內息如潮汐疊浪，一氣未盡、後勁已續",
            Self::ChamberClear => "神息澄明似水，氣機順支脈自然滑轉而不滯",
            Self::BattleFierce => "胸中一口烈息激盪，迎著鋒芒寸步不讓",
        }
    }

    pub fn combat_modifiers(&self) -> (f64, f64, f64, &'static str) {
        match self {
            Self::ForgeHeavy => (
                1.10,
                1.03,
                3.0,
                "【內息催動・鍛爐沉息】你吐納沉如風箱鼓火，腰胯沉穩催發重勁，出招力道更添三分凝實。",
            ),
            Self::ForestLong => (
                1.04,
                1.09,
                2.0,
                "【內息催動・山林長息】你呼吸綿長無聲，換氣間步法輕靈，閃轉卸力後回氣極快。",
            ),
            Self::TideContinuous => (
                1.05,
                1.10,
                2.0,
                "【內息催動・海潮綿息】你內息如潮汐疊浪綿延不絕，纏鬥間氣脈深長、後勁不衰。",
            ),
            Self::ChamberClear => (
                1.06,
                1.04,
                4.5,
                "【內息催動・靜室清息】你神息澄明似水，體內氣機順支脈自然滑轉，招式落點分毫不差。",
            ),
            Self::BattleFierce => (
                1.12,
                1.02,
                3.5,
                "【內息催動・百戰烈息】你胸中一口烈息如烘爐驟燃，臨敵硬撼時氣勢暴漲、寸步不讓。",
            ),
        }
    }

    pub fn craft_mutation_bonus(&self) -> f64 {
        match self {
            Self::ForgeHeavy => 0.06,
            Self::ChamberClear => 0.06,
            Self::TideContinuous => 0.05,
            Self::ForestLong => 0.04,
            Self::BattleFierce => 0.03,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BreathHabitRecord {
    pub entity_id: String,
    pub dominant: BreathHabitKind,
    pub forge_score: i32,
    pub forest_score: i32,
    pub tide_score: i32,
    pub chamber_score: i32,
    pub battle_score: i32,
    pub updated_at: i64,
}

static BREATH_HABIT_CACHE: LazyLock<RwLock<HashMap<String, BreathHabitRecord>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_or_derive_breath_habit(
    entity_id: &str,
    vocation_hint: &str,
    soul_seed: i64,
) -> BreathHabitRecord {
    ensure_aux_pg_tables();
    if let Some(rec) = BREATH_HABIT_CACHE.read().unwrap().get(entity_id).cloned() {
        return rec;
    }
    let default_kind = if vocation_hint.contains("鍛")
        || vocation_hint.contains("鐵")
        || vocation_hint.contains("礦")
        || vocation_hint.contains("石")
    {
        BreathHabitKind::ForgeHeavy
    } else if vocation_hint.contains("漁")
        || vocation_hint.contains("海")
        || vocation_hint.contains("港")
        || vocation_hint.contains("磨")
        || vocation_hint.contains("渡")
    {
        BreathHabitKind::TideContinuous
    } else if vocation_hint.contains("醫")
        || vocation_hint.contains("院")
        || vocation_hint.contains("書")
        || vocation_hint.contains("茶")
    {
        BreathHabitKind::ChamberClear
    } else if vocation_hint.contains("衛")
        || vocation_hint.contains("武")
        || vocation_hint.contains("巡")
    {
        BreathHabitKind::BattleFierce
    } else if vocation_hint.contains("獵")
        || vocation_hint.contains("林")
        || vocation_hint.contains("樵")
        || vocation_hint.contains("採")
    {
        BreathHabitKind::ForestLong
    } else {
        let effective_seed = if soul_seed != 0 {
            soul_seed.unsigned_abs()
        } else {
            entity_id.bytes().fold(1u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
        };
        match effective_seed % 5 {
            0 => BreathHabitKind::ForgeHeavy,
            1 => BreathHabitKind::ForestLong,
            2 => BreathHabitKind::TideContinuous,
            3 => BreathHabitKind::ChamberClear,
            _ => BreathHabitKind::BattleFierce,
        }
    };
    let rec = BreathHabitRecord {
        entity_id: entity_id.to_string(),
        dominant: default_kind,
        forge_score: if default_kind == BreathHabitKind::ForgeHeavy { 6 } else { 4 },
        forest_score: if default_kind == BreathHabitKind::ForestLong { 6 } else { 4 },
        tide_score: if default_kind == BreathHabitKind::TideContinuous { 6 } else { 4 },
        chamber_score: if default_kind == BreathHabitKind::ChamberClear { 6 } else { 4 },
        battle_score: if default_kind == BreathHabitKind::BattleFierce { 6 } else { 4 },
        updated_at: 0,
    };
    BREATH_HABIT_CACHE
        .write()
        .unwrap()
        .insert(entity_id.to_string(), rec.clone());
    rec
}

pub fn record_breath_activity(
    entity_id: &str,
    habit: BreathHabitKind,
    weight: i32,
    now_unix: i64,
) -> BreathHabitRecord {
    ensure_aux_pg_tables();
    let delta = weight.max(1);
    let updated = {
        let mut guard = BREATH_HABIT_CACHE.write().unwrap();
        let entry = guard.entry(entity_id.to_string()).or_insert_with(|| BreathHabitRecord {
            entity_id: entity_id.to_string(),
            dominant: habit,
            forge_score: 4,
            forest_score: 4,
            tide_score: 4,
            chamber_score: 4,
            battle_score: 4,
            updated_at: now_unix,
        });
        match habit {
            BreathHabitKind::ForgeHeavy => entry.forge_score += delta,
            BreathHabitKind::ForestLong => entry.forest_score += delta,
            BreathHabitKind::TideContinuous => entry.tide_score += delta,
            BreathHabitKind::ChamberClear => entry.chamber_score += delta,
            BreathHabitKind::BattleFierce => entry.battle_score += delta,
        }
        entry.updated_at = now_unix;
        // 慣性法則：新習慣分數必須超越現有主導底色至少 4 分才會自然轉移，防止一秒切換
        let current_score = match entry.dominant {
            BreathHabitKind::ForgeHeavy => entry.forge_score,
            BreathHabitKind::ForestLong => entry.forest_score,
            BreathHabitKind::TideContinuous => entry.tide_score,
            BreathHabitKind::ChamberClear => entry.chamber_score,
            BreathHabitKind::BattleFierce => entry.battle_score,
        };
        let candidates = [
            (BreathHabitKind::ForgeHeavy, entry.forge_score),
            (BreathHabitKind::ForestLong, entry.forest_score),
            (BreathHabitKind::TideContinuous, entry.tide_score),
            (BreathHabitKind::ChamberClear, entry.chamber_score),
            (BreathHabitKind::BattleFierce, entry.battle_score),
        ];
        if let Some((best_kind, best_score)) = candidates.into_iter().max_by_key(|(_, s)| *s)
            && best_kind != entry.dominant
            && best_score >= current_score + 4
        {
            entry.dominant = best_kind;
        }
        entry.clone()
    };
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "INSERT INTO entity_breath_habits
               (entity_id, dominant_code, forge_score, forest_score, tide_score, chamber_score, battle_score, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (entity_id) DO UPDATE SET
               dominant_code = EXCLUDED.dominant_code,
               forge_score = EXCLUDED.forge_score,
               forest_score = EXCLUDED.forest_score,
               tide_score = EXCLUDED.tide_score,
               chamber_score = EXCLUDED.chamber_score,
               battle_score = EXCLUDED.battle_score,
               updated_at = EXCLUDED.updated_at",
            &[
                &updated.entity_id,
                &updated.dominant.code(),
                &updated.forge_score,
                &updated.forest_score,
                &updated.tide_score,
                &updated.chamber_score,
                &updated.battle_score,
                &updated.updated_at,
            ],
        );
    }
    updated
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VesselFamiliarity {
    /// 初握生澀
    Unfamiliar,
    /// 漸趨順手
    GettingSmooth,
    /// 沉穩趁手
    WellBalanced,
    /// 如臂使指
    LikeOwnArm,
}

impl VesselFamiliarity {
    pub fn from_use_count(use_count: i32) -> Self {
        match use_count {
            0..=2 => Self::Unfamiliar,
            3..=8 => Self::GettingSmooth,
            9..=19 => Self::WellBalanced,
            _ => Self::LikeOwnArm,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Unfamiliar => "初握生澀",
            Self::GettingSmooth => "漸趨順手",
            Self::WellBalanced => "沉穩趁手",
            Self::LikeOwnArm => "如臂使指",
        }
    }

    pub fn feel_prose(&self) -> &'static str {
        match self {
            Self::Unfamiliar => "器物重心尚待摸索，運使時須多留一分力道收束",
            Self::GettingSmooth => "手掌已漸漸習慣握柄紋理，起落收發初見流暢",
            Self::WellBalanced => "分寸輕重皆在掌握，隨手一引便能順勢入勁",
            Self::LikeOwnArm => "人器氣機渾然相通，心念動處鋒芒已至，恍若肢體延伸",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VesselCondition {
    /// 鋒芒畢露（完好銳利）
    Sharp,
    /// 刃口微鈍（可於街邊磨刀石推磨開鋒）
    SlightlyDull,
    /// 捲刃待修（須於鍛造工坊借爐火鐵砧回火修整）
    RolledEdge,
}

impl VesselCondition {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sharp => "sharp",
            Self::SlightlyDull => "slightly_dull",
            Self::RolledEdge => "rolled_edge",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "slightly_dull" => Self::SlightlyDull,
            "rolled_edge" => Self::RolledEdge,
            _ => Self::Sharp,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Sharp => "鋒芒畢露",
            Self::SlightlyDull => "刃口微鈍",
            Self::RolledEdge => "捲刃待修",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VesselStateRecord {
    pub entity_id: String,
    pub item_id: String,
    pub use_count: i32,
    pub wear_points: i32,
    pub condition: VesselCondition,
    pub updated_at: i64,
}

impl VesselStateRecord {
    pub fn familiarity(&self) -> VesselFamiliarity {
        VesselFamiliarity::from_use_count(self.use_count)
    }

    pub fn status_badge(&self) -> String {
        format!("{}・{}", self.familiarity().label(), self.condition.label())
    }
}

static VESSEL_STATE_CACHE: LazyLock<RwLock<HashMap<(String, String), VesselStateRecord>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_vessel_state(entity_id: &str, item_id: &str) -> VesselStateRecord {
    ensure_aux_pg_tables();
    if let Some(rec) = VESSEL_STATE_CACHE
        .read()
        .unwrap()
        .get(&(entity_id.to_string(), item_id.to_string()))
        .cloned()
    {
        return rec;
    }
    VesselStateRecord {
        entity_id: entity_id.to_string(),
        item_id: item_id.to_string(),
        use_count: 0,
        wear_points: 0,
        condition: VesselCondition::Sharp,
        updated_at: 0,
    }
}

fn persist_vessel_state(rec: &VesselStateRecord) {
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "INSERT INTO entity_vessel_states
               (entity_id, item_id, use_count, wear_points, condition_code, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (entity_id, item_id) DO UPDATE SET
               use_count = EXCLUDED.use_count,
               wear_points = EXCLUDED.wear_points,
               condition_code = EXCLUDED.condition_code,
               updated_at = EXCLUDED.updated_at",
            &[
                &rec.entity_id,
                &rec.item_id,
                &rec.use_count,
                &rec.wear_points,
                &rec.condition.code(),
                &rec.updated_at,
            ],
        );
    }
}

pub fn record_vessel_use(
    entity_id: &str,
    item_id: &str,
    wear_delta: i32,
    familiarity_delta: i32,
    now_unix: i64,
) -> VesselStateRecord {
    ensure_aux_pg_tables();
    let updated = {
        let mut guard = VESSEL_STATE_CACHE.write().unwrap();
        let entry = guard
            .entry((entity_id.to_string(), item_id.to_string()))
            .or_insert_with(|| VesselStateRecord {
                entity_id: entity_id.to_string(),
                item_id: item_id.to_string(),
                use_count: 0,
                wear_points: 0,
                condition: VesselCondition::Sharp,
                updated_at: now_unix,
            });
        entry.use_count = (entry.use_count + familiarity_delta.max(0)).min(9999);
        entry.wear_points = (entry.wear_points + wear_delta.max(0)).min(100);
        entry.condition = if entry.wear_points >= 8 {
            VesselCondition::RolledEdge
        } else if entry.wear_points >= 3 {
            VesselCondition::SlightlyDull
        } else {
            VesselCondition::Sharp
        };
        entry.updated_at = now_unix;
        entry.clone()
    };
    persist_vessel_state(&updated);
    updated
}

pub fn sharpen_vessel_at_grindstone(
    entity_id: &str,
    item_id: &str,
    now_unix: i64,
) -> Result<VesselStateRecord, &'static str> {
    ensure_aux_pg_tables();
    let current = get_vessel_state(entity_id, item_id);
    if current.condition == VesselCondition::RolledEdge {
        return Err("此器刃口已然捲曲崩缺，尋常青石磨刀槽難以推平，須前往鍛造工坊借爐火與鐵砧回火修整。");
    }
    let updated = {
        let mut guard = VESSEL_STATE_CACHE.write().unwrap();
        let entry = guard
            .entry((entity_id.to_string(), item_id.to_string()))
            .or_insert(current.clone());
        if current.wear_points > 0 || current.condition != VesselCondition::Sharp {
            entry.use_count = (entry.use_count + 2).min(9999);
        }
        entry.wear_points = 0;
        entry.condition = VesselCondition::Sharp;
        entry.updated_at = now_unix;
        entry.clone()
    };
    persist_vessel_state(&updated);
    Ok(updated)
}

pub fn temper_vessel_at_forge(
    entity_id: &str,
    item_id: &str,
    now_unix: i64,
) -> VesselStateRecord {
    ensure_aux_pg_tables();
    let current = get_vessel_state(entity_id, item_id);
    let updated = {
        let mut guard = VESSEL_STATE_CACHE.write().unwrap();
        let entry = guard
            .entry((entity_id.to_string(), item_id.to_string()))
            .or_insert(current);
        entry.use_count = (entry.use_count + 3).min(9999);
        entry.wear_points = 0;
        entry.condition = VesselCondition::Sharp;
        entry.updated_at = now_unix;
        entry.clone()
    };
    persist_vessel_state(&updated);
    updated
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LineageImprintRecord {
    pub entity_id: String,
    pub mentor_name: String,
    pub imprint_title: String,
    pub imprint_prose: String,
    pub interaction_count: i32,
    pub manual_gifted: bool,
    pub updated_at: i64,
}

static LINEAGE_IMPRINT_CACHE: LazyLock<RwLock<HashMap<String, Vec<LineageImprintRecord>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn record_mentor_imprint_progress(
    entity_id: &str,
    mentor_name: &str,
    imprint_title: &str,
    imprint_prose: &str,
    now_unix: i64,
) -> (LineageImprintRecord, bool) {
    ensure_aux_pg_tables();
    let (rec, newly_gifted) = {
        let mut guard = LINEAGE_IMPRINT_CACHE.write().unwrap();
        let list = guard.entry(entity_id.to_string()).or_default();
        if let Some(existing) = list.iter_mut().find(|r| r.mentor_name == mentor_name) {
            existing.interaction_count += 1;
            existing.updated_at = now_unix;
            let gift_now = existing.interaction_count >= 2 && !existing.manual_gifted;
            if gift_now {
                existing.manual_gifted = true;
            }
            (existing.clone(), gift_now)
        } else {
            let new_rec = LineageImprintRecord {
                entity_id: entity_id.to_string(),
                mentor_name: mentor_name.to_string(),
                imprint_title: imprint_title.to_string(),
                imprint_prose: imprint_prose.to_string(),
                interaction_count: 1,
                manual_gifted: false,
                updated_at: now_unix,
            };
            list.push(new_rec.clone());
            (new_rec, false)
        }
    };
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "INSERT INTO entity_lineage_imprints
               (entity_id, mentor_name, imprint_title, imprint_prose, interaction_count, manual_gifted, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (entity_id, mentor_name) DO UPDATE SET
               imprint_title = EXCLUDED.imprint_title,
               imprint_prose = EXCLUDED.imprint_prose,
               interaction_count = EXCLUDED.interaction_count,
               manual_gifted = EXCLUDED.manual_gifted,
               updated_at = EXCLUDED.updated_at",
            &[
                &rec.entity_id,
                &rec.mentor_name,
                &rec.imprint_title,
                &rec.imprint_prose,
                &rec.interaction_count,
                &rec.manual_gifted,
                &rec.updated_at,
            ],
        );
    }
    (rec, newly_gifted)
}

pub fn get_entity_lineage_imprints(entity_id: &str) -> Vec<LineageImprintRecord> {
    ensure_aux_pg_tables();
    LINEAGE_IMPRINT_CACHE
        .read()
        .unwrap()
        .get(entity_id)
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_detects_the_all_gather_pathology() {
        let audit = LivingWorldAudit {
            character_count: 2,
            npc_count: 2,
            continuity_count: 2,
            autonomous_count: 2,
            obligation_count: 0,
            relationship_count: 0,
            knowledge_count: 0,
            action_receipt_count: 0,
            source_record_count: 0,
            distinct_npc_activities: 1,
            gathering_npc_count: 2,
            trade_count: 0,
            trade_receipt_count: 0,
            unbalanced_trade_count: 0,
            work_count: 0,
            work_receipt_count: 0,
            unbalanced_work_count: 0,
        };
        assert!(audit.all_npcs_gathering());
    }

    #[test]
    fn breath_habit_has_inertia_and_vessel_grindstone_respects_rolled_edge() {
        let h0 = record_breath_activity("test_ent_habit", BreathHabitKind::ForestLong, 5, 100);
        assert_eq!(h0.dominant, BreathHabitKind::ForestLong);
        // Small activity in another habit should not immediately flip dominant habit (inertia)
        let h1 = record_breath_activity("test_ent_habit", BreathHabitKind::ForgeHeavy, 3, 101);
        assert_eq!(h1.dominant, BreathHabitKind::ForestLong);
        // Sustained forge work shifts the breath habit
        let h2 = record_breath_activity("test_ent_habit", BreathHabitKind::ForgeHeavy, 10, 102);
        assert_eq!(h2.dominant, BreathHabitKind::ForgeHeavy);

        // Vessel state progression & grindstone vs forge repair
        let v1 = record_vessel_use("test_ent_vessel", "wt_boning_knife", 4, 3, 100);
        assert_eq!(v1.condition, VesselCondition::SlightlyDull);
        assert_eq!(v1.familiarity(), VesselFamiliarity::GettingSmooth);
        let sharpened = sharpen_vessel_at_grindstone("test_ent_vessel", "wt_boning_knife", 105).unwrap();
        assert_eq!(sharpened.condition, VesselCondition::Sharp);
        let initial_use_count = sharpened.use_count;
        // Grindstone on an already sharp and unworn blade does NOT increment use_count (F8)
        let sharpened_again = sharpen_vessel_at_grindstone("test_ent_vessel", "wt_boning_knife", 106).unwrap();
        assert_eq!(sharpened_again.use_count, initial_use_count);
        let v_rolled = record_vessel_use("test_ent_vessel", "wt_boning_knife", 9, 5, 110);
        assert_eq!(v_rolled.condition, VesselCondition::RolledEdge);
        assert!(sharpen_vessel_at_grindstone("test_ent_vessel", "wt_boning_knife", 115).is_err());
        let tempered = temper_vessel_at_forge("test_ent_vessel", "wt_boning_knife", 120);
        assert_eq!(tempered.condition, VesselCondition::Sharp);
        assert_eq!(tempered.familiarity(), VesselFamiliarity::WellBalanced);
    }

    #[test]
    fn end_to_end_organic_living_world_nine_systems_loop() {
        let pid = "player_e2e_living_loop";
        let now = 1_760_000_000i64;

        // 1. 部位傷勢與分級療傷
        record_entity_affliction(pid, AfflictionKind::LegSprain, "cliff_fall", now);
        record_entity_affliction(pid, AfflictionKind::Miasma, "swamp_fog", now);
        assert_eq!(get_entity_afflictions(pid).len(), 2);
        let _ = clear_entity_afflictions(pid, false, now + 5); // 一般止血散不拔除瘴毒
        assert_eq!(get_entity_afflictions(pid), vec![AfflictionKind::Miasma]);
        let _ = clear_entity_afflictions(pid, true, now + 8); // 診所／草藥拔除瘴毒
        assert!(get_entity_afflictions(pid).is_empty());

        // 2. 氣機自動繞路與動中體察（零數字、零 G0 代碼）
        let somatic = crate::combat::wordplate::somatic_meridian_awareness(42_4242);
        assert!(!somatic.is_empty());
        assert!(!somatic.chars().any(|c| c.is_ascii_digit()));
        assert!(!somatic.contains("G0"));

        // 3. 內息底色養成與慣性
        let h_init = get_or_derive_breath_habit(pid, "山林獵戶", 42_4242);
        assert_eq!(h_init.dominant, BreathHabitKind::ForestLong);
        let h_minor = record_breath_activity(pid, BreathHabitKind::ForgeHeavy, 2, now + 10);
        assert_eq!(h_minor.dominant, BreathHabitKind::ForestLong);
        let h_shifted = record_breath_activity(pid, BreathHabitKind::ForgeHeavy, 8, now + 20);
        assert_eq!(h_shifted.dominant, BreathHabitKind::ForgeHeavy);

        // 4. 人器磨合 × 鋒刃器況與街邊磨刀槽 vs 鍛爐回火
        let v_used = record_vessel_use(pid, "craft_steel_hunting_knife", 4, 4, now + 30);
        assert_eq!(v_used.condition, VesselCondition::SlightlyDull);
        assert_eq!(v_used.familiarity(), VesselFamiliarity::GettingSmooth);
        let v_honed = sharpen_vessel_at_grindstone(pid, "craft_steel_hunting_knife", now + 40).unwrap();
        assert_eq!(v_honed.condition, VesselCondition::Sharp);
        let use_after_hone = v_honed.use_count;
        // 對已完好之刀重複推磨不無限刷磨合度
        let v_hone_again = sharpen_vessel_at_grindstone(pid, "craft_steel_hunting_knife", now + 45).unwrap();
        assert_eq!(v_hone_again.use_count, use_after_hone);

        // 5. 師承烙印與親筆傳世手札觸發
        let (imp1, gifted1) = record_mentor_imprint_progress(
            pid,
            "岑鐵匠",
            "岑氏沉腕落錘法",
            "沉腕蓄勁、順紋落錘",
            now + 50,
        );
        assert_eq!(imp1.interaction_count, 1);
        assert!(!gifted1);
        let (imp2, gifted2) = record_mentor_imprint_progress(
            pid,
            "岑鐵匠",
            "岑氏沉腕落錘法",
            "沉腕蓄勁、順紋落錘",
            now + 60,
        );
        assert_eq!(imp2.interaction_count, 2);
        assert!(gifted2);

        // 6. 百工高光分層 LLM 數據轉描述與事實保真護欄
        let craft_pkt = crate::ai::CraftActionFactPacket {
            actor_name: "林山行".to_string(),
            tool_name: "鍛鐵重錘".to_string(),
            tool_familiarity: "沉穩趁手".to_string(),
            breath_habit: "鍛爐沉息".to_string(),
            material_names: vec!["赤鐵礦".to_string(), "硬木炭".to_string()],
            output_name: "精鍛防身獵刀".to_string(),
            highlight_kind: "工坊器物異變".to_string(),
            environment_hint: "文曜分家巷鍛爐火光熾烈".to_string(),
        };
        let craft_prose = crate::ai::compose_craft_highlight_from_data(&craft_pkt);
        assert!(crate::ai::verify_craft_output(&craft_prose, &craft_pkt).is_ok());
    }
}

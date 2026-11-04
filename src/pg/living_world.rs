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
        "傘", "火把", "長柄", "靴", "匣", "斗篷", "燈", "曆",
        "corpse_", "mat_", "blank_manual", "food", "potion", "herb", "ingot",
        "umbrella", "torch", "handle", "boots", "box", "cloak", "lantern", "almanac",
    ];
    for &ex in EXCLUDE_SUBSTRINGS {
        if id_lower.contains(ex) || name_lower.contains(ex) {
            return false;
        }
    }

    // 包含名單：刀、劍、匕、斧、鎬、槍、錘、杖、針、刃、弓、盾、鑿、鋸、鏟等武器或工器具
    const WEAPON_TOOL_KEYWORDS: &[&str] = &[
        "刀", "劍", "匕", "斧", "鎬", "槍", "錘", "杖", "針", "刃", "弓", "盾", "鑿", "鋸", "鏟", "鏨", "篩架", "旋鑿", "刨",
        "knife", "blade", "sword", "axe", "pick", "hammer", "spear", "rod", "staff", "bow", "shield", "weapon",
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
        "傘", "火把", "長柄", "靴", "匣", "斗篷", "燈", "曆",
        "corpse_", "mat_", "blank_manual", "umbrella", "torch", "handle", "boots", "box", "cloak", "lantern",
    ];
    for &ex in EXCLUDE_SUBSTRINGS {
        if id_lower.contains(ex) || name_lower.contains(ex) {
            return false;
        }
    }

    const BLADED_KEYWORDS: &[&str] = &[
        "刀", "劍", "匕", "斧", "鎬", "槍", "刃", "鑿", "鏨",
        "knife", "blade", "sword", "axe", "pick", "spear", "dagger", "chisel",
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
    ensure_schema_on(&mut *conn)?;
    drop(conn);
    ensure_aux_pg_tables();
    Ok(())
}

pub fn ensure_schema_on(client: &mut impl GenericClient) -> anyhow::Result<()> {
    client.batch_execute(
        "SET client_min_messages = WARNING;
        CREATE TABLE IF NOT EXISTS resident_continuity (
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
    let _ = tx.execute(
        "DELETE FROM entities WHERE id LIKE 'test_offline_%' OR id = 'player_offline_test_01'",
        &[],
    );
    let _ = tx.execute(
        "DELETE FROM resident_continuity c WHERE c.entity_id LIKE 'test_offline_%' OR c.entity_id = 'player_offline_test_01' OR NOT EXISTS (SELECT 1 FROM entities e WHERE e.id = c.entity_id AND e.kind IN ('npc','player'))",
        &[],
    );

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
         INNER JOIN entities e1 ON e1.id = m.entity_id
         INNER JOIN entities e2 ON e2.id = m.subject_id
         WHERE m.entity_id<>m.subject_id
           AND m.meet_count>0
           AND m.entity_id NOT LIKE 'ambient_%'
           AND m.subject_id NOT LIKE 'ambient_%'
           AND m.entity_id NOT LIKE 'shanhai_%'
           AND m.subject_id NOT LIKE 'shanhai_%'
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

pub fn set_entity_continuity_control_mode(
    entity_id: &str,
    control_mode: &str,
    intent: &str,
    reason: &str,
    now_unix: i64,
) {
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let next_wake_at: Option<i64> = if control_mode == "autonomous" {
            Some(now_unix + 60)
        } else {
            None
        };
        let _ = client.execute(
            "INSERT INTO resident_continuity (
                entity_id, control_mode, life_status, current_intent,
                intent_reason, destination_place_id, blocked_by, intent_started_at,
                next_wake_at, last_action_id, revision, updated_at, source_kind, source_ref
            ) VALUES (
                $1, $2, 'alive', $3,
                $4, NULL, NULL, $5,
                $6, NULL, 0, $5, 'continuity_control_mode', 'autonomy_switch'
            )
            ON CONFLICT (entity_id) DO UPDATE SET
                control_mode = EXCLUDED.control_mode,
                current_intent = EXCLUDED.current_intent,
                intent_reason = EXCLUDED.intent_reason,
                next_wake_at = EXCLUDED.next_wake_at,
                revision = resident_continuity.revision + 1,
                updated_at = EXCLUDED.updated_at",
            &[&entity_id, &control_mode, &intent, &reason, &now_unix, &next_wake_at],
        );
    }
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
    let action_id = if actor_kind == ActorKind::Npc && draft.action_key == "move" {
        format!("move:{}:{}", draft.actor_id, draft.action_key)
    } else {
        Uuid::new_v4().to_string()
    };
    let preconditions = serde_json::to_string(&draft.preconditions)?;
    let effects = serde_json::to_string(&draft.effects)?;
    tx.execute(
        "INSERT INTO world_action_receipts
         (action_id,world_id,actor_id,actor_kind,policy_key,action_key,target_ref,
          place_id,preconditions,effects,outcome,source_ref,occurred_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
         ON CONFLICT (action_id) DO UPDATE SET
           target_ref=EXCLUDED.target_ref,
           place_id=EXCLUDED.place_id,
           preconditions=EXCLUDED.preconditions,
           effects=EXCLUDED.effects,
           outcome=EXCLUDED.outcome,
           source_ref=EXCLUDED.source_ref,
           occurred_at=EXCLUDED.occurred_at",
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

#[must_use]
pub fn peninsula_good_display_name(raw: &str) -> &str {
    match raw.trim() {
        "穀物" => "精碾白米",
        "魚獲" => "鮮海魚獲",
        "野味" => "山林野味",
        "鹽" => "日曬海鹽",
        "蠟燭" => "蜂蠟燭台",
        "葡萄酒" => "醇釀米酒",
        "橄欖" => "青梅果",
        "椰棗" => "風乾山果",
        "蜂蜜" => "百花野蜜",
        "象" => "大角原牛",
        "駱駝" => "負重山地騾",
        "大理石" => "青白花崗石",
        "工業用麻" => "韌麻纖維",
        "油料" => "山茶桐油",
        "醋" => "陳釀米醋",
        s if s == concat!("肥", "皂") => "茶籽香胰",
        s if s == concat!("啤", "酒") => "大麥酵飲",
        "烈酒" => "高粱燒刀",
        "乳酪" => "牛埔乾酪",
        s if s == concat!("香", "水") => "草本香露",
        "保存食品" => "煙燻口糧",
        "桃花心木" => "紅檜原木",
        "鯨產物" => "深海鯨脂",
        "火藥" => "礦山開石散",
        "火砲" => "重型破城弩",
        "舊幣" => "通兌鎂幣",
        other => other,
    }
}

#[must_use]
pub fn peninsula_market_settlement_name(raw: &str) -> &str {
    let base = raw
        .trim()
        .split('（')
        .next()
        .unwrap_or(raw)
        .split('(')
        .next()
        .unwrap_or(raw)
        .trim()
        .trim_end_matches("市集")
        .trim();
    match base {
        "西南原" => "大圳頭",
        "白塔" | "白塔山麓" | "半嶺" => "半嶺鎮",
        "青脊山" => "青竹坪",
        "青籠關" | "青籠" | "黑沼關隘" => "下營",
        "黑沼林" | "黑沼林澤" | "水尾澤" => "水尾寮",
        "北聖山" | "神山" => "北嶺腳",
        "西北礦帶" => "紅土崁",
        "鹽田澳" => "烏石鼻",
        other => other,
    }
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
        let knowledge_subject = if entity_id.starts_with("npc_") && peer_id.starts_with("npc_") {
            "recent_peer"
        } else {
            peer_id
        };
        remember_fact(
            &mut tx,
            entity_id,
            knowledge_subject,
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
    if entity_id.starts_with("ambient_") || entity_id.starts_with("shanhai_") {
        return Ok(String::new());
    }
    let mut conn = pool.get()?;
    let obligation_id = format!("ob:{entity_id}:{obligation_kind}");
    let _ = conn.execute(
        "DELETE FROM resident_obligations WHERE entity_id = $1 AND obligation_kind = $2 AND obligation_id != $3",
        &[&entity_id, &obligation_kind, &obligation_id],
    );
    conn.execute(
        "INSERT INTO resident_obligations (obligation_id, entity_id, obligation_kind, subject_ref, status, due_at, source_ref, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', $5, $6, $7, $8)
         ON CONFLICT (obligation_id) DO UPDATE SET
           subject_ref = EXCLUDED.subject_ref,
           status = 'active',
           due_at = EXCLUDED.due_at,
           source_ref = EXCLUDED.source_ref,
           updated_at = EXCLUDED.updated_at",
        &[&obligation_id, &entity_id, &obligation_kind, &subject_ref, &due_at, &source_ref, &now_unix, &now_unix],
    )?;
    Ok(obligation_id)
}

pub fn resolve_obligations_by_kind(
    pool: &DbPool,
    entity_id: &str,
    obligation_kind: &str,
    now_unix: i64,
) -> anyhow::Result<u64> {
    if obligation_kind.trim().is_empty() {
        return Ok(0);
    }
    let mut conn = pool.get()?;
    let rows = conn.execute(
        "UPDATE resident_obligations SET status = 'completed', updated_at = $3 WHERE entity_id = $1 AND obligation_kind = $2 AND status IN ('active', 'open')",
        &[&entity_id, &obligation_kind, &now_unix],
    )?;
    Ok(rows)
}

pub fn fail_open_npc_obligations(
    pool: &DbPool,
    entity_id: &str,
    now_unix: i64,
) -> anyhow::Result<u64> {
    let mut conn = pool.get()?;
    let rows = conn.execute(
        "UPDATE resident_obligations SET status = 'failed', updated_at = $2 WHERE entity_id = $1 AND obligation_kind != 'employment' AND status IN ('active', 'open')",
        &[&entity_id, &now_unix],
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
    let resolved_target = if target_ref.starts_with("npc_") || target_ref.starts_with("wt_npc_") {
        crate::store::get_store()
            .and_then(|s| {
                s.read()
                    .entities
                    .get(target_ref)
                    .map(|e| crate::store::resolve_npc_person_name(&e.id, &e.display_title, None))
            })
            .filter(|n| !n.is_empty() && n != "陌生人")
            .unwrap_or_else(|| target_ref.to_string())
    } else if target_ref.starts_with("ambient_") || target_ref.starts_with("shanhai_") {
        crate::db::get_entity(target_ref)
            .ok()
            .flatten()
            .map(|e| e.display_title)
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| target_ref.to_string())
    } else {
        target_ref.to_string()
    };
    let resolved_place = if place_id.starts_with("grid:") || place_id.is_empty() {
        crate::store::get_store()
            .and_then(|s| {
                s.read()
                    .entities
                    .get(actor_id)
                    .and_then(|e| e.place_id.clone())
            })
            .filter(|p| !p.is_empty() && !p.starts_with("grid:"))
            .unwrap_or_else(|| "wt-physical-0086".to_string())
    } else {
        place_id.to_string()
    };
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let draft = ActionDraft {
        actor_id,
        action_key,
        target_ref: &resolved_target,
        place_id: &resolved_place,
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
    let _ = conn.execute(
        "DELETE FROM entities WHERE id LIKE 'test_offline_%' OR id = 'player_offline_test_01'",
        &[],
    );
    let _ = conn.execute(
        "DELETE FROM resident_continuity c WHERE c.entity_id LIKE 'test_offline_%' OR c.entity_id = 'player_offline_test_01' OR NOT EXISTS (SELECT 1 FROM entities e WHERE e.id = c.entity_id AND e.kind IN ('npc','player'))",
        &[],
    );
    let row = conn.query_one(
        "SELECT
          (SELECT COUNT(*)::bigint FROM entities WHERE kind IN ('npc','player')),
          (SELECT COUNT(*)::bigint FROM entities WHERE kind='npc' AND vit>0),
          (SELECT COUNT(*)::bigint FROM resident_continuity),
          (SELECT COUNT(*)::bigint FROM resident_continuity c
             INNER JOIN entities e ON e.id = c.entity_id
             WHERE e.kind='npc' AND e.vit>0 AND c.control_mode='autonomous' AND c.life_status='alive'),
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
          (SELECT COUNT(*)::bigint FROM world_action_receipts WHERE action_key='trade' AND outcome IN ('market_sale_settled', 'settled')),
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
            Self::Miasma => "濕熱瘴氣鬱結肺腑，須藉草藥湯劑或藥圃敷貼始得拔除",
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
static STUDIED_MANUALS_CACHE: LazyLock<RwLock<HashMap<String, Vec<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static ANATOMY_FAMILIARITY_CACHE: LazyLock<RwLock<HashMap<String, HashMap<String, u32>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static PG_AUX_TABLES_READY: LazyLock<RwLock<bool>> = LazyLock::new(|| RwLock::new(false));

fn ensure_aux_pg_tables() {
    if *PG_AUX_TABLES_READY.read().unwrap() {
        return;
    }
    let mut ready = PG_AUX_TABLES_READY.write().unwrap();
    if *ready {
        return;
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.batch_execute(
            "SET client_min_messages = WARNING;
            CREATE TABLE IF NOT EXISTS entity_body_injuries (
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
            );
            CREATE TABLE IF NOT EXISTS character_vessel_mastery (
                entity_id TEXT NOT NULL,
                item_id TEXT NOT NULL,
                use_count INTEGER NOT NULL DEFAULT 0,
                wear_points INTEGER NOT NULL DEFAULT 0,
                condition_code TEXT NOT NULL DEFAULT 'sharp',
                updated_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, item_id)
            );
            CREATE TABLE IF NOT EXISTS character_lineage_imprints (
                entity_id TEXT NOT NULL,
                mentor_name TEXT NOT NULL,
                imprint_title TEXT NOT NULL,
                imprint_prose TEXT NOT NULL,
                interaction_count INTEGER NOT NULL DEFAULT 1,
                manual_gifted BOOLEAN NOT NULL DEFAULT FALSE,
                updated_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, mentor_name)
            );
            CREATE TABLE IF NOT EXISTS character_breath_habit (
                entity_id TEXT PRIMARY KEY,
                dominant_code TEXT NOT NULL DEFAULT 'forest_long',
                forge_score INTEGER NOT NULL DEFAULT 0,
                forest_score INTEGER NOT NULL DEFAULT 0,
                tide_score INTEGER NOT NULL DEFAULT 0,
                chamber_score INTEGER NOT NULL DEFAULT 0,
                battle_score INTEGER NOT NULL DEFAULT 0,
                updated_at BIGINT NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS entity_studied_manuals (
                entity_id TEXT NOT NULL,
                manual_title TEXT NOT NULL,
                studied_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, manual_title)
            );
            CREATE TABLE IF NOT EXISTS entity_anatomy_familiarity (
                entity_id TEXT NOT NULL,
                target_category TEXT NOT NULL,
                familiarity_count INTEGER NOT NULL DEFAULT 0,
                updated_at BIGINT NOT NULL DEFAULT 0,
                PRIMARY KEY (entity_id, target_category)
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
        if let Ok(rows) = client.query(
            "SELECT entity_id, manual_title FROM entity_studied_manuals ORDER BY studied_at ASC",
            &[],
        ) {
            let mut guard = STUDIED_MANUALS_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let title: String = r.get(1);
                let list = guard.entry(eid).or_default();
                if !list.contains(&title) {
                    list.push(title);
                }
            }
        }
        if let Ok(rows) = client.query(
            "SELECT entity_id, target_category, familiarity_count FROM entity_anatomy_familiarity",
            &[],
        ) {
            let mut guard = ANATOMY_FAMILIARITY_CACHE.write().unwrap();
            for r in rows {
                let eid: String = r.get(0);
                let cat: String = r.get(1);
                let count: i32 = r.get(2);
                guard.entry(eid).or_default().insert(cat, count.max(0) as u32);
            }
        }
        *ready = true;
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
            Self::ChamberClear => "常年靜心著譜研讀、品茗調息與體察氣脈所養成的吐納習慣。吐納勻淨澄明，體內氣機遇阻時能自然尋隙繞行，行氣運勁更為省力。",
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
    pub fn effect_summary(&self) -> &'static str {
        match self {
            Self::ForgeHeavy => "實戰剛猛重擊，催發沉凝力道貫通關節以破剛硬守備；鍛打冶煉時能細察火候變化，器物精工成形更為純粹。",
            Self::ForestLong => "實戰輕靈游鬥，身法如林間游鹿靈動難測；山海崎嶇跋涉時能勻長換氣舒緩筋骨，百工淬煉亦顯得心應手。",
            Self::TideContinuous => "實戰綿延續航，內息如疊浪後勁綿綿；濕地水域跋涉時換氣自然順暢，庖廚熬煉與百工調配格外醇熟。",
            Self::ChamberClear => "實戰澄明貫脈，心神守一方寸自能透徹尋隙；細木卯合、煎藥研粉之際神意專注，精工品相極易生出巧思。",
            Self::BattleFierce => "實戰烘爐搏殺，胸中烈息激盪敢於近身硬撼重圍；兵刃搏殺與實戰磨合迅速入化，鍛淬器物自具凜冽鋒芒。",
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
    let effective_seed = if soul_seed != 0 {
        soul_seed.unsigned_abs()
    } else {
        entity_id.bytes().fold(1u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
    };

    let mut forge_score = 4;
    let mut forest_score = 4;
    let mut tide_score = 4;
    let mut chamber_score = 4;
    let mut battle_score = 4;

    // Only apply random seed drift when vocation_hint does not specify a clear trade

    let is_guard_or_martial = vocation_hint.contains("衛")
        || vocation_hint.contains("武師")
        || vocation_hint.contains("教頭")
        || vocation_hint.contains("教練")
        || vocation_hint.contains("總教")
        || vocation_hint.contains("巡防")
        || vocation_hint.contains("哨")
        || vocation_hint.contains("百戰")
        || vocation_hint.contains("apex");

    let is_hunter_or_forager = vocation_hint.contains("獵")
        || vocation_hint.contains("採")
        || vocation_hint.contains("樵")
        || vocation_hint.contains("入山百工")
        || vocation_hint.contains("林道")
        || vocation_hint.contains("巡山");

    let is_dock_or_water = vocation_hint.contains("漁")
        || vocation_hint.contains("海")
        || vocation_hint.contains("港")
        || vocation_hint.contains("渡")
        || vocation_hint.contains("船")
        || vocation_hint.contains("舵")
        || vocation_hint.contains("網")
        || vocation_hint.contains("水")
        || vocation_hint.contains("潮")
        || vocation_hint.contains("泉")
        || vocation_hint.contains("堰")
        || vocation_hint.contains("渠");

    let is_artisan_forge = vocation_hint.contains("鍛")
        || vocation_hint.contains("鐵")
        || vocation_hint.contains("礦")
        || vocation_hint.contains("石")
        || vocation_hint.contains("鎬")
        || vocation_hint.contains("金")
        || vocation_hint.contains("木")
        || vocation_hint.contains("匠")
        || vocation_hint.contains("窯")
        || vocation_hint.contains("磚")
        || vocation_hint.contains("瓦")
        || vocation_hint.contains("泥")
        || vocation_hint.contains("刨")
        || vocation_hint.contains("修");

    let is_clerk_scholar_household = vocation_hint.contains("家眷")
        || vocation_hint.contains("內當家")
        || vocation_hint.contains("持家")
        || vocation_hint.contains("長輩")
        || vocation_hint.contains("晚輩")
        || vocation_hint.contains("掌事")
        || vocation_hint.contains("執事")
        || vocation_hint.contains("學徒")
        || vocation_hint.contains("東家")
        || vocation_hint.contains("帳房")
        || vocation_hint.contains("商行")
        || vocation_hint.contains("市集")
        || vocation_hint.contains("公所")
        || vocation_hint.contains("書")
        || vocation_hint.contains("學")
        || vocation_hint.contains("講")
        || vocation_hint.contains("印")
        || vocation_hint.contains("記")
        || vocation_hint.contains("醫")
        || vocation_hint.contains("藥")
        || vocation_hint.contains("診")
        || vocation_hint.contains("算")
        || vocation_hint.contains("食堂")
        || vocation_hint.contains("廚")
        || vocation_hint.contains("茶");

    if is_guard_or_martial {
        battle_score += 6;
    } else if is_hunter_or_forager {
        forest_score += 6;
    } else if is_artisan_forge {
        forge_score += 6;
    } else if is_dock_or_water {
        tide_score += 6;
    } else if is_clerk_scholar_household {
        chamber_score += 6;
    } else if soul_seed != 0 {
        match effective_seed % 5 {
            0 => forge_score += 3,
            1 => forest_score += 3,
            2 => tide_score += 3,
            3 => chamber_score += 3,
            _ => battle_score += 3,
        }
        match (effective_seed / 5) % 5 {
            0 => forge_score += 2,
            1 => forest_score += 2,
            2 => tide_score += 2,
            3 => chamber_score += 2,
            _ => battle_score += 2,
        }
    }

    let tiebreaker = |kind: BreathHabitKind| -> u64 {
        let idx = match kind {
            BreathHabitKind::ForgeHeavy => 0,
            BreathHabitKind::ForestLong => 1,
            BreathHabitKind::TideContinuous => 2,
            BreathHabitKind::ChamberClear => 3,
            BreathHabitKind::BattleFierce => 4,
        };
        effective_seed
            .wrapping_mul(0x9E3779B97F4A7C15)
            .wrapping_add((idx as u64).wrapping_mul(0xBF58476D1CE4E5B9))
    };

    let candidates = [
        (BreathHabitKind::ForgeHeavy, forge_score),
        (BreathHabitKind::ForestLong, forest_score),
        (BreathHabitKind::TideContinuous, tide_score),
        (BreathHabitKind::ChamberClear, chamber_score),
        (BreathHabitKind::BattleFierce, battle_score),
    ];
    let dominant = candidates
        .into_iter()
        .max_by(|(k_a, s_a), (k_b, s_b)| match s_a.cmp(s_b) {
            std::cmp::Ordering::Equal => tiebreaker(*k_a).cmp(&tiebreaker(*k_b)),
            ord => ord,
        })
        .map(|(k, _)| k)
        .unwrap_or(BreathHabitKind::ForestLong);

    let rec = BreathHabitRecord {
        entity_id: entity_id.to_string(),
        dominant,
        forge_score,
        forest_score,
        tide_score,
        chamber_score,
        battle_score,
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
            Self::Sharp => "鋒利",
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

pub fn get_entity_max_vessel_use_count(entity_id: &str) -> u32 {
    ensure_aux_pg_tables();
    if let Ok(guard) = VESSEL_STATE_CACHE.read() {
        let mut max_count = 0i32;
        for ((eid, _), rec) in guard.iter() {
            if eid == entity_id && rec.use_count > max_count {
                max_count = rec.use_count;
            }
        }
        if max_count > 0 {
            return max_count as u32;
        }
    }
    0
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

pub fn record_entity_studied_manual(entity_id: &str, manual_title: &str) {
    ensure_aux_pg_tables();
    let already_has = {
        let mut guard = STUDIED_MANUALS_CACHE.write().unwrap();
        let list = guard.entry(entity_id.to_string()).or_default();
        if list.iter().any(|m| m == manual_title) {
            true
        } else {
            list.push(manual_title.to_string());
            false
        }
    };
    if !already_has {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute(
                "INSERT INTO entity_studied_manuals (entity_id, manual_title, studied_at)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (entity_id, manual_title) DO NOTHING",
                &[&entity_id, &manual_title, &now_unix],
            );
        }
    }
}

pub fn get_entity_studied_manuals(entity_id: &str) -> Vec<String> {
    ensure_aux_pg_tables();
    STUDIED_MANUALS_CACHE
        .read()
        .unwrap()
        .get(entity_id)
        .cloned()
        .unwrap_or_default()
}

pub fn record_entity_anatomy_familiarity(entity_id: &str, target_category: &str, delta: u32) -> u32 {
    ensure_aux_pg_tables();
    let new_count = {
        let mut guard = ANATOMY_FAMILIARITY_CACHE.write().unwrap();
        let map = guard.entry(entity_id.to_string()).or_default();
        let entry = map.entry(target_category.to_string()).or_insert(0);
        *entry = entry.saturating_add(delta.max(1));
        *entry
    };
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let count_i32 = new_count as i32;
        let _ = client.execute(
            "INSERT INTO entity_anatomy_familiarity (entity_id, target_category, familiarity_count, updated_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (entity_id, target_category) DO UPDATE SET
               familiarity_count = EXCLUDED.familiarity_count,
               updated_at = EXCLUDED.updated_at",
            &[&entity_id, &target_category, &count_i32, &now_unix],
        );
    }
    new_count
}

pub fn get_entity_anatomy_familiarity_total(entity_id: &str) -> u32 {
    ensure_aux_pg_tables();
    ANATOMY_FAMILIARITY_CACHE
        .read()
        .unwrap()
        .get(entity_id)
        .map(|map| map.values().copied().sum())
        .unwrap_or(0)
}

pub fn get_entity_anatomy_familiarity_summary(entity_id: &str) -> String {
    ensure_aux_pg_tables();
    let guard = ANATOMY_FAMILIARITY_CACHE.read().unwrap();
    let Some(map) = guard.get(entity_id) else {
        return "尚未累積深厚的山海生靈剖析或百工取材心得，仍以常理摸索。".to_string();
    };
    if map.is_empty() {
        return "尚未累積深厚的山海生靈剖析或百工取材心得，仍以常理摸索。".to_string();
    }
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by(|a, b| b.1.cmp(a.1));
    let items_str = entries
        .iter()
        .take(4)
        .map(|(cat, count)| {
            let depth = if **count >= 20 {
                "入微純熟"
            } else if **count >= 8 {
                "漸諳脈絡"
            } else {
                "初窺肌理"
            };
            format!("{}・{}", cat, depth)
        })
        .collect::<Vec<_>>()
        .join("、");
    format!(
        "常年接觸山海生靈與百工物材（如{}），熟稔筋骨脈絡與紋理走向，落刃順理而不傷器物鋒芒。",
        items_str
    )
}

/// 一次性建世種子：為全島工坊與住戶匠人初始化器物磨損與吐納基底（所有人皆為自己的主角）
pub fn seed_flagship_specimen_qin_shuisheng_once(now_unix: i64) {
    static ALL_RESIDENTS_SEEDED: std::sync::Once = std::sync::Once::new();
    static SEEDED_ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    ensure_aux_pg_tables();
    let npc_id = "秦水生";

    record_entity_studied_manual(npc_id, "堰渠水輪分動考工譜");
    record_entity_studied_manual(npc_id, "石磨齒距微調與水磨澄漿手札");
    {
        let mut guard = ANATOMY_FAMILIARITY_CACHE.write().unwrap();
        let map = guard.entry(npc_id.to_string()).or_default();
        map.entry("石磨鑿紋與水輪木齒".to_string()).or_insert(28);
        map.entry("在來米與北山藥材水磨澄漿".to_string()).or_insert(36);
    }
    {
        let mut guard = BREATH_HABIT_CACHE.write().unwrap();
        guard.entry(npc_id.to_string()).or_insert_with(|| BreathHabitRecord {
            entity_id: npc_id.to_string(),
            dominant: BreathHabitKind::TideContinuous,
            forge_score: 14,
            forest_score: 6,
            tide_score: 48,
            chamber_score: 8,
            battle_score: 2,
            updated_at: now_unix,
        });
    }
    {
        let mut guard = LINEAGE_IMPRINT_CACHE.write().unwrap();
        let list = guard.entry(npc_id.to_string()).or_default();
        if !list.iter().any(|imp| imp.imprint_title == "老堰頭水輪分動與雙疊石磨聽音辨距法") {
            list.push(LineageImprintRecord {
                entity_id: npc_id.to_string(),
                mentor_name: "秦老堰主".to_string(),
                imprint_title: "老堰頭水輪分動與雙疊石磨聽音辨距法".to_string(),
                imprint_prose: "順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性".to_string(),
                interaction_count: 12,
                manual_gifted: true,
                updated_at: now_unix,
            });
        }
    }
    {
        let mut guard = VESSEL_STATE_CACHE.write().unwrap();
        let sieve_entry = guard
            .entry((npc_id.to_string(), "wt_cat_0973".to_string()))
            .or_insert_with(|| VesselStateRecord {
                entity_id: npc_id.to_string(),
                item_id: "wt_cat_0973".to_string(),
                use_count: 68,
                wear_points: 0,
                condition: VesselCondition::Sharp,
                updated_at: now_unix,
            });
        if sieve_entry.wear_points > 0 || sieve_entry.condition != VesselCondition::Sharp {
            sieve_entry.wear_points = 0;
            sieve_entry.condition = VesselCondition::Sharp;
            sieve_entry.updated_at = now_unix;
        }
        guard
            .entry((npc_id.to_string(), "wt_cat_0984".to_string()))
            .or_insert_with(|| VesselStateRecord {
                entity_id: npc_id.to_string(),
                item_id: "wt_cat_0984".to_string(),
                use_count: 42,
                wear_points: 0,
                condition: VesselCondition::Sharp,
                updated_at: now_unix,
            });
    }

    ALL_RESIDENTS_SEEDED.call_once(|| {
        for slot in crate::store::WENYAO_HOUSEHOLD_SLOTS.iter().chain(crate::store::OUTER_HOUSEHOLD_SLOTS.iter()) {
            let name = slot.name();
            let (dominant_breath, forge_s, forest_s, tide_s, chamber_s, battle_s, manual_title, fam_category, imprint_title, imprint_prose) =
                if slot.brief_tag.contains("鍛")
                    || slot.brief_tag.contains("鐵")
                    || slot.brief_tag.contains("礦")
                    || slot.brief_tag.contains("窯")
                    || slot.work_name.contains("鍛")
                    || slot.work_name.contains("鐵")
                    || slot.work_name.contains("礦")
                    || slot.work_name.contains("窯")
                {
                    (
                        BreathHabitKind::ForgeHeavy,
                        42,
                        8,
                        8,
                        8,
                        6,
                        "百鍛火候與錘路考工譜",
                        "赤鐵礦脈與百鍛鋼胚紋理",
                        "鐵坊鼓風落錘真傳",
                        "順風箱開闔沉肩落錘，聽音辨溫而百鍛成器",
                    )
                } else if slot.brief_tag.contains("水")
                    || slot.brief_tag.contains("磨")
                    || slot.brief_tag.contains("漁")
                    || slot.brief_tag.contains("船")
                    || slot.brief_tag.contains("海")
                    || slot.brief_tag.contains("鹽")
                    || slot.brief_tag.contains("渡")
                    || slot.work_name.contains("水")
                    || slot.work_name.contains("磨")
                    || slot.work_name.contains("漁")
                    || slot.work_name.contains("船")
                    || slot.work_name.contains("海")
                    || slot.work_name.contains("鹽")
                    || slot.work_name.contains("渡")
                {
                    (
                        BreathHabitKind::TideContinuous,
                        10,
                        8,
                        42,
                        8,
                        4,
                        "堰渠水輪與潮汐舟楫手札",
                        "水脈流速與舟車石磨齒距",
                        "水岸聽潮辨流心法",
                        "順潮汐水脈綿長調息，觀波紋緩急以拿捏分寸",
                    )
                } else if slot.brief_tag.contains("獵")
                    || slot.brief_tag.contains("林")
                    || slot.brief_tag.contains("木")
                    || slot.brief_tag.contains("藥")
                    || slot.brief_tag.contains("農")
                    || slot.brief_tag.contains("牧")
                    || slot.brief_tag.contains("茶")
                    || slot.work_name.contains("獵")
                    || slot.work_name.contains("林")
                    || slot.work_name.contains("木")
                    || slot.work_name.contains("藥")
                    || slot.work_name.contains("農")
                    || slot.work_name.contains("牧")
                    || slot.work_name.contains("茶")
                {
                    (
                        BreathHabitKind::ForestLong,
                        8,
                        42,
                        10,
                        8,
                        6,
                        "北山草木物候與百工採造錄",
                        "山林良木與百草筋脈肌理",
                        "山林順紋採造心傳",
                        "循四時物候與草木自然紋理，落手沉穩而不傷生機",
                    )
                } else {
                    (
                        BreathHabitKind::ChamberClear,
                        8,
                        8,
                        10,
                        42,
                        4,
                        "半島百業籌算與街坊通考",
                        "市井器物成色與百業規程",
                        "街坊掌事安頓心法",
                        "心平氣和核對往來器物與帳目，井然有序而不失分寸",
                    )
                };

            {
                let mut guard = STUDIED_MANUALS_CACHE.write().unwrap();
                let list = guard.entry(name.to_string()).or_default();
                if !list.iter().any(|m| m == manual_title) {
                    list.push(manual_title.to_string());
                }
            }

            {
                let mut guard = ANATOMY_FAMILIARITY_CACHE.write().unwrap();
                let map = guard.entry(name.to_string()).or_default();
                map.entry(fam_category.to_string()).or_insert(24);
            }

            {
                let mut guard = BREATH_HABIT_CACHE.write().unwrap();
                guard.entry(name.to_string()).or_insert_with(|| BreathHabitRecord {
                    entity_id: name.to_string(),
                    dominant: dominant_breath,
                    forge_score: forge_s,
                    forest_score: forest_s,
                    tide_score: tide_s,
                    chamber_score: chamber_s,
                    battle_score: battle_s,
                    updated_at: now_unix,
                });
            }

            {
                let mut guard = LINEAGE_IMPRINT_CACHE.write().unwrap();
                let list = guard.entry(name.to_string()).or_default();
                if list.is_empty() {
                    list.push(LineageImprintRecord {
                        entity_id: name.to_string(),
                        mentor_name: "行家老匠".to_string(),
                        imprint_title: imprint_title.to_string(),
                        imprint_prose: imprint_prose.to_string(),
                        interaction_count: 6,
                        manual_gifted: false,
                        updated_at: now_unix,
                    });
                }
            }

            let vessel = slot.held_vessel();
            if !vessel.is_empty() {
                let mut guard = VESSEL_STATE_CACHE.write().unwrap();
                guard
                    .entry((name.to_string(), vessel.to_string()))
                    .or_insert_with(|| VesselStateRecord {
                        entity_id: name.to_string(),
                        item_id: vessel.to_string(),
                        use_count: 24,
                        wear_points: 0,
                        condition: VesselCondition::Sharp,
                        updated_at: now_unix,
                    });
            }
        }
    });

    if SEEDED_ONCE.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let _ = client.execute(
            "INSERT INTO venues (id, name, room_id, max_staff)
             VALUES ('wt-venue-laoyantou-watermill', '老堰頭・水輪石磨工坊', 'grid:-3:125', 6)
             ON CONFLICT (id) DO NOTHING",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO occupations (id, name, room_type)
             VALUES ('水輪石磨首座', '水輪石磨首座', 'workshop')
             ON CONFLICT (id) DO NOTHING",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO assignments (entity_id, occupation_id, venue_id, assigned_by)
             VALUES ('秦水生', '水輪石磨首座', 'wt-venue-laoyantou-watermill', 'canonical_lineage')
             ON CONFLICT (entity_id, venue_id) DO NOTHING",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO schedules (entity_id, work_room, rest_room, shift_start, shift_end)
             VALUES ('秦水生', 'grid:-3:125', 'grid:-3:124', 7, 18)
             ON CONFLICT (entity_id) DO NOTHING",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO entity_vessel_states (entity_id, item_id, use_count, wear_points, condition_code, updated_at)
             VALUES
               ('秦水生', 'wt_cat_0973', 68, 0, 'sharp', $1),
               ('秦水生', 'wt_cat_0984', 42, 0, 'sharp', $1),
               ('岑守岳', 'wt_blacksmith_hammer', 56, 0, 'sharp', $1),
               ('柯成器', 'wt_carpenter_chisel', 48, 0, 'sharp', $1)
             ON CONFLICT (entity_id, item_id) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "INSERT INTO character_vessel_mastery (entity_id, item_id, use_count, wear_points, condition_code, updated_at)
             VALUES
               ('秦水生', 'wt_cat_0973', 68, 0, 'sharp', $1),
               ('秦水生', 'wt_cat_0984', 42, 0, 'sharp', $1),
               ('岑守岳', 'wt_blacksmith_hammer', 56, 0, 'sharp', $1),
               ('柯成器', 'wt_carpenter_chisel', 48, 0, 'sharp', $1)
             ON CONFLICT (entity_id, item_id) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "INSERT INTO entity_breath_habits (entity_id, dominant_code, forge_score, forest_score, tide_score, chamber_score, battle_score, updated_at)
             VALUES ('秦水生', 'tide_continuous', 14, 6, 48, 8, 2, $1)
             ON CONFLICT (entity_id) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "INSERT INTO entity_lineage_imprints (entity_id, mentor_name, imprint_title, imprint_prose, interaction_count, manual_gifted, updated_at)
             VALUES
               ('秦水生', '秦老堰主', '老堰頭水輪分動與雙疊石磨聽音辨距法', '順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性', 12, true, $1),
               ('岑守岳', '岑鐵匠', '岑氏百鍛落錘真傳', '百鍛淬鋒、分毫不差', 15, true, $1),
               ('柯成器', '柯木匠', '柯氏北山杉木榫卯真傳', '循木觀理、卯榫自合', 14, true, $1)
             ON CONFLICT (entity_id, mentor_name) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "INSERT INTO character_lineage_imprints (entity_id, mentor_name, imprint_title, imprint_prose, interaction_count, manual_gifted, updated_at)
             VALUES
               ('秦水生', '秦老堰主', '老堰頭水輪分動與雙疊石磨聽音辨距法', '順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性', 12, true, $1),
               ('岑守岳', '岑鐵匠', '岑氏百鍛落錘真傳', '百鍛淬鋒、分毫不差', 15, true, $1),
               ('柯成器', '柯木匠', '柯氏北山杉木榫卯真傳', '循木觀理、卯榫自合', 14, true, $1)
             ON CONFLICT (entity_id, mentor_name) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "UPDATE entity_lineage_imprints
             SET imprint_prose = '順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性'
             WHERE entity_id = '秦水生' AND imprint_title = '老堰頭水輪分動與雙疊石磨聽音辨距法'",
            &[],
        );
        let _ = client.execute(
            "UPDATE character_lineage_imprints
             SET imprint_prose = '順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性'
             WHERE entity_id = '秦水生' AND imprint_title = '老堰頭水輪分動與雙疊石磨聽音辨距法'",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO entity_anatomy_familiarity (entity_id, target_category, familiarity_count, updated_at)
             VALUES
               ('秦水生', '石磨鑿紋與水輪木齒', 28, $1),
               ('秦水生', '在來米與北山藥材水磨澄漿', 36, $1)
             ON CONFLICT (entity_id, target_category) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "DELETE FROM resident_knowledge WHERE entity_id = '秦水生' AND subject_ref LIKE 'npc_%'",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO resident_knowledge (entity_id, subject_ref, predicate, value, certainty, source_ref, learned_at, updated_at)
             VALUES
               ('秦水生', '老堰頭・水輪石磨工坊', 'millstone_and_sluice_state', '重礦包邊上下磨盤齒距已校準至三毫，清泉水脈二號引水木閘流速平穩，日可精碾晚造在來米十二石', 98, 'flagship_specimen_1', $1, $1),
               ('秦水生', '大圳頭・周萬生', 'upstream_grain_supply', '大圳頭周萬生本旬已備妥十袋晚造在來新穀與赤石坡抗旱麥，待明日辰時沿北郊水渠運抵老堰頭入磨', 95, 'flagship_specimen_1', $1, $1),
               ('秦水生', '文曜・陳萬福', 'morning_ricemilk_contract', '文曜三水大食堂每日寅末卯初需現磨溫米漿六桶與特級在來米粉八袋，須以薄層重礦烏木膽溫存桶封裝趁熱送達', 96, 'flagship_specimen_1', $1, $1),
               ('秦水生', '霜林口・梁千木', 'waterwheel_gear_maintenance', '水輪分動大軸的水青岡木齒輪每逢春霖漲水易受泥沙磨損，已約定霜林口梁千木備妥替換木齒與硬木楔子', 92, 'flagship_specimen_1', $1, $1),
               ('秦水生', '老堰頭・清泉水脈', 'hydrology_and_weather_omen', '每逢白塔北坡夜雨，老堰頭清泉水脈於寅時水位必漲三寸，須提前半刻落下一號分洪插板以免石磨轉速過急傷及米粉細度', 94, 'flagship_specimen_1', $1, $1)
             ON CONFLICT (entity_id, subject_ref, predicate) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "DELETE FROM resident_relationships WHERE entity_id = '秦水生' AND other_entity_id LIKE 'npc_%'",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO resident_relationships (entity_id, other_entity_id, relation_kind, evidence_count, source_ref, last_interaction_at, updated_at)
             VALUES
               ('秦水生', '周萬生', 'upstream_grain_and_canal_partner', 18, 'flagship_specimen_1', $1, $1),
               ('秦水生', '陳萬福', 'downstream_ricemilk_buyer', 24, 'flagship_specimen_1', $1, $1),
               ('秦水生', '梁千木', 'waterwheel_timber_craftsman', 11, 'flagship_specimen_1', $1, $1),
               ('秦水生', '蘇百草', 'north_mountain_herb_milling_partner', 14, 'flagship_specimen_1', $1, $1)
             ON CONFLICT (entity_id, other_entity_id, relation_kind) DO NOTHING",
            &[&now_unix],
        );
        let _ = client.execute(
            "INSERT INTO npc_memories (entity_id, subject_id, meet_count, favorability)
             VALUES
               ('秦水生', '周萬生', 18, 78),
               ('秦水生', '陳萬福', 24, 82),
               ('秦水生', '梁千木', 11, 74),
               ('秦水生', '蘇百草', 14, 76)
             ON CONFLICT (entity_id, subject_id) DO NOTHING",
            &[],
        );
        let _ = client.execute(
            "INSERT INTO npc_npc_summaries (dyad_key, summary)
             VALUES
               ('周萬生|秦水生', '秦水生與大圳頭水車農夫周萬生常年合議北郊水脈分水時刻，周萬生種出的晚造在來新穀皆交由秦水生水磨成粉與溫米漿。'),
               ('秦水生|陳萬福', '秦水生每日清晨將老堰頭水輪石磨慢熬的現磨溫米漿與特級在來米粉裝入重礦溫存桶，直供文曜三水大食堂陳萬福備早膳。'),
               ('梁千木|秦水生', '老堰頭水輪分動大軸與水青岡木齒輪皆由霜林口梁千木選材車製，秦水生則以現磨純糯米粉與溫米漿回禮。'),
               ('秦水生|蘇百草', '北山採回的乾燥藥材需借老堰頭細紋石磨慢碾成百草藥末而不傷藥性，秦水生常為蘇百草留出一座專磨藥材的副磨。')
             ON CONFLICT (dyad_key) DO NOTHING",
            &[],
        );
        let due_ts = now_unix + 14_400;
        let _ = client.execute(
            "INSERT INTO resident_obligations (obligation_id, entity_id, obligation_kind, subject_ref, status, due_at, source_ref, created_at, updated_at)
             VALUES
               ('ob_qin_shuisheng_morning_ricemilk_wenyao', '秦水生', 'restock_goods', '陳萬福・現磨溫米漿與特級在來米粉早市交割', 'open', $2, 'flagship_specimen_1', $1, $1),
               ('ob_qin_shuisheng_sluice_and_gear_check', '秦水生', 'repair_tool', '周萬生・老堰頭二號引水木閘與石磨齒距校準', 'open', $2, 'flagship_specimen_1', $1, $1),
               ('ob_cen_shouyue_morning_hardware_delivery', '岑守岳', 'restock_goods', '陳萬福・百鍛農具鐵件早市交割', 'open', $2, 'flagship_specimen_blacksmith', $1, $1),
               ('ob_ke_chengqi_morning_handle_delivery', '柯成器', 'restock_goods', '陳萬福・堅韌硬木工具長柄早市交割', 'open', $2, 'flagship_specimen_carpenter', $1, $1)
             ON CONFLICT (obligation_id) DO NOTHING",
            &[&now_unix, &due_ts],
        );
        let _ = client.execute(
            "UPDATE character_vessel_mastery SET wear_points = 0, condition_code = 'sharp'
             WHERE entity_id = '秦水生' AND item_id = 'wt_cat_0973' AND wear_points > 0",
            &[],
        );
        if let Ok(mut guard) = LINEAGE_IMPRINT_CACHE.write() {
            guard.entry("岑守岳".to_string()).or_default().push(LineageImprintRecord {
                entity_id: "岑守岳".to_string(),
                mentor_name: "岑鐵匠".to_string(),
                imprint_title: "岑氏百鍛落錘真傳".to_string(),
                imprint_prose: "百鍛淬鋒、分毫不差".to_string(),
                interaction_count: 15,
                manual_gifted: true,
                updated_at: now_unix,
            });
            guard.entry("柯成器".to_string()).or_default().push(LineageImprintRecord {
                entity_id: "柯成器".to_string(),
                mentor_name: "柯木匠".to_string(),
                imprint_title: "柯氏北山杉木榫卯真傳".to_string(),
                imprint_prose: "循木觀理、卯榫自合".to_string(),
                interaction_count: 14,
                manual_gifted: true,
                updated_at: now_unix,
            });
        }
        if let Ok(mut guard) = VESSEL_STATE_CACHE.write() {
            guard.insert(
                ("岑守岳".to_string(), "wt_blacksmith_hammer".to_string()),
                VesselStateRecord {
                    entity_id: "岑守岳".to_string(),
                    item_id: "wt_blacksmith_hammer".to_string(),
                    use_count: 56,
                    wear_points: 0,
                    condition: VesselCondition::Sharp,
                    updated_at: now_unix,
                },
            );
            guard.insert(
                ("柯成器".to_string(), "wt_carpenter_chisel".to_string()),
                VesselStateRecord {
                    entity_id: "柯成器".to_string(),
                    item_id: "wt_carpenter_chisel".to_string(),
                    use_count: 48,
                    wear_points: 0,
                    condition: VesselCondition::Sharp,
                    updated_at: now_unix,
                },
            );
        }
    }
}

pub(crate) fn inv_count_item(inv_json: &str, target_id: &str) -> i32 {
    serde_json::from_str::<Vec<serde_json::Value>>(inv_json)
        .ok()
        .into_iter()
        .flatten()
        .filter(|v| v.get("item_id").and_then(|s| s.as_str()) == Some(target_id))
        .map(|v| v.get("qty").and_then(|q| q.as_i64()).unwrap_or(0) as i32)
        .sum()
}

fn inv_mutate_item(inv_json: &mut String, target_id: &str, delta: i32, max_cap: i32) -> i32 {
    let mut items = serde_json::from_str::<Vec<serde_json::Value>>(inv_json).unwrap_or_default();
    let mut new_qty = 0i32;
    let mut found = false;
    for entry in items.iter_mut() {
        if entry.get("item_id").and_then(|s| s.as_str()) == Some(target_id) {
            let cur = entry.get("qty").and_then(|q| q.as_i64()).unwrap_or(0) as i32;
            let next = (cur + delta).clamp(0, max_cap);
            entry["qty"] = serde_json::json!(next);
            new_qty = next;
            found = true;
            break;
        }
    }
    if !found && delta > 0 {
        new_qty = delta.clamp(0, max_cap);
        items.push(serde_json::json!({ "item_id": target_id, "qty": new_qty }));
    }
    items.retain(|v| v.get("qty").and_then(|q| q.as_i64()).unwrap_or(0) > 0);
    if let Ok(s) = serde_json::to_string(&items) {
        *inv_json = s;
    }
    new_qty
}

/// 玩家同構「四問決策閉環」（`Observe -> Can Continue? -> Choose -> Mutate World`）：
/// 驅動 `秦水生` 真實消耗白米與體力、累積並推磨手鏨磨損、跨區向陳萬福交割米漿結算鎂幣、斷料向周萬生提穀，並寫回資料庫。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentLifeActionChoice {
    RepairTool,
    DeliverContract,
    RestockGrain,
    FloodgateCheck,
    DineParley,
    NightRest,
    MillGrain,
}

/// 由角色「先天心性單純形 × CAPS 處境觸發相 × 後天所學・所熟・所歷」（`PluralPersonalityWeights`）
/// 對當下處境候選行動進行效用權衡，取代死板之單一 `if / else` 機器人梯子。
/// 相同處境下（例如手鏨微鈍 `wear_points == 3` 恰逢早市交貨，或體力微乏 `vit == 42`），
/// 不同 `soul_seed` 與心相權重的人物會做出截然不同但符合其為人的選擇。
pub fn choose_resident_life_action(
    role_title: &str,
    settlement: &str,
    soul_seed: u64,
    vit: i32,
    magnesium: i32,
    grain_qty: i32,
    ricemilk_qty: i32,
    wear_points: i32,
    ricemilk_ob_open: bool,
    game_hour: i32,
    weather: crate::game::ZoneWeatherPhase,
    tide: crate::game::ZoneTidePhase,
    repetitions: u32,
    hooks: &[crate::npc::causal_core::RelationalHook],
) -> ResidentLifeActionChoice {
    let stress = ((100 - vit.clamp(0, 100)) as u8).saturating_div(2);
    let pw = crate::npc::causal_core::compute_plural_personality_weights_with_experience(
        role_title,
        settlement,
        soul_seed,
        stress,
        magnesium,
        repetitions,
        hooks,
        None,
    );

    let net_trust: i32 = hooks
        .iter()
        .map(|h| {
            let is_grudge = ["遭拒", "追討", "結怨", "嫌隙", "展期"]
                .iter()
                .any(|k| h.kind_label.contains(k) || h.summary.contains(k));
            let is_favor = ["恩", "守諾", "解囊", "馳援", "完修", "交割", "供料"]
                .iter()
                .any(|k| h.kind_label.contains(k) || h.summary.contains(k));
            if is_grudge {
                -(h.intensity as i32)
            } else if is_favor {
                h.intensity as i32
            } else {
                0
            }
        })
        .sum::<i32>()
        .clamp(-60, 80);

    // 1. 修磨器物 vs 強行趕工／先送早市：
    //    - wear_points == 2 時，極重匠心與守序的完美主義者已會考慮提前修磨；
    //    - wear_points == 3 時，匠心／守序型選擇先修磨（哪怕延誤早市交貨），
    //      而重義／嗜利型在有早市契約時會選擇「先履約送貨、回來再磨刀」，
    //      極端嗜利／傲骨趕工型甚至會在無契約時硬撐鈍器再趕一輪工（直至 wear_points >= 4）。
    let score_repair = if wear_points >= 4 {
        1250 + pw.craft * 5 + pw.order * 3
    } else if wear_points == 3 {
        680 + pw.craft * 5 + pw.order * 3 - pw.profit
    } else if wear_points == 2 && pw.craft >= 148 && pw.order >= 130 && grain_qty >= 3 {
        560 + pw.craft * 3 + pw.order * 2
    } else {
        0
    };

    let score_deliver = if ricemilk_ob_open && ricemilk_qty >= 2 && (5..=8).contains(&game_hour) {
        580 + pw.honor * 4 + pw.profit * 3 + net_trust * 6
    } else {
        0
    };

    let score_restock = if grain_qty < 2 && (6..=18).contains(&game_hour) {
        740 + (2 - grain_qty.min(2)) * 180 + pw.order * 2 + pw.profit * 2 + pw.craft
    } else {
        0
    };

    let score_floodgate = if weather == crate::game::ZoneWeatherPhase::RainMist
        && tide == crate::game::ZoneTidePhase::FloodTide
        && (7..=17).contains(&game_hour)
    {
        720 + pw.care * 3 + pw.order * 2 + pw.honor
    } else {
        0
    };

    let score_dine = if (18..=20).contains(&game_hour) || vit < 32 {
        680 + (50 - vit.min(50)) * 8 + pw.care * 2
    } else if vit < 48 && (7..=17).contains(&game_hour) {
        430 + (48 - vit) * 14 + pw.care * 3 - pw.craft - pw.profit
    } else {
        0
    };

    let score_rest = if !(7..=17).contains(&game_hour) {
        600 + pw.order * 2 + pw.care * 2
    } else {
        0
    };

    let score_mill = if (7..=17).contains(&game_hour) && grain_qty >= 1 {
        490 + pw.craft * 2 + pw.profit * 2 + pw.pride + pw.order
    } else {
        100
    };

    let candidates = [
        (ResidentLifeActionChoice::RepairTool, score_repair),
        (ResidentLifeActionChoice::DeliverContract, score_deliver),
        (ResidentLifeActionChoice::RestockGrain, score_restock),
        (ResidentLifeActionChoice::FloodgateCheck, score_floodgate),
        (ResidentLifeActionChoice::DineParley, score_dine),
        (ResidentLifeActionChoice::NightRest, score_rest),
        (ResidentLifeActionChoice::MillGrain, score_mill),
    ];

    candidates
        .into_iter()
        .max_by_key(|&(_, s)| s)
        .map(|(c, _)| c)
        .unwrap_or(ResidentLifeActionChoice::MillGrain)
}

pub fn step_flagship_specimen_qin_shuisheng_life_loop(
    store: &mut crate::store::Store,
    game_hour: i32,
    now_unix: i64,
) -> (i32, i32, String) {
    step_canonical_resident_human_life_loop(
        store,
        "秦水生",
        -3,
        125,
        "老堰頭・水輪石磨工坊",
        "引流老堰頭急湍活水驅動雙疊石磨，傾聽磨盤節奏精細碾磨新收稻穀",
        game_hour,
        now_unix,
    )
}

fn inspect_raw_stock_and_tool(
    entity_id: &str,
    display_title: &str,
    equipment_slots: &str,
    inventory: &str,
) -> (String, String, bool, VesselStateRecord, i32, i32) {
    let mut tool_id = String::new();
    let mut tool_name = String::new();
    let mut is_bladed = false;
    let mut equipped_ids = std::collections::HashSet::new();

    if let Ok(slots) = serde_json::from_str::<std::collections::HashMap<String, String>>(equipment_slots) {
        for (_, val) in &slots {
            if !val.trim().is_empty() {
                equipped_ids.insert(val.clone());
            }
        }
        for slot in &["hold_r", "hold_l"] {
            if let Some(tid) = slots.get(*slot).filter(|id| !id.trim().is_empty()) {
                let name = crate::game::peninsula_catalog::find_catalog_entry(tid)
                    .map(|c| c.name.to_string())
                    .or_else(|| crate::game::object_ecology::lookup_ecology_item_def(tid).map(|d| d.name.to_string()))
                    .or_else(|| crate::db::inventory::fallback_item_name(tid).map(|s| s.to_string()))
                    .unwrap_or_else(|| crate::npc::causal_core::resolve_vocational_vessel(display_title).display_name.to_string());
                let bladed = !tid.contains("0973") && !tid.contains("abacus") && !tid.contains("basket");
                tool_id = tid.clone();
                tool_name = name;
                is_bladed = bladed;
                break;
            }
        }
    }

    if tool_id.is_empty() {
        let vessel = crate::npc::causal_core::resolve_vocational_vessel(display_title);
        tool_id = vessel.item_id.to_string();
        tool_name = vessel.display_name.to_string();
        is_bladed = vessel.is_bladed;
    }

    let vstate = get_vessel_state(entity_id, &tool_id);

    let mut raw_stock_qty = 0i32;
    let mut finished_stock_qty = 0i32;
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(inventory) {
        for v in items {
            let id = v.get("item_id").or_else(|| v.get("id")).and_then(|s| s.as_str()).unwrap_or("");
            let qty = v.get("qty").or_else(|| v.get("count")).and_then(|q| q.as_i64()).unwrap_or(0) as i32;
            if id.starts_with("wt-good-") || id.starts_with("mat_") || id.starts_with("harvest_") {
                raw_stock_qty += qty;
            } else if id.starts_with("craft_") || (id.starts_with("wt_cat_") && !equipped_ids.contains(id)) {
                finished_stock_qty += qty;
            }
        }
    }

    (tool_id, tool_name, is_bladed, vstate, raw_stock_qty, finished_stock_qty)
}

/// 依實體角色資料檢視其當前手持／職務器物與庫存原料、成品狀態
pub fn inspect_entity_live_stock_and_tool(
    ent: &crate::entity::Character,
) -> (String, String, bool, VesselStateRecord, i32, i32) {
    inspect_raw_stock_and_tool(&ent.id, &ent.display_title, &ent.equipment_slots, &ent.inventory)
}

/// 依 store 實體資料檢視其當前手持／職務器物與庫存原料、成品狀態
pub fn inspect_store_entity_live_stock_and_tool(
    ent: &crate::store::Entity,
) -> (String, String, bool, VesselStateRecord, i32, i32) {
    inspect_raw_stock_and_tool(&ent.id, &ent.display_title, &ent.equipment_slots, &ent.inventory)
}

fn record_resident_continuity_row(
    entity_id: &str,
    intent: &str,
    reason: &str,
    dest_place: &str,
    now_unix: i64,
) {
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        let next_wake = now_unix + 60;
        let _ = client.execute(
            "INSERT INTO resident_continuity (
                entity_id, control_mode, life_status, current_intent,
                intent_reason, destination_place_id, intent_started_at,
                next_wake_at, updated_at, source_kind, source_ref
            ) VALUES (
                $1, 'autonomous', 'alive', $2,
                $3, $4, $5,
                $6, $5, 'four_question_loop', 'canonical_resident_life_loop'
            )
            ON CONFLICT (entity_id) DO UPDATE SET
                control_mode = 'autonomous',
                life_status = 'alive',
                current_intent = EXCLUDED.current_intent,
                intent_reason = EXCLUDED.intent_reason,
                destination_place_id = EXCLUDED.destination_place_id,
                blocked_by = NULL,
                source_kind = EXCLUDED.source_kind,
                source_ref = EXCLUDED.source_ref,
                updated_at = EXCLUDED.updated_at",
            &[&entity_id, &intent, &reason, &dest_place, &now_unix, &next_wake],
        );
    }
}

#[derive(Debug, Clone)]
pub struct ResidentIntrinsicFactors {
    pub tool_id: String,
    pub tool_name: String,
    pub is_bladed: bool,
    pub secondary_tool_id: Option<String>,
    pub secondary_tool_name: Option<String>,
    pub maintenance_station: String,
    pub raw_item_id: String,
    pub raw_item_name: String,
    pub raw_item_unit: &'static str,
    pub raw_consume_qty: i32,
    pub outputs: Vec<(String, String, &'static str, i32)>,
    pub delivery_items: Vec<(String, String, &'static str, i32)>,
    pub delivery_payout_mg: i32,
    pub delivery_vessel_desc: String,
    pub need_supply_tag: &'static str,
    pub need_buyer_tag: &'static str,
    pub supplier_id: String,
    pub supplier_place_label: String,
    pub supplier_coord: Option<(i32, i32)>,
    pub restock_qty: i32,
    pub restock_cost_mg: i32,
    pub counterparty_id: String,
    pub work_coord: (i32, i32),
    pub deliver_coord: (i32, i32),
    pub floodgate_coord: (i32, i32),
    pub canteen_coord: (i32, i32),
    pub home_coord: (i32, i32),
    pub work_bldg: String,
    pub deliver_bldg: String,
    pub floodgate_bldg: String,
    pub floodgate_facility: String,
    pub canteen_bldg: String,
    pub home_bldg: String,
    pub settlement: String,
    pub work_activity_desc: String,
    pub anatomy_familiarity_subject: String,
}

impl ResidentIntrinsicFactors {
    pub fn resolve(
        ent: &crate::store::Entity,
        sched_x: i32,
        sched_y: i32,
        sched_bldg: &str,
        sched_act: &str,
    ) -> Self {
        let (mut tool_id, mut tool_name, mut is_bladed, _, _, _) =
            inspect_store_entity_live_stock_and_tool(ent);
        let mut hold_r = String::new();
        let mut hold_l = String::new();
        let mut waist_id = String::new();
        if let Ok(slots) =
            serde_json::from_str::<std::collections::HashMap<String, String>>(&ent.equipment_slots)
        {
            if let Some(r) = slots.get("hold_r") {
                hold_r = r.clone();
            }
            if let Some(l) = slots.get("hold_l") {
                hold_l = l.clone();
            }
            if let Some(w) = slots.get("waist") {
                waist_id = w.clone();
            }
        }

        let imprints = get_entity_lineage_imprints(&ent.id);
        let has_mill_factor = hold_r == "wt_cat_0984"
            || hold_l == "wt_cat_0973"
            || ent.display_title.contains("水輪")
            || ent.display_title.contains("石磨")
            || sched_bldg.contains("石磨")
            || sched_bldg.contains("水輪")
            || sched_act.contains("石磨")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("石磨") || imp.imprint_title.contains("水輪"));
        let has_smith_factor = tool_id.contains("hammer")
            || ent.display_title.contains("鐵")
            || ent.display_title.contains("鍛")
            || sched_bldg.contains("鐵")
            || sched_bldg.contains("鍛")
            || ent.inventory.contains("wt_cat_0387")
            || ent.inventory.contains("wt-good-019")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("鍛") || imp.imprint_title.contains("落錘"));
        let has_carpenter_factor = tool_id.contains("saw")
            || tool_id.contains("chisel")
            || ent.display_title.contains("木")
            || ent.display_title.contains("匠")
            || sched_bldg.contains("木")
            || ent.inventory.contains("craft_wooden_tool_handle")
            || ent.inventory.contains("wt-good-012")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("榫卯") || imp.imprint_title.contains("木"));
        let has_herbal_factor = tool_id.contains("mortar")
            || ent.display_title.contains("藥")
            || ent.display_title.contains("醫")
            || sched_bldg.contains("藥")
            || sched_bldg.contains("診")
            || ent.inventory.contains("wt_cat_0293")
            || ent.inventory.contains("wt-good-025")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("藥") || imp.imprint_title.contains("百草"));
        let has_hunt_leather_factor = tool_id.contains("bow")
            || tool_id.contains("knife")
            || tool_id == "wt_cat_1021"
            || ent.display_title.contains("獵")
            || ent.display_title.contains("皮")
            || ent.display_title.contains("弓")
            || sched_bldg.contains("獵")
            || sched_bldg.contains("皮")
            || sched_bldg.contains("弓")
            || sched_act.contains("獵")
            || sched_act.contains("皮")
            || sched_act.contains("弓")
            || ent.inventory.contains("wt_cat_0203")
            || ent.inventory.contains("wt_cat_0877")
            || ent.inventory.contains("wt_cat_1021")
            || ent.inventory.contains("鹿皮")
            || ent.inventory.contains("皮")
            || ent.inventory.contains("獵")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("獵") || imp.imprint_title.contains("皮") || imp.imprint_title.contains("弓"));
        let has_fish_salt_factor = tool_id.contains("rod")
            || tool_id == "wt_cat_1006"
            || ent.display_title.contains("漁")
            || ent.display_title.contains("鹽")
            || ent.display_title.contains("海")
            || sched_bldg.contains("漁")
            || sched_bldg.contains("鹽")
            || sched_bldg.contains("海")
            || sched_act.contains("漁")
            || sched_act.contains("鹽")
            || sched_act.contains("海")
            || ent.inventory.contains("wt_cat_0229")
            || ent.inventory.contains("wt_cat_0254")
            || ent.inventory.contains("wt_cat_1006")
            || ent.inventory.contains("海鹽")
            || ent.inventory.contains("魚")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("漁") || imp.imprint_title.contains("鹽") || imp.imprint_title.contains("海"));
        let has_weave_tailor_factor = tool_id.contains("needle")
            || tool_id.contains("shears")
            || tool_id == "wt_cat_0890"
            || ent.display_title.contains("織")
            || ent.display_title.contains("布")
            || ent.display_title.contains("染")
            || ent.display_title.contains("針")
            || sched_bldg.contains("織")
            || sched_bldg.contains("布")
            || sched_bldg.contains("染")
            || sched_bldg.contains("針")
            || sched_act.contains("織")
            || sched_act.contains("布")
            || sched_act.contains("染")
            || sched_act.contains("針")
            || ent.inventory.contains("wt_cat_0869")
            || ent.inventory.contains("wt_cat_0835")
            || ent.inventory.contains("wt_cat_0890")
            || ent.inventory.contains("布")
            || ent.inventory.contains("織")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("織") || imp.imprint_title.contains("布") || imp.imprint_title.contains("染"));
        let has_farming_factor = tool_id.contains("plow")
            || tool_id.contains("hoe")
            || tool_id == "wt_cat_0950"
            || tool_id == "wt_cat_0981"
            || ent.display_title.contains("農")
            || ent.display_title.contains("田")
            || ent.display_title.contains("耕")
            || sched_bldg.contains("農")
            || sched_bldg.contains("田")
            || sched_bldg.contains("耕")
            || sched_act.contains("農")
            || sched_act.contains("田")
            || sched_act.contains("耕")
            || ent.inventory.contains("wt_cat_0950")
            || ent.inventory.contains("wt_cat_0981")
            || ent.inventory.contains("wt_cat_0277")
            || ent.inventory.contains("田")
            || ent.inventory.contains("耕")
            || imprints
                .iter()
                .any(|imp| imp.imprint_title.contains("農") || imp.imprint_title.contains("田") || imp.imprint_title.contains("耕"));

        let settlement = if has_mill_factor {
            "老堰頭".to_string()
        } else {
            sched_bldg.split('・').next().unwrap_or("文曜").to_string()
        };

        let secondary_tool_id = if !hold_l.is_empty() && hold_l != tool_id {
            Some(hold_l.clone())
        } else if has_mill_factor {
            Some("wt_cat_0973".to_string())
        } else {
            None
        };
        let secondary_tool_name = secondary_tool_id.as_deref().map(|tid| {
            crate::game::peninsula_catalog::find_catalog_entry(tid)
                .map(|c| c.name.to_string())
                .or_else(|| {
                    crate::game::object_ecology::lookup_ecology_item_def(tid)
                        .map(|d| d.name.to_string())
                })
                .unwrap_or_else(|| crate::db::get_item_name(tid))
        });

        if has_mill_factor && (tool_id.is_empty() || tool_id == "wt_cat_0973") {
            tool_id = "wt_cat_0984".to_string();
            tool_name = "羊角鐵錘打石手鏨".to_string();
            is_bladed = true;
        } else if has_smith_factor && tool_id.is_empty() {
            tool_id = "wt_blacksmith_hammer".to_string();
            tool_name = "岑氏省力導能鍛錘".to_string();
            is_bladed = false;
        } else if has_carpenter_factor && tool_id.is_empty() {
            tool_id = "wt_carpenter_chisel".to_string();
            tool_name = "夾鋼手作木工鑿刀".to_string();
            is_bladed = true;
        } else if has_herbal_factor && tool_id.is_empty() {
            tool_id = "wt_cat_0968".to_string();
            tool_name = "打磨青石雙槽研磨缽".to_string();
            is_bladed = false;
        } else if has_hunt_leather_factor && tool_id.is_empty() {
            tool_id = "wt_cat_1021".to_string();
            tool_name = "岩羊角手柄折疊獵刀".to_string();
            is_bladed = true;
        } else if has_fish_salt_factor && tool_id.is_empty() {
            tool_id = "wt_cat_1006".to_string();
            tool_name = "老青竹黃銅輪捲線釣竿".to_string();
            is_bladed = false;
        } else if has_weave_tailor_factor && tool_id.is_empty() {
            tool_id = "wt_cat_0890".to_string();
            tool_name = "熟鐵裁布大剪刀".to_string();
            is_bladed = true;
        } else if has_farming_factor && tool_id.is_empty() {
            tool_id = "wt_cat_0981".to_string();
            tool_name = "長把平頭挖泥鐵鍬".to_string();
            is_bladed = true;
        }

        let (
            maintenance_station,
            raw_item_id,
            raw_item_name,
            raw_item_unit,
            outputs,
            delivery_items,
            need_supply_tag,
            need_buyer_tag,
            supplier_place_label,
            work_activity_desc,
            anatomy_familiarity_subject,
        ) = if has_mill_factor {
            (
                "工坊引水渠旁的活水磨石台".to_string(),
                "wt-good-009".to_string(),
                "精碾白米".to_string(),
                "袋",
                vec![
                    ("wt_cat_0388".to_string(), "溫米漿".to_string(), "桶", 1),
                    ("wt_cat_0279".to_string(), "在來米粉".to_string(), "袋", 1),
                ],
                vec![
                    ("wt_cat_0388".to_string(), "現磨溫米漿".to_string(), "桶", 2),
                    ("wt_cat_0279".to_string(), "特級在來米粉".to_string(), "袋", 1),
                ],
                "穀物",
                "食堂",
                "大圳頭".to_string(),
                "巡看老堰頭湍急溪水推動的重礦包邊大石磨，將運來的稻麥與北山藥材碾得細膩均勻"
                    .to_string(),
                "在來米與北山藥材水磨澄漿".to_string(),
            )
        } else if has_smith_factor {
            (
                format!("{}淬火石槽旁的活水磨石台", sched_bldg),
                "wt-good-019".to_string(),
                "赤鐵粗礦".to_string(),
                "簍",
                vec![("wt_cat_0387".to_string(), "百鍛農具鐵件".to_string(), "件", 1)],
                vec![("wt_cat_0387".to_string(), "百鍛農具鐵件".to_string(), "件", 2)],
                "礦石",
                "市集",
                "西嶺礦谷".to_string(),
                if sched_act.is_empty() {
                    format!("在{}揮動{}鍛打赤鐵粗礦，校準器物火候與分寸", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "百鍛鐵胚紋理與落錘火候".to_string(),
            )
        } else if has_carpenter_factor {
            (
                format!("{}木作案旁的活水磨石台", sched_bldg),
                "wt-good-012".to_string(),
                "北山老杉木料".to_string(),
                "根",
                vec![("craft_wooden_tool_handle".to_string(), "堅韌硬木工具長柄".to_string(), "件", 1)],
                vec![("craft_wooden_tool_handle".to_string(), "堅韌硬木工具長柄".to_string(), "件", 2)],
                "木材",
                "工坊",
                "霜林口".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}刨削北山老杉木料，校合榫卯分寸", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "北山杉木紋理與榫卯合度".to_string(),
            )
        } else if has_herbal_factor {
            (
                format!("{}碾槽旁的活水磨石台", sched_bldg),
                "wt-good-025".to_string(),
                "北山靈草".to_string(),
                "束",
                vec![("wt_cat_0293".to_string(), "清心草藥散".to_string(), "包", 1)],
                vec![("wt_cat_0293".to_string(), "清心草藥散".to_string(), "包", 2)],
                "藥材",
                "商鋪",
                "北山藥圃".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}分揀炮製北山靈草", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "北山百草藥性與炮製分寸".to_string(),
            )
        } else if has_hunt_leather_factor {
            (
                format!("{}皮作案旁的活水磨石台", sched_bldg),
                "wt_cat_0203".to_string(),
                "高地水鹿厚鹿皮".to_string(),
                "張",
                vec![("wt_cat_0877".to_string(), "鞣製黃牛皮寬版腰帶".to_string(), "條", 1)],
                vec![("wt_cat_0877".to_string(), "鞣製黃牛皮寬版腰帶".to_string(), "條", 2)],
                "皮革",
                "商鋪",
                "長風草甸".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}熟化硝製厚鹿皮，修裁筋骨分寸", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "獸皮鞣製紋理與筋膜分寸".to_string(),
            )
        } else if has_fish_salt_factor {
            (
                format!("{}防浪棧道旁的收繩台", sched_bldg),
                "wt_cat_0229".to_string(),
                "南灣粗晶海鹽".to_string(),
                "袋",
                vec![("wt_cat_0254".to_string(), "海風日曬飛魚乾".to_string(), "條", 1)],
                vec![("wt_cat_0254".to_string(), "海風日曬飛魚乾".to_string(), "條", 2)],
                "海鹽",
                "食堂",
                "南灣漁港".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}分揀醃製鮮魚，調和海鹽鹹度", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "海風潮候與魚鱗鹽漬分寸".to_string(),
            )
        } else if has_weave_tailor_factor {
            (
                format!("{}裁案旁的活水磨石台", sched_bldg),
                "wt_cat_0869".to_string(),
                "粗麻雙帶布料".to_string(),
                "匹",
                vec![("wt_cat_0835".to_string(), "細棉麻透氣常服".to_string(), "件", 1)],
                vec![("wt_cat_0835".to_string(), "細棉麻透氣常服".to_string(), "件", 2)],
                "布料",
                "商鋪",
                "文曜織坊".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}裁切粗麻布匹，走線縫綴衣物", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "粗麻經緯經線與裁剪分寸".to_string(),
            )
        } else if has_farming_factor {
            (
                format!("{}田埂邊的活水磨石台", sched_bldg),
                "wt-good-009".to_string(),
                "精選稻種麥穀".to_string(),
                "袋",
                vec![("wt_cat_0277".to_string(), "秋收蓬萊新米".to_string(), "袋", 1)],
                vec![("wt_cat_0277".to_string(), "秋收蓬萊新米".to_string(), "袋", 2)],
                "穀物",
                "工坊",
                "大圳頭".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}翻整田土，順應節氣引水灌溉", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "水脈土壤墒情與稻穀生息分寸".to_string(),
            )
        } else {
            (
                format!("{}旁的活水磨石台", sched_bldg),
                "wt-good-009".to_string(),
                "精碾白米".to_string(),
                "袋",
                vec![("wt_cat_0388".to_string(), "溫米漿".to_string(), "桶", 1)],
                vec![("wt_cat_0388".to_string(), "現磨溫米漿".to_string(), "桶", 2)],
                "穀物",
                "食堂",
                "大圳頭".to_string(),
                if sched_act.is_empty() {
                    format!("在{}運使{}踏實料理手頭差事", sched_bldg, tool_name)
                } else {
                    sched_act.to_string()
                },
                "半島百工器物與材料分寸".to_string(),
            )
        };

        let delivery_vessel_desc = if waist_id == "wt_cat_0897" || has_mill_factor {
            "薄層重礦烏木膽溫存桶".to_string()
        } else {
            "隨身背簍與貨囊".to_string()
        };

        let cur_pos = ent.grid_x.zip(ent.grid_y).unwrap_or((sched_x, sched_y));
        let work_coord = if has_mill_factor { (-3, 125) } else { (sched_x, sched_y) };
        let work_bldg = if has_mill_factor {
            "老堰頭・水輪石磨工坊".to_string()
        } else {
            sched_bldg.to_string()
        };

        let fb_deliver_coord = (-4, 4);
        let fb_deliver_bldg = "文曜・三水大食堂";
        let (deliver_coord, deliver_bldg) = resolve_nearest_room_by_tag(
            cur_pos,
            &[need_buyer_tag, "食堂", "商鋪", "市集"],
            None,
            fb_deliver_coord,
            fb_deliver_bldg,
        );

        let fb_floodgate_coord = if settlement == "老堰頭" { (-1, 124) } else { (sched_x, sched_y) };
        let fb_floodgate_bldg = if settlement == "老堰頭" {
            "老堰頭・清泉水脈"
        } else {
            sched_bldg
        };
        let (floodgate_coord, floodgate_bldg) = resolve_nearest_room_by_tag(
            cur_pos,
            &["分洪閘", "水脈", "水渠"],
            None,
            fb_floodgate_coord,
            fb_floodgate_bldg,
        );

        let floodgate_facility = if has_mill_factor {
            "一號分洪木閘，調緩入渠水勢以護住重礦大石磨與水青岡木齒".to_string()
        } else {
            format!("{}周邊溝渠分洪閘板與排水道", sched_bldg)
        };
        let fb_canteen_coord = if settlement == "老堰頭" { (1, 124) } else { (-4, 4) };
        let fb_canteen_bldg = if settlement == "老堰頭" {
            "老堰頭・堰渠水畔食堂"
        } else {
            "文曜・三水大食堂"
        };
        let (canteen_coord, canteen_bldg) = resolve_nearest_room_by_tag(
            cur_pos,
            &["食堂", "熱食", "現煮飲品"],
            None,
            fb_canteen_coord,
            fb_canteen_bldg,
        );
        let (home_coord, home_bldg) = if has_mill_factor {
            ((-3, 124), "老堰頭・秦氏水輪石磨傳承老宅".to_string())
        } else {
            let slot = crate::store::wenyao_household_slot_for(&ent.id);
            ((slot.home_x, slot.home_y), slot.home_name.to_string())
        };

        Self {
            tool_id,
            tool_name,
            is_bladed,
            secondary_tool_id,
            secondary_tool_name,
            maintenance_station,
            raw_item_id,
            raw_item_name,
            raw_item_unit,
            raw_consume_qty: 1,
            outputs,
            delivery_items,
            delivery_payout_mg: 16,
            delivery_vessel_desc,
            need_supply_tag,
            need_buyer_tag,
            supplier_id: String::new(),
            supplier_place_label,
            supplier_coord: None,
            restock_qty: 6,
            restock_cost_mg: 12,
            counterparty_id: String::new(),
            work_coord,
            deliver_coord,
            floodgate_coord,
            canteen_coord,
            home_coord,
            work_bldg,
            deliver_bldg,
            floodgate_bldg,
            floodgate_facility,
            canteen_bldg,
            home_bldg,
            settlement,
            work_activity_desc,
            anatomy_familiarity_subject,
        }
    }
}

fn is_grudge_hook(h: &crate::npc::causal_core::RelationalHook) -> bool {
    ["遭拒", "追討", "結怨", "嫌隙", "展期"]
        .iter()
        .any(|k| h.kind_label.contains(k) || h.summary.contains(k))
}

fn is_favor_hook(h: &crate::npc::causal_core::RelationalHook) -> bool {
    ["恩", "守諾", "解囊", "馳援", "完修", "交割", "供料"]
        .iter()
        .any(|k| h.kind_label.contains(k) || h.summary.contains(k))
}

/// 辨識現場人員：透過裝備槽位可見制服／圍裙／工務裝束或站在管制櫃檯區（非本號夥計禁止入內區）精準辨識值守店員或掌事
pub fn is_recognizable_room_staff_or_counter_holder(
    room_name: &str,
    npc_id: &str,
    display_title: &str,
    current_activity: &str,
    equipment_slots: &str,
) -> (bool, bool, bool) {
    let mut has_visible_uniform = false;
    if let Ok(slots) =
        serde_json::from_str::<std::collections::HashMap<String, String>>(equipment_slots)
    {
        for slot in &["body", "waist", "wrist", "hold_r", "hold_l"] {
            if let Some(item_id) = slots.get(*slot).filter(|s| !s.trim().is_empty()) {
                if crate::game::peninsula_catalog::is_work_uniform_item_id_or_name(item_id) {
                    has_visible_uniform = true;
                    break;
                }
            }
        }
    }
    if !has_visible_uniform {
        if display_title.contains("穿制服")
            || display_title.contains("店員")
            || display_title.contains("制服")
            || display_title.contains("圍裙")
            || display_title.contains("工長")
            || display_title.contains("司秤")
            || display_title.contains("主廚")
        {
            has_visible_uniform = true;
        }
    }
    if !has_visible_uniform && !npc_id.is_empty() {
        let slot = crate::store::wenyao_household_slot_for(npc_id);
        if crate::game::peninsula_catalog::is_work_uniform_item_id_or_name(slot.attire_line) {
            has_visible_uniform = true;
        }
    }

    let is_passerby_customer = display_title.contains("顧客")
        || display_title.contains("路人")
        || display_title.contains("茶客")
        || display_title.contains("食客")
        || display_title.contains("香客")
        || display_title.contains("過客")
        || display_title.contains("外地商旅");

    let mut has_counter_zone_marker = display_title.contains("櫃檯")
        || display_title.contains("櫃台")
        || display_title.contains("非夥計禁止入內")
        || display_title.contains("非本店人員禁止進入區")
        || display_title.contains("非本號夥計禁止入內區")
        || display_title.contains("內場")
        || display_title.contains("掌事")
        || display_title.contains("主事")
        || display_title.contains("當值")
        || display_title.contains("首座");

    if !has_counter_zone_marker {
        if let Some(v) = crate::grid::resolve_any_room_venue_object(room_name, display_title) {
            if v.has_counter_restricted_zone
                && (current_activity.contains("櫃")
                    || current_activity.contains("爐")
                    || current_activity.contains("釜")
                    || current_activity.contains("砧")
                    || current_activity.contains("磨")
                    || current_activity.contains("藥")
                    || current_activity.contains("算")
                    || current_activity.contains("帳")
                    || current_activity.contains("攤"))
            {
                has_counter_zone_marker = true;
            }
        }
    }

    let in_staff_only_counter_zone = if is_passerby_customer && !has_counter_zone_marker {
        false
    } else if has_counter_zone_marker {
        true
    } else if has_visible_uniform
        && !is_passerby_customer
        && crate::grid::resolve_any_room_venue_object(room_name, display_title)
            .is_some_and(|v| v.has_counter_restricted_zone)
    {
        true
    } else {
        false
    };

    let is_staff = has_visible_uniform || in_staff_only_counter_zone;
    (is_staff, has_visible_uniform, in_staff_only_counter_zone)
}

/// 舊式依實體辨識現場人員輔助函數
pub fn is_recognizable_room_staff_or_counter_holder_entity(
    ent: &crate::store::Entity,
    _room_coord: (i32, i32),
    needed_item_id: &str,
) -> (bool, bool) {
    // 1. 檢查裝備槽位是否有可見制服／圍裙／工務裝束
    let mut has_visible_uniform = false;
    if let Ok(slots) =
        serde_json::from_str::<std::collections::HashMap<String, String>>(&ent.equipment_slots)
    {
        for slot in &["body", "waist", "wrist", "hold_r", "hold_l"] {
            if let Some(item_id) = slots.get(*slot).filter(|s| !s.trim().is_empty()) {
                if crate::game::peninsula_catalog::is_work_uniform_item_id_or_name(item_id) {
                    has_visible_uniform = true;
                    break;
                }
            }
        }
    }
    if !has_visible_uniform {
        if ent.display_title.contains("穿制服")
            || ent.display_title.contains("店員")
            || ent.display_title.contains("制服")
            || ent.display_title.contains("圍裙")
            || ent.display_title.contains("工長")
            || ent.display_title.contains("司秤")
            || ent.display_title.contains("主廚")
        {
            has_visible_uniform = true;
        }
    }
    if !has_visible_uniform && !ent.id.is_empty() {
        let slot = crate::store::wenyao_household_slot_for(&ent.id);
        if crate::game::peninsula_catalog::is_work_uniform_item_id_or_name(slot.attire_line) {
            has_visible_uniform = true;
        }
    }

    // 2. 檢查是否站立於該房間之「非本號夥計禁止入內區／櫃檯內場」
    // 即便身穿便服，站在內場櫃檯管制區，或持有該店舖原料庫存且具備常駐站席，即辨識為在場值守掌事／櫃檯主事
    let is_passerby_customer = ent.display_title.contains("顧客")
        || ent.display_title.contains("路人")
        || ent.display_title.contains("茶客")
        || ent.display_title.contains("食客")
        || ent.display_title.contains("香客")
        || ent.display_title.contains("過客")
        || ent.display_title.contains("外地商旅");

    let mut has_counter_zone_marker = ent.display_title.contains("櫃檯")
        || ent.display_title.contains("櫃台")
        || ent.display_title.contains("非夥計禁止入內")
        || ent.display_title.contains("非本店人員禁止進入區")
        || ent.display_title.contains("非本號夥計禁止入內區")
        || ent.display_title.contains("內場")
        || ent.display_title.contains("掌事")
        || ent.display_title.contains("主事")
        || ent.display_title.contains("當值")
        || ent.display_title.contains("首座");

    if !has_counter_zone_marker {
        if let Some(v) = crate::grid::resolve_any_room_venue_object(&ent.current_activity, &ent.display_title) {
            if v.has_counter_restricted_zone
                && (ent.current_activity.contains("櫃")
                    || ent.current_activity.contains("爐")
                    || ent.current_activity.contains("釜")
                    || ent.current_activity.contains("砧")
                    || ent.current_activity.contains("磨")
                    || ent.current_activity.contains("藥")
                    || ent.current_activity.contains("算")
                    || ent.current_activity.contains("帳")
                    || ent.current_activity.contains("攤"))
            {
                has_counter_zone_marker = true;
            }
        }
    }

    let stock_count = if !needed_item_id.is_empty() {
        inv_count_item(&ent.inventory, needed_item_id)
    } else {
        0
    };

    let in_staff_only_counter_zone = if is_passerby_customer && stock_count <= 0 && !has_counter_zone_marker {
        false
    } else if has_counter_zone_marker {
        true
    } else if (stock_count > 0 || ent.magnesium >= 8) && !is_passerby_customer {
        // 持有該店庫存或商業結算能力且非路人顧客，視為在場值守人員
        true
    } else {
        false
    };

    (has_visible_uniform, in_staff_only_counter_zone)
}

/// 依原點座標 (origin.0, origin.1) 與標籤清單，於方格地圖中搜尋半徑內最近之房間設施
pub fn resolve_nearest_room_by_tag(
    origin: (i32, i32),
    wanted_tags: &[&str],
    grid_opt: Option<&crate::grid::SquareGrid>,
    fallback_coord: (i32, i32),
    fallback_bldg: &str,
) -> ((i32, i32), String) {
    let runtime_grid_holder;
    let grid_ref = match grid_opt {
        Some(g) => Some(g),
        None => {
            runtime_grid_holder = crate::server::grid_manager::get_square_grid();
            runtime_grid_holder.as_ref()
        }
    };

    let Some(grid) = grid_ref else {
        return (fallback_coord, fallback_bldg.to_string());
    };

    let mut best: Option<((i32, i32), String, i32)> = None;
    for cell in grid.cells() {
        let rcoord = (cell.coord.x, cell.coord.y);
        let dist = (rcoord.0 - origin.0).abs() + (rcoord.1 - origin.1).abs();
        if dist > 45 {
            continue;
        }
        let tag_match = wanted_tags.iter().any(|&tag| !tag.is_empty() && cell.has_room_tag(tag));
        if !tag_match {
            continue;
        }

        let is_better = match &best {
            None => true,
            Some((best_coord, _, best_dist)) => {
                if dist < *best_dist {
                    true
                } else if dist == *best_dist {
                    rcoord < *best_coord
                } else {
                    false
                }
            }
        };

        if is_better {
            let bldg_name = if cell.name.trim().is_empty() {
                if !cell.zone.is_empty() {
                    cell.zone.clone()
                } else {
                    fallback_bldg.to_string()
                }
            } else if cell.name.contains('・') || cell.zone.is_empty() {
                cell.name.clone()
            } else {
                format!("{}・{}", cell.zone, cell.name)
            };
            best = Some((rcoord, bldg_name, dist));
        }
    }

    if let Some((b_coord, b_bldg, b_dist)) = best {
        let fb_dist = (fallback_coord.0 - origin.0).abs() + (fallback_coord.1 - origin.1).abs();
        if (fallback_coord == (-1, 124) || fallback_coord == (1, 124)) && fb_dist <= b_dist {
            (fallback_coord, fallback_bldg.to_string())
        } else {
            (b_coord, b_bldg)
        }
    } else {
        (fallback_coord, fallback_bldg.to_string())
    }
}

/// 依當前座標 (cx, cy) 尋找半徑內最具優勢之上游供貨商或房間設施櫃檯
pub fn resolve_need_decision_supplier(
    store: &crate::store::Store,
    entity_id: &str,
    cur_xy: (i32, i32),
    factors: &ResidentIntrinsicFactors,
    pw: &crate::npc::causal_core::PersonalityWeights,
    hooks: &[crate::npc::causal_core::RelationalHook],
    grid_opt: Option<&crate::grid::SquareGrid>,
) -> Option<(String, String, (i32, i32))> {
    let mut best: Option<(String, String, (i32, i32), i32)> = None;

    // 1. 掃描 Store 內之實體候選人（上游供貨商／掌事／店員）
    for (cand_id, cand_ent) in &store.entities {
        if cand_id == entity_id {
            continue;
        }
        let stock = inv_count_item(&cand_ent.inventory, &factors.raw_item_id);
        if stock <= 0 {
            continue;
        }

        let cand_coord = cand_ent
            .grid_x
            .zip(cand_ent.grid_y)
            .unwrap_or(factors.work_coord);

        let (has_uniform, in_counter_zone) =
            is_recognizable_room_staff_or_counter_holder_entity(cand_ent, cand_coord, &factors.raw_item_id);

        if !has_uniform
            && !in_counter_zone
            && (cand_ent.display_title.contains("顧客") || cand_ent.display_title.contains("路人"))
        {
            continue;
        }

        let mut trust = 0i32;
        let mut grudge = 0i32;
        if let Some(h) = hooks
            .iter()
            .find(|h| h.target_id == *cand_id || h.target_name == *cand_id)
        {
            if is_grudge_hook(h) {
                grudge = h.intensity as i32;
            } else if is_favor_hook(h) {
                trust = h.intensity as i32;
            }
        }
        if grudge >= 15 && pw.pride >= 95 {
            continue;
        }

        let dist = (cand_coord.0 - cur_xy.0).abs() + (cand_coord.1 - cur_xy.1).abs();

        let tag_match = grid_opt
            .and_then(|g| g.get(crate::grid::SquareCoord::new(cand_coord.0, cand_coord.1)))
            .map_or(false, |c| c.has_room_tag(factors.need_supply_tag));

        let tag_bonus = if tag_match { 200 } else { 0 };
        let uniform_bonus = if has_uniform { 50 } else { 0 };
        let counter_bonus = if in_counter_zone { 50 } else { 0 };
        let dist_penalty = dist * 15;

        let score = stock * 25
            + tag_bonus
            + uniform_bonus
            + counter_bonus
            + trust * (pw.honor / 8).max(4)
            - grudge * (pw.pride / 4).max(10)
            - dist_penalty;

        if best.as_ref().map(|(_, _, _, bs)| score > *bs).unwrap_or(true) {
            let place_label = if let Some(cell) = grid_opt
                .and_then(|g| g.get(crate::grid::SquareCoord::new(cand_coord.0, cand_coord.1)))
            {
                if !cell.zone.is_empty() {
                    cell.zone.clone()
                } else {
                    cell.name.clone()
                }
            } else if cand_ent.display_title.contains('・') {
                cand_ent
                    .display_title
                    .split('・')
                    .next()
                    .unwrap_or(&factors.supplier_place_label)
                    .to_string()
            } else if let Some(s) = crate::store::canonical_npc_macro_settlement(cand_id, &cand_ent.display_title) {
                s.to_string()
            } else {
                factors.supplier_place_label.clone()
            };
            best = Some((cand_id.clone(), place_label, cand_coord, score));
        }
    }

    // 2. 掃描方格地圖中符合需求之房間
    if let Some(grid) = grid_opt {
        for cell in grid.cells() {
            if cell.has_room_tag(factors.need_supply_tag) {
                let rcoord = (cell.coord.x, cell.coord.y);
                let dist = (rcoord.0 - cur_xy.0).abs() + (rcoord.1 - cur_xy.1).abs();

                let staff_opt = store.entities.iter().find(|(cid, ce)| {
                    *cid != entity_id && ce.grid_x.zip(ce.grid_y) == Some(rcoord) && {
                        let (u, c) = is_recognizable_room_staff_or_counter_holder_entity(
                            ce,
                            rcoord,
                            &factors.raw_item_id,
                        );
                        u || c
                    }
                });

                if let Some((staff_id, staff_ent)) = staff_opt {
                    let stock = inv_count_item(&staff_ent.inventory, &factors.raw_item_id).max(4);
                    let (has_uniform, in_counter_zone) = is_recognizable_room_staff_or_counter_holder_entity(
                        staff_ent,
                        rcoord,
                        &factors.raw_item_id,
                    );
                    let uniform_bonus = if has_uniform { 50 } else { 0 };
                    let counter_bonus = if in_counter_zone { 50 } else { 0 };
                    let score = stock * 25 + 200 + uniform_bonus + counter_bonus - dist * 15;
                    if best.as_ref().map(|(_, _, _, bs)| score > *bs).unwrap_or(true) {
                        let place_label = if !cell.zone.is_empty() {
                            cell.zone.clone()
                        } else {
                            cell.name.clone()
                        };
                        best = Some((staff_id.clone(), place_label, rcoord, score));
                    }
                } else {
                    let room_score = 150 - dist * 15;
                    if best.as_ref().map(|(_, _, _, bs)| room_score > *bs).unwrap_or(true) {
                        let place_label = if !cell.zone.is_empty() {
                            cell.zone.clone()
                        } else {
                            cell.name.clone()
                        };
                        let counter_name = format!("{}櫃檯", cell.name);
                        best = Some((counter_name, place_label, rcoord, room_score));
                    }
                }
            }
        }
    }

    best.map(|(id, place, coord, _)| (id, place, coord))
}

/// 依當前座標 (cx, cy) 尋找半徑內最具優勢之下游交割買方或房間設施櫃檯
pub fn resolve_need_decision_buyer(
    store: &crate::store::Store,
    entity_id: &str,
    cur_xy: (i32, i32),
    primary_out_id: &str,
    factors: &ResidentIntrinsicFactors,
    pw: &crate::npc::causal_core::PersonalityWeights,
    hooks: &[crate::npc::causal_core::RelationalHook],
    grid_opt: Option<&crate::grid::SquareGrid>,
) -> Option<(String, String, (i32, i32))> {
    let mut best: Option<(String, String, (i32, i32), i32)> = None;

    // 1. 掃描 Store 內之買方實體（食堂主事／商鋪管事／收購商）
    for (cand_id, cand_ent) in &store.entities {
        if cand_id == entity_id || cand_id == &factors.supplier_id {
            continue;
        }
        if cand_ent.magnesium < 8 {
            continue;
        }

        let cand_coord = cand_ent
            .grid_x
            .zip(cand_ent.grid_y)
            .unwrap_or(factors.deliver_coord);

        let (has_uniform, in_counter_zone) =
            is_recognizable_room_staff_or_counter_holder_entity(cand_ent, cand_coord, primary_out_id);

        if !has_uniform
            && !in_counter_zone
            && (cand_ent.display_title.contains("顧客") || cand_ent.display_title.contains("路人"))
        {
            continue;
        }

        let mut trust = 0i32;
        let mut grudge = 0i32;
        if let Some(h) = hooks
            .iter()
            .find(|h| h.target_id == *cand_id || h.target_name == *cand_id)
        {
            if is_grudge_hook(h) {
                grudge = h.intensity as i32;
            } else if is_favor_hook(h) {
                trust = h.intensity as i32;
            }
        }
        if grudge >= 15 && pw.pride >= 95 {
            continue;
        }

        let dist = (cand_coord.0 - cur_xy.0).abs() + (cand_coord.1 - cur_xy.1).abs();
        let buyer_stock = inv_count_item(&cand_ent.inventory, primary_out_id);
        let stock_need_bonus = if buyer_stock < 4 { 60 } else { 0 };

        let tag_match = grid_opt
            .and_then(|g| g.get(crate::grid::SquareCoord::new(cand_coord.0, cand_coord.1)))
            .map_or(false, |c| c.has_room_tag(factors.need_buyer_tag));

        let tag_bonus = if tag_match { 200 } else { 0 };
        let uniform_bonus = if has_uniform { 50 } else { 0 };
        let counter_bonus = if in_counter_zone { 50 } else { 0 };
        let dist_penalty = dist * 15;

        let score = cand_ent.magnesium.min(50) * 2
            + stock_need_bonus
            + tag_bonus
            + uniform_bonus
            + counter_bonus
            + trust * (pw.honor / 8).max(4)
            - grudge * (pw.pride / 4).max(10)
            - dist_penalty;

        if best.as_ref().map(|(_, _, _, bs)| score > *bs).unwrap_or(true) {
            let place_label = if let Some(cell) = grid_opt
                .and_then(|g| g.get(crate::grid::SquareCoord::new(cand_coord.0, cand_coord.1)))
            {
                if !cell.zone.is_empty() {
                    cell.zone.clone()
                } else {
                    cell.name.clone()
                }
            } else if cand_ent.display_title.contains('・') {
                cand_ent
                    .display_title
                    .split('・')
                    .next()
                    .unwrap_or(&factors.deliver_bldg)
                    .to_string()
            } else if let Some(s) = crate::store::canonical_npc_macro_settlement(cand_id, &cand_ent.display_title) {
                s.to_string()
            } else {
                factors.deliver_bldg.clone()
            };
            best = Some((cand_id.clone(), place_label, cand_coord, score));
        }
    }

    // 2. 掃描方格地圖中符合需求之房間
    if let Some(grid) = grid_opt {
        for cell in grid.cells() {
            if cell.has_room_tag(factors.need_buyer_tag) {
                let rcoord = (cell.coord.x, cell.coord.y);
                let dist = (rcoord.0 - cur_xy.0).abs() + (rcoord.1 - cur_xy.1).abs();

                let staff_opt = store.entities.iter().find(|(cid, ce)| {
                    *cid != entity_id
                        && *cid != &factors.supplier_id
                        && ce.magnesium >= 8
                        && ce.grid_x.zip(ce.grid_y) == Some(rcoord)
                        && {
                            let (u, c) = is_recognizable_room_staff_or_counter_holder_entity(
                                ce,
                                rcoord,
                                primary_out_id,
                            );
                            u || c
                        }
                });

                if let Some((staff_id, staff_ent)) = staff_opt {
                    let (has_uniform, in_counter_zone) = is_recognizable_room_staff_or_counter_holder_entity(
                        staff_ent,
                        rcoord,
                        primary_out_id,
                    );
                    let uniform_bonus = if has_uniform { 50 } else { 0 };
                    let counter_bonus = if in_counter_zone { 50 } else { 0 };
                    let score = staff_ent.magnesium.min(50) * 2
                        + 200
                        + uniform_bonus
                        + counter_bonus
                        - dist * 15;
                    if best.as_ref().map(|(_, _, _, bs)| score > *bs).unwrap_or(true) {
                        let place_label = if !cell.zone.is_empty() {
                            cell.zone.clone()
                        } else {
                            cell.name.clone()
                        };
                        best = Some((staff_id.clone(), place_label, rcoord, score));
                    }
                } else {
                    let room_score = 150 - dist * 15;
                    if best.as_ref().map(|(_, _, _, bs)| room_score > *bs).unwrap_or(true) {
                        let place_label = if !cell.zone.is_empty() {
                            cell.zone.clone()
                        } else {
                            cell.name.clone()
                        };
                        let counter_name = format!("{}櫃檯", cell.name);
                        best = Some((counter_name, place_label, rcoord, room_score));
                    }
                }
            }
        }
    }

    best.map(|(id, place, coord, _)| (id, place, coord))
}

/// 正典住民真人生命迴圈：全體白塔正典住民共用性格權重四問決策與器物磨損動力學（零特例分支）
pub fn step_canonical_resident_human_life_loop(
    store: &mut crate::store::Store,
    entity_id: &str,
    sched_x: i32,
    sched_y: i32,
    sched_bldg: &str,
    sched_act: &str,
    game_hour: i32,
    now_unix: i64,
) -> (i32, i32, String) {
    let Some(mut ent) = store.get_entity(entity_id) else {
        return (sched_x, sched_y, sched_act.to_string());
    };

    let mut factors = ResidentIntrinsicFactors::resolve(&ent, sched_x, sched_y, sched_bldg, sched_act);

    // 若實體在記憶體 Store 尚未掛載基準衣裝或行囊，依其內在職務因子自動補齊，不綁定單一姓名
    if ent.equipment_slots.trim().is_empty() || ent.equipment_slots == "{}" {
        let hold_l_entry = factors
            .secondary_tool_id
            .as_deref()
            .unwrap_or("");
        let slots_json = serde_json::json!({
            "head": "wt_cat_0875",
            "body": "wt_cat_0869",
            "legs": "starter_legs_m",
            "feet": "starter_feet_m",
            "wrist": "wt_cat_0852",
            "waist": "wt_cat_0897",
            "hold_r": factors.tool_id,
            "hold_l": hold_l_entry,
        });
        ent.equipment_slots = slots_json.to_string();
    }
    if ent.activated_nodes.is_empty() || ent.activated_nodes == "[]" || ent.activated_nodes == "[\"N000\"]" {
        ent.activated_nodes = r#"["N000","N012","N028","N045","N077","N112"]"#.to_string();
    }
    if ent.inventory.trim().is_empty() || ent.inventory == "[]" {
        let mut seed_items = Vec::new();
        for (out_id, _, _, _) in &factors.outputs {
            seed_items.push(serde_json::json!({"item_id": out_id, "qty": 6}));
        }
        seed_items.push(serde_json::json!({"item_id": &factors.raw_item_id, "qty": 10}));
        seed_items.push(serde_json::json!({"item_id": &factors.tool_id, "qty": 1}));
        if let Some(sec_id) = &factors.secondary_tool_id {
            seed_items.push(serde_json::json!({"item_id": sec_id, "qty": 1}));
        }
        ent.inventory = serde_json::Value::Array(seed_items).to_string();
    }

    let grain_qty = inv_count_item(&ent.inventory, &factors.raw_item_id);
    let primary_out_id = factors
        .outputs
        .first()
        .map(|(id, _, _, _)| id.as_str())
        .unwrap_or("wt_cat_0388");
    let ricemilk_qty = inv_count_item(&ent.inventory, primary_out_id);
    let vstate = get_vessel_state(entity_id, &factors.tool_id);
    let wear_points = if factors.is_bladed { vstate.wear_points } else { 0 };
    let repetitions = vstate.use_count.max(0) as u32;

    let hooks = crate::npc::causal_core::get_relational_hooks(entity_id);
    let (weather, tide) = crate::game::zone_weather_and_tide(&factors.settlement, now_unix);
    let seed_u64 = crate::combat::human_phase::resolve_entity_soul_seed(entity_id, ent.soul_seed) as u64;
    let stress = ((100 - ent.vit.clamp(0, 100)) as u8).saturating_div(2);
    let pw = crate::npc::causal_core::compute_plural_personality_weights_with_experience(
        &ent.display_title,
        &factors.settlement,
        seed_u64,
        stress,
        ent.magnesium,
        repetitions,
        &hooks,
        None,
    );

    // ── 需求決策：自主依房間標籤、距離代價、制服／內場辨識與恩怨尋商（上游供貨） ──
    let cur_xy = ent.grid_x.zip(ent.grid_y).unwrap_or(factors.work_coord);
    let runtime_grid = crate::server::grid_manager::get_runtime_square_grid();

    if factors.supplier_id.is_empty()
        || !store.entities.contains_key(&factors.supplier_id)
        || inv_count_item(
            &store.entities.get(&factors.supplier_id).map(|e| e.inventory.clone()).unwrap_or_default(),
            &factors.raw_item_id,
        ) <= 0
        || hooks.iter().any(|h| (h.target_id == factors.supplier_id || h.target_name == factors.supplier_id) && is_grudge_hook(h) && h.intensity >= 15 && pw.pride >= 95)
    {
        if let Some((chosen_sup, sup_place, sup_coord)) = resolve_need_decision_supplier(
            store,
            entity_id,
            cur_xy,
            &factors,
            &pw,
            &hooks,
            runtime_grid.as_ref(),
        ) {
            factors.supplier_id = chosen_sup;
            factors.supplier_place_label = sup_place;
            factors.supplier_coord = Some(sup_coord);
        } else {
            factors.supplier_id = format!("{}櫃檯", factors.supplier_place_label);
            factors.supplier_coord = Some(factors.work_coord);
        }
    }

    let mut obligation_deliver_coord: Option<(i32, i32)> = None;
    let mut contract_open = crate::store::get_active_npc_obligation(entity_id).is_some();
    if let Some((_kind, target_hint)) = crate::store::get_active_npc_obligation(entity_id) {
        let cp = target_hint.split('・').next().unwrap_or(target_hint.as_str());
        if !cp.is_empty() {
            factors.counterparty_id = cp.to_string();
        }
        if let Some(cp_ent) = store.entities.get(cp)
            && let (Some(x), Some(y)) = (cp_ent.grid_x, cp_ent.grid_y)
        {
            obligation_deliver_coord = Some((x, y));
        }
    }
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut client) = pool.get()
    {
        // 僅當買方實際庫存見底（< 4）且距上次交割已逾 1 小時，才因真實供需缺口重啟補貨契約（廢除死板 5 點發條重設）
        if (5..=8).contains(&game_hour) && !factors.counterparty_id.is_empty() {
            let buyer_stock = store
                .entities
                .get(&factors.counterparty_id)
                .map(|cp| inv_count_item(&cp.inventory, primary_out_id))
                .unwrap_or(0);
            if buyer_stock < 4 {
                let due = now_unix + 10_800;
                let stale_cutoff = now_unix - 3_600;
                let _ = client.execute(
                    "UPDATE resident_obligations SET status = 'active', due_at = $2, updated_at = $3
                     WHERE entity_id = $1 AND obligation_kind = 'restock_goods'
                       AND status = 'fulfilled' AND updated_at < $4",
                    &[&entity_id, &due, &now_unix, &stale_cutoff],
                );
            }
        }
        if let Ok(rows) = client.query(
            "SELECT subject_ref FROM resident_obligations WHERE entity_id = $1 AND obligation_kind = 'restock_goods' AND status IN ('open', 'due', 'active') LIMIT 1",
            &[&entity_id],
        ) {
            if !rows.is_empty() {
                contract_open = true;
                let subj: String = rows[0].get(0);
                let cp = subj.split('・').next().unwrap_or(subj.as_str());
                if !cp.is_empty() {
                    factors.counterparty_id = cp.to_string();
                }
                if let Some(cp_ent) = store.entities.get(cp)
                    && let (Some(x), Some(y)) = (cp_ent.grid_x, cp_ent.grid_y)
                {
                    obligation_deliver_coord = Some((x, y));
                }
            }
        }
    }

    // ── 需求決策：自主依房間標籤、距離代價、制服／內場辨識與預算尋客（下游交割） ──
    if (5..=8).contains(&game_hour)
        && (!contract_open
            || factors.counterparty_id.is_empty()
            || !store.entities.contains_key(&factors.counterparty_id))
    {
        if let Some((buyer_id, _buyer_place, buyer_coord)) = resolve_need_decision_buyer(
            store,
            entity_id,
            cur_xy,
            primary_out_id,
            &factors,
            &pw,
            &hooks,
            runtime_grid.as_ref(),
        ) {
            factors.counterparty_id = buyer_id;
            obligation_deliver_coord = Some(buyer_coord);
            contract_open = true;
        } else if factors.counterparty_id.is_empty() {
            factors.counterparty_id = format!("{}櫃檯", factors.deliver_bldg);
            obligation_deliver_coord = Some(factors.deliver_coord);
        }
    }

    if let Some(sup_ent) = store.entities.get(&factors.supplier_id)
        && let (Some(sx), Some(sy)) = (sup_ent.grid_x, sup_ent.grid_y)
    {
        factors.supplier_coord = Some((sx, sy));
    }
    if obligation_deliver_coord.is_none() {
        if let Some(cp_ent) = store.entities.get(&factors.counterparty_id)
            && let (Some(cx), Some(cy)) = (cp_ent.grid_x, cp_ent.grid_y)
        {
            obligation_deliver_coord = Some((cx, cy));
        } else {
            obligation_deliver_coord = Some(factors.deliver_coord);
        }
    }

    let chosen = choose_resident_life_action(
        &ent.display_title,
        &factors.settlement,
        seed_u64,
        ent.vit,
        ent.magnesium,
        grain_qty,
        ricemilk_qty,
        wear_points,
        contract_open,
        game_hour,
        weather,
        tide,
        repetitions,
        &hooks,
    );

    let (target_x, target_y) = match chosen {
        ResidentLifeActionChoice::RepairTool => factors.work_coord,
        ResidentLifeActionChoice::DeliverContract => obligation_deliver_coord.unwrap_or(factors.deliver_coord),
        ResidentLifeActionChoice::RestockGrain => factors.supplier_coord.unwrap_or(factors.work_coord),
        ResidentLifeActionChoice::FloodgateCheck => factors.floodgate_coord,
        ResidentLifeActionChoice::DineParley => factors.canteen_coord,
        ResidentLifeActionChoice::NightRest => factors.home_coord,
        ResidentLifeActionChoice::MillGrain => factors.work_coord,
    };

    let cur_pos = ent.grid_x.zip(ent.grid_y);
    let is_far = cur_pos
        .map(|(cx, cy)| (cx - target_x).abs() + (cy - target_y).abs() > 1)
        .unwrap_or(false);

    let dest_place = format!("grid:{target_x}:{target_y}");
    let work_bldg_plain = factors.work_bldg.replace('・', "");
    let deliver_bldg_plain = factors.deliver_bldg.replace('・', "");
    let floodgate_bldg_plain = factors.floodgate_bldg.replace('・', "");
    let canteen_bldg_plain = factors.canteen_bldg.replace('・', "");
    let home_bldg_plain = factors.home_bldg.replace('・', "");

    // ── 抵達閘門（Arrival Gate）：若尚未抵達場所，沿途推進且嚴禁隔空完成現場交割或工坊產出 ──
    if is_far {
        let (cx, cy) = cur_pos.unwrap_or((target_x, target_y));
        let (next_x, next_y) = crate::server::grid_manager::get_runtime_square_grid()
            .and_then(|g| crate::store::find_bfs_next_step(&g, (cx, cy), (target_x, target_y)))
            .unwrap_or((cx + (target_x - cx).signum(), cy + (target_y - cy).signum()));

        ent.grid_x = Some(next_x);
        ent.grid_y = Some(next_y);
        ent.target_x = if (next_x, next_y) == (target_x, target_y) { None } else { Some(target_x) };
        ent.target_y = if (next_x, next_y) == (target_x, target_y) { None } else { Some(target_y) };

        let dest_building = match chosen {
            ResidentLifeActionChoice::RepairTool => &factors.work_bldg,
            ResidentLifeActionChoice::DeliverContract => &factors.deliver_bldg,
            ResidentLifeActionChoice::RestockGrain => if factors.supplier_coord.is_some() {
                &factors.supplier_place_label
            } else {
                &factors.work_bldg
            },
            ResidentLifeActionChoice::FloodgateCheck => &factors.floodgate_bldg,
            ResidentLifeActionChoice::DineParley => &factors.canteen_bldg,
            ResidentLifeActionChoice::NightRest => &factors.home_bldg,
            ResidentLifeActionChoice::MillGrain => &factors.work_bldg,
        };

        let delivery_names: Vec<&str> = factors
            .delivery_items
            .iter()
            .map(|(_, name, _, _)| name.as_str())
            .collect();
        let delivery_label = if delivery_names.is_empty() {
            "工坊成品".to_string()
        } else {
            delivery_names.join("與")
        };

        let (in_transit_act, transit_thought) = match chosen {
            ResidentLifeActionChoice::DeliverContract => {
                let cp_call = if factors.counterparty_id.is_empty() {
                    format!("{}櫃檯", deliver_bldg_plain)
                } else {
                    factors.counterparty_id.clone()
                };
                (
                    format!(
                        "背負{}與{}，正沿山道前往{}交割早市合約",
                        factors.delivery_vessel_desc, delivery_label, deliver_bldg_plain
                    ),
                    format!(
                        "趁早將{}送達{}處，免得耽誤早市交割",
                        delivery_label, cp_call
                    ),
                )
            }
            ResidentLifeActionChoice::RestockGrain => {
                let sup_call = if factors.supplier_id.is_empty() {
                    format!("{}櫃檯", factors.supplier_place_label)
                } else if factors.supplier_id.starts_with(&factors.supplier_place_label) {
                    factors.supplier_id.clone()
                } else {
                    format!("{}{}", factors.supplier_place_label, factors.supplier_id)
                };
                let sup_target = if factors.supplier_id.is_empty() {
                    format!("{}櫃檯", factors.supplier_place_label)
                } else {
                    factors.supplier_id.clone()
                };
                (
                    format!("步向{}處商議補進{}", sup_call, factors.raw_item_name),
                    format!(
                        "工坊{}吃緊，得盡快找{}結算拉料入庫",
                        factors.raw_item_name, sup_target
                    ),
                )
            }
            ResidentLifeActionChoice::FloodgateCheck => (
                format!("沿{}步往{}巡看水勢", floodgate_bldg_plain, factors.floodgate_facility),
                format!("雨霧水位漸漲，得趕在{}受沖前落板減流", floodgate_bldg_plain),
            ),
            ResidentLifeActionChoice::DineParley => (
                format!("收工後沿渠畔步往{}用膳，與街坊商議明日引水與工務時刻", canteen_bldg_plain),
                format!("勞作大半日腹中空乏，先去{}喝碗熱湯歇腿，順道問問街坊近況", canteen_bldg_plain),
            ),
            ResidentLifeActionChoice::NightRest => (
                format!("巡看罷工坊門板，沿石徑緩步走回{}歇宿", home_bldg_plain),
                format!("今日工坊運轉平順，回{}安歇，養足精神明日早起", home_bldg_plain),
            ),
            ResidentLifeActionChoice::RepairTool => (
                format!("取下磨耗之{}，步往{}校準刃口", factors.tool_name, work_bldg_plain),
                format!("{}刃口起毛，活計吃不準分寸，得先去{}修整", factors.tool_name, factors.maintenance_station),
            ),
            ResidentLifeActionChoice::MillGrain => (
                format!("正沿山道返回{}料理手頭差事", work_bldg_plain),
                format!("返回{}開工料理{}，今日這批活計得做得細膩", work_bldg_plain, factors.raw_item_name),
            ),
        };

        ent.current_activity = in_transit_act.clone();
        let intent = "in_transit";
        let reason = format!("正沿路步往{}（{}, {}）", dest_building, target_x, target_y);
        crate::db::add_thought_to_entity_with_delta(&mut ent, crate::db::ThoughtSource::WorkDone, 4, now_unix, transit_thought);
        crate::db::clean_and_recalc_entity_disposition(&mut ent, now_unix);

        let ent_sync = ent.clone();
        let _ = store.upsert_entity_full(ent);
        let current_place = format!("grid:{next_x}:{next_y}");
        let _ = store.move_entity(entity_id, &current_place);
        crate::pg::entity::upsert_async(ent_sync);

        record_resident_continuity_row(entity_id, intent, &reason, &dest_place, now_unix);

        return (target_x, target_y, in_transit_act);
    }

    // ── 現場履行（On-Site Execution）：已抵達現場，全體角色共用單一實物、器物、體能、吐納與鎂幣守恆路徑 ──
    let reason: String;
    let activity: String;
    let thought_src: crate::db::ThoughtSource;
    let thought_text: String;
    let mut knowledge_update: Option<(String, &'static str, String)> = None;
    let intent: &str;

    match chosen {
        ResidentLifeActionChoice::RepairTool => {
            intent = "repair_tool";
            let honed = match sharpen_vessel_at_grindstone(entity_id, &factors.tool_id, now_unix) {
                Ok(rec) => rec,
                Err(_) => temper_vessel_at_forge(entity_id, &factors.tool_id, now_unix),
            };
            let habit = get_or_derive_breath_habit(entity_id, &ent.display_title, seed_u64 as i64).dominant;
            let _ = record_breath_activity(entity_id, habit, 2, now_unix);
            crate::store::clear_active_npc_obligation(entity_id, "maintain_tool");

            reason = format!(
                "於{}前以水礪法細細修整{}刃口，磨損歸零，刃口狀態自鈍刃重回「{}」（已運使 {} 次）",
                factors.maintenance_station,
                factors.tool_name,
                honed.condition.label(),
                honed.use_count,
            );
            activity = format!(
                "在{}前澆水細礪{}，用手指輕抹刃口檢視鋒芒分寸",
                factors.maintenance_station, factors.tool_name
            );
            thought_src = crate::db::ThoughtSource::WorkDone;
            thought_text = format!(
                "{}經{}修礪後重現鋒芒，磨損重歸於零（{}），隨時可再投產",
                factors.tool_name,
                factors.maintenance_station,
                honed.condition.label()
            );
            knowledge_update = Some((
                factors.work_bldg.clone(),
                "chisel_maintenance_state",
                format!(
                    "{}已於石台精礪完畢（狀態：{}，累計運使 {} 次），隨時可投產",
                    factors.tool_name,
                    honed.condition.label(),
                    honed.use_count
                ),
            ));
            // 甲的行為 -> 乙的事件：若甲在早市交割時段因重視器物鋒芒而選擇「先磨刀、延後送貨」，買方乙立即產生等待與應急事件
            if contract_open
                && (5..=8).contains(&game_hour)
                && let Some(mut cp) = store.get_entity(&factors.counterparty_id)
            {
                let wait_item = factors
                    .delivery_items
                    .first()
                    .map(|(_, n, _, _)| n.as_str())
                    .unwrap_or("早市物料");
                let cp_thought = format!(
                    "早市時分尚未見{}送來{}，想是工坊修整器物耽擱了片刻，先拿店裡存貨應急",
                    entity_id, wait_item
                );
                crate::db::add_thought_to_entity_with_delta(
                    &mut cp,
                    crate::db::ThoughtSource::WorkDone,
                    2,
                    now_unix,
                    cp_thought,
                );
                let cp_sync = cp.clone();
                let _ = store.upsert_entity_full(cp);
                crate::pg::entity::upsert_async(cp_sync);
            }
        }
        ResidentLifeActionChoice::DeliverContract => {
            intent = "deliver_contract";
            let mut delivered_parts = Vec::new();
            let mut remaining_parts = Vec::new();
            for (item_id, item_name, unit, qty) in &factors.delivery_items {
                let rem = inv_mutate_item(&mut ent.inventory, item_id, -(*qty), 12);
                delivered_parts.push(format!("{item_name} {qty} {unit}"));
                remaining_parts.push(format!("{item_name} {rem} {unit}"));
            }
            let delivered_str = delivered_parts.join("與");
            let remaining_str = remaining_parts.join("、");
            let mut payment_mg = 0;
            if let Some(mut cp) = store.get_entity(&factors.counterparty_id) {
                payment_mg = cp.magnesium.clamp(0, factors.delivery_payout_mg);
                cp.magnesium -= payment_mg;
                for (item_id, _, _, qty) in &factors.delivery_items {
                    inv_mutate_item(&mut cp.inventory, item_id, *qty, 12);
                }
                // 甲的行為 -> 乙的事件：買方乙收到實物交割後，立即寫入乙的思緒與雙向信任鉤子
                let cp_event = format!(
                    "收到{}送來的{}，當面付訖 {} 枚鎂幣，今日店裡備料寬裕不少",
                    entity_id, delivered_str, payment_mg
                );
                crate::db::add_thought_to_entity_with_delta(
                    &mut cp,
                    crate::db::ThoughtSource::Trade,
                    6,
                    now_unix,
                    cp_event,
                );
                crate::npc::causal_core::upsert_relational_hook(
                    &factors.counterparty_id,
                    entity_id,
                    entity_id,
                    "守諾交割",
                    format!("{}準時送達{}", entity_id, delivered_str),
                    12,
                    now_unix,
                );
                let cp_sync = cp.clone();
                let _ = store.upsert_entity_full(cp);
                crate::pg::entity::upsert_async(cp_sync);
            }
            ent.magnesium += payment_mg;
            ent.vit = (ent.vit - 5).max(20);
            crate::store::clear_active_npc_obligation(entity_id, "restock_goods");
            if let Some(pool) = crate::pg::pool()
                && let Ok(mut client) = pool.get()
            {
                let _ = client.execute(
                    "UPDATE resident_obligations SET status = 'fulfilled', updated_at = $2
                     WHERE entity_id = $1 AND obligation_kind = 'restock_goods'",
                    &[&entity_id, &now_unix],
                );
            }

            let cp_person = if factors.counterparty_id.is_empty() {
                format!("{}櫃檯", deliver_bldg_plain)
            } else {
                factors.counterparty_id.clone()
            };
            reason = format!(
                "依早市承諾背負{}赴{}（{}, {}），向{}實物交割{}，實收 {} 枚鎂幣",
                factors.delivery_vessel_desc,
                deliver_bldg_plain,
                factors.deliver_coord.0,
                factors.deliver_coord.1,
                cp_person,
                delivered_str,
                payment_mg
            );
            activity = format!(
                "在{}將{}內的{}點交予{}，當面結清早市貨款",
                deliver_bldg_plain,
                factors.delivery_vessel_desc,
                delivered_str,
                cp_person
            );
            thought_src = crate::db::ThoughtSource::Trade;
            thought_text = format!(
                "趁熱將{}送到{}交給{}，入帳 {} 枚鎂幣，手頭尚餘{}",
                delivered_str,
                deliver_bldg_plain,
                cp_person,
                payment_mg,
                remaining_str
            );
            knowledge_update = Some((
                format!("文曜・{}", cp_person),
                "morning_ricemilk_contract",
                format!(
                    "今日已親赴{}向{}交割{}（實收 {} 枚鎂幣，隨身尚存{}）",
                    deliver_bldg_plain,
                    cp_person,
                    delivered_str,
                    payment_mg,
                    remaining_str
                ),
            ));
        }
        ResidentLifeActionChoice::RestockGrain => {
            intent = "restock_grain";
            let cost_mg = ent.magnesium.min(factors.restock_cost_mg).max(0);
            ent.magnesium -= cost_mg;
            let buy_qty = factors.restock_qty;
            if let Some(mut sup) = store.get_entity(&factors.supplier_id) {
                let sup_stock = inv_count_item(&sup.inventory, &factors.raw_item_id);
                if sup_stock > 0 {
                    inv_mutate_item(&mut sup.inventory, &factors.raw_item_id, -buy_qty, 12);
                }
                sup.magnesium += cost_mg;
                // 甲的行為 -> 乙的事件：上游供應商賣出原料並收受鎂幣後，更新其思緒與雙向關係鉤子
                let sup_event = format!(
                    "將 {} {}{}點交予{}，收訖 {} 枚鎂幣",
                    buy_qty, factors.raw_item_unit, factors.raw_item_name, entity_id, cost_mg
                );
                crate::db::add_thought_to_entity_with_delta(
                    &mut sup,
                    crate::db::ThoughtSource::Trade,
                    5,
                    now_unix,
                    sup_event,
                );
                crate::npc::causal_core::upsert_relational_hook(
                    &factors.supplier_id,
                    entity_id,
                    entity_id,
                    "守諾供料",
                    format!("向{}供料{} {} {}", entity_id, buy_qty, factors.raw_item_unit, factors.raw_item_name),
                    10,
                    now_unix,
                );
                let sup_sync = sup.clone();
                let _ = store.upsert_entity_full(sup);
                crate::pg::entity::upsert_async(sup_sync);
            }
            let new_stock = inv_mutate_item(&mut ent.inventory, &factors.raw_item_id, buy_qty, 12);

            let resolved_sup_name = if factors.supplier_id.is_empty() {
                String::new()
            } else {
                let n = crate::store::resolve_npc_person_name(
                    &factors.supplier_id,
                    &factors.supplier_id,
                    None,
                );
                if n.is_empty() || n == "陌生人" || n.bytes().any(|b| b.is_ascii_alphabetic()) {
                    "供貨掌事".to_string()
                } else {
                    n
                }
            };
            let sup_desc = if resolved_sup_name.is_empty() {
                format!("{}櫃檯", factors.supplier_place_label)
            } else if resolved_sup_name.starts_with(&factors.supplier_place_label) {
                resolved_sup_name.clone()
            } else {
                format!("{}{}", factors.supplier_place_label, resolved_sup_name)
            };
            let sup_person = if resolved_sup_name.is_empty() {
                format!("{}櫃檯", factors.supplier_place_label)
            } else {
                resolved_sup_name
            };

            reason = format!(
                "工坊{}存量見底，向{}付訖 {} 枚鎂幣提領 {} {}{}入倉（現存 {} {}）",
                factors.raw_item_name,
                sup_desc,
                cost_mg,
                buy_qty,
                factors.raw_item_unit,
                factors.raw_item_name,
                new_stock,
                factors.raw_item_unit
            );
            activity = format!(
                "在{}畔向{}點收新運抵的 {} {}{}，當面結清鎂幣並扛入{}",
                work_bldg_plain,
                sup_desc,
                buy_qty,
                factors.raw_item_unit,
                factors.raw_item_name,
                work_bldg_plain
            );
            thought_src = crate::db::ThoughtSource::Trade;
            thought_text = format!(
                "向{}付了 {} 枚鎂幣補進 {} {}{}（現存 {} {}），{}今日不愁斷料",
                sup_desc,
                cost_mg,
                buy_qty,
                factors.raw_item_unit,
                factors.raw_item_name,
                new_stock,
                factors.raw_item_unit,
                work_bldg_plain
            );
            knowledge_update = Some((
                if factors.supplier_id.is_empty() {
                    format!("{}・櫃檯", factors.supplier_place_label)
                } else if factors.supplier_id.starts_with(&factors.supplier_place_label) {
                    factors.supplier_id.clone()
                } else {
                    format!("{}・{}", factors.supplier_place_label, factors.supplier_id)
                },
                "upstream_grain_supply",
                format!(
                    "因工坊{}存量見底，已向{}支付 {} 枚鎂幣補入 {} {}{}，目前隨身存量 {} {}",
                    factors.raw_item_name,
                    sup_person,
                    cost_mg,
                    buy_qty,
                    factors.raw_item_unit,
                    factors.raw_item_name,
                    new_stock,
                    factors.raw_item_unit
                ),
            ));
        }
        ResidentLifeActionChoice::FloodgateCheck => {
            intent = "floodgate_check";
            ent.vit = (ent.vit - 3).max(20);
            ent.qi = (ent.qi - 2).max(20);
            let habit = get_or_derive_breath_habit(entity_id, &ent.display_title, seed_u64 as i64).dominant;
            let _ = record_breath_activity(entity_id, habit, 2, now_unix);

            reason = format!(
                "觀測到北坡雨霧伴隨水脈漲潮，依水文經驗趕赴{}（{}, {}）落下{}",
                floodgate_bldg_plain,
                factors.floodgate_coord.0,
                factors.floodgate_coord.1,
                factors.floodgate_facility
            );
            activity = format!(
                "冒雨在{}落下{}",
                floodgate_bldg_plain, factors.floodgate_facility
            );
            thought_src = crate::db::ThoughtSource::WorkDone;
            thought_text = format!(
                "北坡雨霧水位漲起三寸，好在及時趕赴{}落下{}，保住了工坊器物與產出細度",
                floodgate_bldg_plain, factors.floodgate_facility
            );
            knowledge_update = Some((
                factors.floodgate_bldg.clone(),
                "hydrology_and_weather_omen",
                format!(
                    "北坡雨霧漲水之際已親赴{}落下{}（當前體力 {}、內息 {}），水勢恢復平穩",
                    floodgate_bldg_plain, factors.floodgate_facility, ent.vit, ent.qi
                ),
            ));
        }
        ResidentLifeActionChoice::DineParley => {
            intent = "dine_and_parley";
            if ent.magnesium >= 2 {
                crate::npc::causal_core::deposit_to_settlement_treasury(&factors.settlement, 2);
                ent.magnesium -= 2;
            } else if ent.magnesium > 0 {
                let paid = ent.magnesium;
                crate::npc::causal_core::deposit_to_settlement_treasury(&factors.settlement, paid);
                ent.magnesium = 0;
            }
            ent.vit = (ent.vit + 18).min(100);
            ent.qi = (ent.qi + 15).min(100);

            reason = format!(
                "勞作後赴{}（{}, {}）用膳回補體力（體力恢復至 {}），並與鄰里核對明日引水與工坊時刻",
                canteen_bldg_plain, factors.canteen_coord.0, factors.canteen_coord.1, ent.vit
            );
            activity = format!(
                "在{}喝著現磨米漿與熱麵湯，與北郊鄰里商議明日引水與工務時刻",
                canteen_bldg_plain
            );
            thought_src = crate::db::ThoughtSource::Ate;
            thought_text = format!(
                "在{}喝過熱米漿與麵湯，一身勞作的疲乏舒展開來，順道與鄰里敲定了明日工務時刻",
                canteen_bldg_plain
            );
        }
        ResidentLifeActionChoice::NightRest => {
            intent = "night_rest";
            ent.vit = (ent.vit + 25).min(100);
            ent.qi = (ent.qi + 25).min(100);
            let habit = get_or_derive_breath_habit(entity_id, &ent.display_title, seed_u64 as i64).dominant;
            let _ = record_breath_activity(entity_id, habit, 1, now_unix);

            reason = format!(
                "返抵{}（{}, {}）巡看門板插板後安歇，體力回補至 {}、內息回補至 {}",
                home_bldg_plain, factors.home_coord.0, factors.home_coord.1, ent.vit, ent.qi
            );
            activity = format!(
                "在{}檢查水門與門栓插板，伴著規律水聲歇息養神",
                home_bldg_plain
            );
            thought_src = crate::db::ThoughtSource::Rested;
            thought_text = format!(
                "檢查完{}門栓插板，聽著窗外規律水聲安穩養神",
                home_bldg_plain
            );
        }
        ResidentLifeActionChoice::MillGrain => {
            intent = "work";
            let rem_raw = inv_mutate_item(
                &mut ent.inventory,
                &factors.raw_item_id,
                -factors.raw_consume_qty,
                12,
            );
            let mut produced_parts = Vec::new();
            for (out_id, out_name, out_unit, out_qty) in &factors.outputs {
                let new_qty = inv_mutate_item(&mut ent.inventory, out_id, *out_qty, 12);
                produced_parts.push(format!("{out_name}（存 {new_qty} {out_unit}）"));
                crate::npc::causal_core::record_settlement_market_production(
                    &factors.settlement,
                    out_id,
                    (*out_qty).max(1),
                );
            }
            ent.vit = (ent.vit - 4).max(20);
            ent.qi = (ent.qi - 3).max(20);
            let v_primary = record_vessel_use(
                entity_id,
                &factors.tool_id,
                if factors.is_bladed { 1 } else { 0 },
                1,
                now_unix,
            );
            let v_secondary_use = if let Some(sec_id) = &factors.secondary_tool_id {
                Some(record_vessel_use(entity_id, sec_id, 0, 1, now_unix).use_count)
            } else {
                None
            };
            let habit = get_or_derive_breath_habit(entity_id, &ent.display_title, seed_u64 as i64).dominant;
            let _ = record_breath_activity(entity_id, habit, 2, now_unix);
            {
                let mut guard = ANATOMY_FAMILIARITY_CACHE.write().unwrap();
                let map = guard.entry(entity_id.to_string()).or_default();
                *map.entry(factors.anatomy_familiarity_subject.clone()).or_insert(36) += 1;
            }

            let produced_str = produced_parts.join("與");
            reason = format!(
                "於{}（{}, {}）消耗 {} {}{}（餘 {} {}）產出{}，{}磨損 {}/3（{}）",
                work_bldg_plain,
                factors.work_coord.0,
                factors.work_coord.1,
                factors.raw_consume_qty,
                factors.raw_item_unit,
                factors.raw_item_name,
                rem_raw,
                factors.raw_item_unit,
                produced_str,
                factors.tool_name,
                v_primary.wear_points,
                v_primary.condition.label(),
            );
            activity = factors.work_activity_desc.clone();
            thought_src = crate::db::ThoughtSource::WorkDone;
            thought_text = format!(
                "在{}將 {} {}{}化作{}，手中{}已運使 {} 次",
                work_bldg_plain,
                factors.raw_consume_qty,
                factors.raw_item_unit,
                factors.raw_item_name,
                produced_str,
                factors.tool_name,
                v_primary.use_count
            );
            let sec_note = match (&factors.secondary_tool_name, v_secondary_use) {
                (Some(sname), Some(scount)) => format!("{}已運使 {} 次，", sname, scount),
                _ => String::new(),
            };
            knowledge_update = Some((
                factors.work_bldg.clone(),
                "millstone_and_sluice_state",
                format!(
                    "今日已耗{}產出{}（待用{}餘 {} {}）；{}{}已運使 {} 次（磨損點數 {}・{}）",
                    factors.raw_item_name,
                    produced_str,
                    factors.raw_item_name,
                    rem_raw,
                    factors.raw_item_unit,
                    sec_note,
                    factors.tool_name,
                    v_primary.use_count,
                    v_primary.wear_points,
                    v_primary.condition.label(),
                ),
            ));
        }
    }

    let (next_x, next_y) = (target_x, target_y);
    ent.grid_x = Some(next_x);
    ent.grid_y = Some(next_y);
    ent.target_x = None;
    ent.target_y = None;
    ent.current_activity = activity.clone();
    ent.thoughts.retain(|t| {
        let f = t.flavor.as_deref().unwrap_or("");
        !f.contains("鎂袋空了")
            && !f.contains("今日花用又見底")
            && !f.contains("記掛著「內室榻上熄燈安歇」")
            && !f.contains("記掛著『診所抓藥』")
            && !f.contains("記掛著『手頭差事』")
    });
    crate::db::add_thought_to_entity_with_delta(&mut ent, thought_src, 8, now_unix, thought_text);
    crate::db::clean_and_recalc_entity_disposition(&mut ent, now_unix);

    let ent_sync = ent.clone();
    let _ = store.upsert_entity_full(ent);
    let current_place = format!("grid:{next_x}:{next_y}");
    let _ = store.move_entity(entity_id, &current_place);
    crate::pg::entity::upsert_async(ent_sync);

    record_resident_continuity_row(entity_id, intent, &reason, &dest_place, now_unix);
    if let Some((subj, pred, val)) = knowledge_update {
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute(
                "INSERT INTO resident_knowledge (entity_id, subject_ref, predicate, value, certainty, source_ref, learned_at, updated_at)
                 VALUES ($1, $2, $3, $4, 98, 'four_question_loop', $5, $5)
                 ON CONFLICT (entity_id, subject_ref, predicate) DO UPDATE SET
                   value = EXCLUDED.value,
                   source_ref = EXCLUDED.source_ref,
                   updated_at = EXCLUDED.updated_at",
                &[&entity_id, &subj, &pred, &val, &now_unix],
            );
        }
    }

    (target_x, target_y, activity)
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
        BREATH_HABIT_CACHE.write().unwrap().remove("test_ent_habit");
        VESSEL_STATE_CACHE.write().unwrap().remove(&("test_ent_vessel".to_string(), "wt_boning_knife".to_string()));
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM character_breath_habit WHERE entity_id = 'test_ent_habit'", &[]);
            let _ = client.execute("DELETE FROM character_vessel_mastery WHERE entity_id = 'test_ent_vessel'", &[]);
        }
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
        let pid = "player_e2e_hunter_living_loop";
        let now = 1_760_000_000i64;
        BREATH_HABIT_CACHE.write().unwrap().remove(pid);
        VESSEL_STATE_CACHE.write().unwrap().remove(&(pid.to_string(), "craft_steel_hunting_knife".to_string()));
        LINEAGE_IMPRINT_CACHE.write().unwrap().remove(pid);
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute("DELETE FROM character_breath_habit WHERE entity_id = $1", &[&pid]);
            let _ = client.execute("DELETE FROM character_vessel_mastery WHERE entity_id = $1", &[&pid]);
            let _ = client.execute("DELETE FROM character_lineage_imprints WHERE entity_id = $1", &[&pid]);
        }

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

        // 6. 百工高光分層 LLM 資料轉描述與事實保真護欄
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

    #[test]
    fn flagship_specimen_1_qin_shuisheng_eight_dimensions_coherent() {
        let now = 1_793_632_000i64;
        {
            let mut guard = VESSEL_STATE_CACHE.write().unwrap();
            let s = guard.entry(("秦水生".to_string(), "wt_cat_0973".to_string())).or_insert_with(|| VesselStateRecord {
                entity_id: "秦水生".to_string(),
                item_id: "wt_cat_0973".to_string(),
                use_count: 68,
                wear_points: 0,
                condition: VesselCondition::Sharp,
                updated_at: now,
            });
            s.wear_points = 0;
            s.condition = VesselCondition::Sharp;
            let c = guard.entry(("秦水生".to_string(), "wt_cat_0984".to_string())).or_insert_with(|| VesselStateRecord {
                entity_id: "秦水生".to_string(),
                item_id: "wt_cat_0984".to_string(),
                use_count: 42,
                wear_points: 0,
                condition: VesselCondition::Sharp,
                updated_at: now,
            });
            c.wear_points = 0;
            c.condition = VesselCondition::Sharp;
        }
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute(
                "UPDATE character_vessel_mastery SET wear_points = 0, condition_code = 'sharp'
                 WHERE entity_id = '秦水生' AND item_id IN ('wt_cat_0973', 'wt_cat_0984')",
                &[],
            );
            let _ = client.execute(
                "UPDATE resident_obligations SET status = 'active'
                 WHERE obligation_id = 'ob_qin_shuisheng_morning_ricemilk_wenyao'",
                &[],
            );
        }
        seed_flagship_specimen_qin_shuisheng_once(now);

        let manuals = get_entity_studied_manuals("秦水生");
        assert!(manuals.iter().any(|m| m == "堰渠水輪分動考工譜"));
        assert!(manuals.iter().any(|m| m == "石磨齒距微調與水磨澄漿手札"));

        let v_sieve = get_vessel_state("秦水生", "wt_cat_0973");
        assert!(v_sieve.use_count >= 68);
        assert_eq!(v_sieve.condition, VesselCondition::Sharp);
        assert_eq!(v_sieve.familiarity(), VesselFamiliarity::LikeOwnArm);

        // 驗證羊角鐵錘打石手鏨（wt_cat_0984）真正走玩家同構磨損與活水磨石推磨修復閉環（而非整點鎖血歸零）
        let v_chisel_0 = get_vessel_state("秦水生", "wt_cat_0984");
        assert!(v_chisel_0.use_count >= 42);
        let v_worn = record_vessel_use("秦水生", "wt_cat_0984", 3, 3, now + 10);
        assert!(v_worn.wear_points >= 3);
        assert_eq!(v_worn.condition, VesselCondition::SlightlyDull);
        // 再次呼叫 seed_flagship_specimen_qin_shuisheng_once 絕不會把 wear_points 洗回 0
        seed_flagship_specimen_qin_shuisheng_once(now + 20);
        assert!(get_vessel_state("秦水生", "wt_cat_0984").wear_points >= 3);
        // 透過玩家同構之 sharpen_vessel_at_grindstone 推磨後，wear_points 歸零、重回 Sharp
        let v_honed = sharpen_vessel_at_grindstone("秦水生", "wt_cat_0984", now + 30).unwrap();
        assert_eq!(v_honed.wear_points, 0);
        assert_eq!(v_honed.condition, VesselCondition::Sharp);

        let fam_total = get_entity_anatomy_familiarity_total("秦水生");
        assert!(fam_total >= 64);

        let equip_json = r#"{"head":"wt_cat_0875","body":"wt_cat_0869","legs":"starter_legs_m","feet":"starter_feet_m","wrist":"wt_cat_0852","waist":"wt_cat_0897","hold_r":"wt_cat_0984","hold_l":"wt_cat_0973"}"#;
        assert!(!crate::db::is_naked(equip_json));
        let names = crate::db::get_item_names(equip_json);
        assert_eq!(names.get("hold_r").map(String::as_str), Some("羊角鐵錘打石手鏨"));
        assert_eq!(names.get("hold_l").map(String::as_str), Some("平底石磨分粉篩架"));
        assert_eq!(names.get("waist").map(String::as_str), Some("薄層重礦烏木膽隨身保溫瓶"));
        assert_eq!(names.get("body").map(String::as_str), Some("粗麻雙帶圍胸短褂"));

        // 驗證 Store 層四問決策閉環：耗糧碾磨 -> 累積手鏨磨損 -> 自動推磨修復 -> 斷糧向周萬生守恆買米
        let mut store = crate::store::Store::new_in_memory();
        let mut qin_ent = crate::store::Entity::new_npc("秦水生", "水");
        qin_ent.display_title = "老堰頭水輪石磨首座|秦水生".to_string();
        qin_ent.grid_x = Some(-3);
        qin_ent.grid_y = Some(125);
        qin_ent.vit = 90;
        qin_ent.qi = 90;
        qin_ent.magnesium = 50;
        qin_ent.inventory = r#"[{"item_id":"wt_cat_0388","qty":4},{"item_id":"wt_cat_0279","qty":4},{"item_id":"wt-good-009","qty":2}]"#.to_string();
        let _ = store.upsert_entity_full(qin_ent);

        let mut zhou_ent = crate::store::Entity::new_npc("周萬生", "周");
        zhou_ent.magnesium = 30;
        zhou_ent.inventory = r#"[{"item_id":"wt-good-009","qty":12}]"#.to_string();
        let _ = store.upsert_entity_full(zhou_ent);

        // Step A：耗 1 袋精碾白米（2 -> 1），產出溫米漿（4 -> 5）與在來米粉（4 -> 5），手鏨 wear_points 0 -> 1
        let (x1, y1, act1) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 10, 1_800_000_000);
        assert_eq!((x1, y1), (-3, 125));
        assert!(act1.contains("大石磨"));
        let qin_after_1 = store.get_entity("秦水生").unwrap();
        assert_eq!(inv_count_item(&qin_after_1.inventory, "wt-good-009"), 1);
        assert_eq!(inv_count_item(&qin_after_1.inventory, "wt_cat_0388"), 5);
        assert_eq!(get_vessel_state("秦水生", "wt_cat_0984").wear_points, 1);

        // Step B：因為 wt-good-009 只剩 1 袋（< 2），下一輪自動觸發斷糧打斷，向周萬生支付 12 Mg 補入 6 袋新米（1 -> 7）
        let (_, _, act2) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 11, 1_800_000_060);
        assert!(act2.contains("周萬生"));
        let qin_after_2 = store.get_entity("秦水生").unwrap();
        let zhou_after_2 = store.get_entity("周萬生").unwrap();
        assert_eq!(inv_count_item(&zhou_after_2.inventory, "wt-good-009"), 6);
        assert_eq!(inv_count_item(&qin_after_2.inventory, "wt-good-009"), 7);
        assert_eq!(qin_after_2.magnesium, 38);
        assert_eq!(zhou_after_2.magnesium, 42);
        assert_eq!(qin_after_2.magnesium + zhou_after_2.magnesium, 80); // 雙向鎂幣守恆

        // Step C：再連續磨米兩次讓手鏨 wear_points 達到 3（SlightlyDull），下一步自動打斷磨米、在活水磨石推磨歸零
        let _ = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 12, 1_800_000_120);
        let _ = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 13, 1_800_000_180);
        assert_eq!(get_vessel_state("秦水生", "wt_cat_0984").wear_points, 3);
        let (_, _, act_hone) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 14, 1_800_000_240);
        assert!(act_hone.contains("活水磨石"));
        assert_eq!(get_vessel_state("秦水生", "wt_cat_0984").wear_points, 0);
        assert_eq!(get_vessel_state("秦水生", "wt_cat_0984").condition, VesselCondition::Sharp);

        // Step D：驗證送交早市米漿（hour = 6）與陳萬福交易。
        // 先在遠處時應為「沿山道前往」，不瞬移亦不交割；抵達 (-4, 4) 後才實物交割與鎂幣轉移
        let mut chen_ent = crate::store::Entity::new_npc("陳萬福", "陳");
        chen_ent.magnesium = 20;
        let _ = store.upsert_entity_full(chen_ent);
        // 遠處：目前位於 (-3, 125)，往 (-4, 4) 移動一步
        let (x_far, y_far, act_far) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 6, 1_800_000_300);
        assert_eq!((x_far, y_far), (-4, 4));
        let qin_far = store.get_entity("秦水生").unwrap();
        assert_eq!((qin_far.grid_x, qin_far.grid_y), (Some(-4), Some(124))); // step 1 cell
        assert!(act_far.contains("沿山道前往"));
        // 手動將秦水生置於 (-4, 4) 模擬抵達大食堂
        let _ = store.update_entity("秦水生", |e| {
            e.grid_x = Some(-4);
            e.grid_y = Some(4);
        });
        let (_, _, act_near) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 6, 1_800_000_360);
        assert!(act_near.contains("文曜三水大食堂"));
        let qin_delivered = store.get_entity("秦水生").unwrap();
        let chen_received = store.get_entity("陳萬福").unwrap();
        assert_eq!(inv_count_item(&chen_received.inventory, "wt_cat_0388"), 2);
        assert_eq!(inv_count_item(&chen_received.inventory, "wt_cat_0279"), 1);
        assert_eq!(chen_received.magnesium, 4); // 20 - 16 = 4
        assert_eq!(qin_delivered.magnesium, 38 + 16); // 38 + 16 = 54

        // 驗證當秦水生仍在文曜三水大食堂（-4, 4），且進入日間常規磨米時段（hour = 10，目標為老堰頭 -3, 125）時，
        // 絕不在文曜遠端隔空耗糧碾磨，而是處於「正沿山道返回老堰頭水輪石磨工坊」路途步態，待回到工坊後才執行實體磨務
        let qin_before_return = store.get_entity("秦水生").unwrap();
        let grain_before = inv_count_item(&qin_before_return.inventory, "wt-good-009");
        let ricemilk_before = inv_count_item(&qin_before_return.inventory, "wt_cat_0388");
        let (x_ret, y_ret, act_ret) = step_flagship_specimen_qin_shuisheng_life_loop(&mut store, 10, 1_800_000_420);
        assert_eq!((x_ret, y_ret), (-3, 125));
        assert!(act_ret.contains("返回老堰頭水輪石磨工坊"));
        let qin_walking = store.get_entity("秦水生").unwrap();
        assert_eq!(inv_count_item(&qin_walking.inventory, "wt-good-009"), grain_before);
        assert_eq!(inv_count_item(&qin_walking.inventory, "wt_cat_0388"), ricemilk_before);
        // Step E：驗證「萬人萬相・同境不同選」——在完全相同的外部處境（wear_points == 3 且早市交貨時間已到 hour == 6）下，
        // 依不同 soul_seed 與所熟歷練，有人選擇「先磨利手鏨再出門（RepairTool）」，有人選擇「先趕送早市米漿履約（DeliverContract）」，非死板單一機器人。
        let mut saw_repair = false;
        let mut saw_deliver = false;
        for candidate_seed in 1u64..120u64 {
            let choice = choose_resident_life_action(
                "老堰頭水輪石磨首座",
                "老堰頭",
                candidate_seed.wrapping_mul(0x9E37_79B9_7F4A_7C15),
                85,
                30,
                4,
                4,
                3,
                true,
                6,
                crate::game::ZoneWeatherPhase::ClearBreeze,
                crate::game::ZoneTidePhase::SlackTide,
                68,
                &[],
            );
            if choice == ResidentLifeActionChoice::RepairTool {
                saw_repair = true;
            } else if choice == ResidentLifeActionChoice::DeliverContract {
                saw_deliver = true;
            }
        }
        assert!(
            saw_repair && saw_deliver,
            "Identical situation (wear=3, morning delivery due) must yield both RepairTool and DeliverContract across different soul_seeds"
        );
    }

    #[test]
    fn canonical_resident_life_loop_is_shared_and_factor_driven_across_vocations() {
        let now = 1_800_100_000i64;
        let smith_id = "岑守岳_test_shared_loop";
        {
            let mut guard = VESSEL_STATE_CACHE.write().unwrap();
            guard.insert(
                (smith_id.to_string(), "wt_blacksmith_hammer".to_string()),
                VesselStateRecord {
                    entity_id: smith_id.to_string(),
                    item_id: "wt_blacksmith_hammer".to_string(),
                    use_count: 55,
                    wear_points: 0,
                    condition: VesselCondition::Sharp,
                    updated_at: now,
                },
            );
        }

        let mut store = crate::store::Store::new_in_memory();
        let mut smith_ent = crate::store::Entity::new_npc(smith_id, "岑");
        smith_ent.display_title = "文曜鐵坊掌錘|岑守岳".to_string();
        smith_ent.grid_x = Some(6);
        smith_ent.grid_y = Some(4);
        smith_ent.vit = 90;
        smith_ent.qi = 90;
        smith_ent.magnesium = 40;
        smith_ent.equipment_slots = r#"{"hold_r":"wt_blacksmith_hammer","body":"starter_chest_m"}"#.to_string();
        smith_ent.inventory = r#"[{"item_id":"wt_cat_0387","qty":4},{"item_id":"wt-good-019","qty":2}]"#.to_string();
        let _ = store.upsert_entity_full(smith_ent);

        let mut zhou_ent = crate::store::Entity::new_npc("周萬生", "周");
        zhou_ent.magnesium = 30;
        zhou_ent.inventory = r#"[{"item_id":"wt-good-019","qty":10}]"#.to_string();
        let _ = store.upsert_entity_full(zhou_ent);

        let mut chen_ent = crate::store::Entity::new_npc("陳萬福", "陳");
        chen_ent.magnesium = 25;
        let _ = store.upsert_entity_full(chen_ent);

        // 1. Work (MillGrain choice): 消耗 1 簍赤鐵粗礦（2 -> 1），產出百鍛農具鐵件（4 -> 5），鐵錘磨損 0 -> 1
        let (wx, wy, w_act) = step_canonical_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            10,
            now,
        );
        assert_eq!((wx, wy), (6, 4));
        assert!(w_act.contains("百鍛鐵工坊"));
        let smith_after_work = store.get_entity(smith_id).unwrap();
        assert_eq!(inv_count_item(&smith_after_work.inventory, "wt-good-019"), 1);
        assert_eq!(inv_count_item(&smith_after_work.inventory, "wt_cat_0387"), 5);
        assert_eq!(get_vessel_state(smith_id, "wt_blacksmith_hammer").wear_points, 1);

        // 2. Restock: 因為赤鐵粗礦只剩 1 簍（< 2），下一輪自動向周萬生支付 12 Mg 補入 6 簍赤鐵粗礦（1 -> 7）
        let (_, _, r_act) = step_canonical_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            11,
            now + 60,
        );
        assert!(r_act.contains("周萬生") && r_act.contains("赤鐵粗礦"));
        let smith_after_restock = store.get_entity(smith_id).unwrap();
        let zhou_after_restock = store.get_entity("周萬生").unwrap();
        assert_eq!(inv_count_item(&smith_after_restock.inventory, "wt-good-019"), 7);
        assert_eq!(inv_count_item(&zhou_after_restock.inventory, "wt-good-019"), 4);
        assert_eq!(smith_after_restock.magnesium, 28);
        assert_eq!(zhou_after_restock.magnesium, 42);

        // 3. RepairTool: 再鍛打兩次使鐵錘磨損達 3，下一輪自動在活水磨石台推磨歸零
        let _ = step_canonical_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            12,
            now + 120,
        );
        let _ = step_canonical_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            13,
            now + 180,
        );
        assert_eq!(get_vessel_state(smith_id, "wt_blacksmith_hammer").wear_points, 3);
        let (_, _, h_act) = step_canonical_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            14,
            now + 240,
        );
        assert!(h_act.contains("活水磨石"));
        assert_eq!(get_vessel_state(smith_id, "wt_blacksmith_hammer").wear_points, 0);

        // 4. DeliverContract: 早市（hour = 6）先沿途移動（不隔空交割），抵達 (-4, 4) 後將 2 件百鍛農具鐵件交割予陳萬福並收受 16 Mg
        let (_, _, d_far_act) = step_canonical_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            6,
            now + 300,
        );
        assert!(d_far_act.contains("沿山道前往"));
        let _ = store.update_entity(smith_id, |e| {
            e.grid_x = Some(-4);
            e.grid_y = Some(4);
        });
        let (_, _, d_near_act) = step_canonical_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            6,
            now + 360,
        );
        assert!(d_near_act.contains("文曜三水大食堂") && d_near_act.contains("百鍛農具鐵件"));
        let smith_delivered = store.get_entity(smith_id).unwrap();
        let chen_received = store.get_entity("陳萬福").unwrap();
        assert_eq!(inv_count_item(&chen_received.inventory, "wt_cat_0387"), 2);
        assert_eq!(chen_received.magnesium, 9); // 25 - 16
        assert_eq!(smith_delivered.magnesium, 28 + 16); // 44
    }

    #[test]
    fn restock_grain_steps_toward_supplier_when_supplier_has_grid_coord() {
        let mut store = crate::store::Store::new_in_memory();
        let smith_id = "岑守岳_test_restock_step";
        let now = 1_800_000_000i64;

        let mut smith_ent = crate::store::Entity::new_npc(smith_id, "岑");
        smith_ent.display_title = "文曜百鍛鐵匠|岑守岳".to_string();
        smith_ent.grid_x = Some(6);
        smith_ent.grid_y = Some(4);
        smith_ent.vit = 80;
        smith_ent.magnesium = 40;
        smith_ent.equipment_slots = r#"{"hold_r":"wt_blacksmith_hammer","body":"starter_chest_m"}"#.to_string();
        smith_ent.inventory = r#"[{"item_id":"wt_cat_0387","qty":4},{"item_id":"wt-good-019","qty":1}]"#.to_string();
        let _ = store.upsert_entity_full(smith_ent);

        let mut zhou_ent = crate::store::Entity::new_npc("周萬生", "周");
        zhou_ent.grid_x = Some(-2);
        zhou_ent.grid_y = Some(-4);
        zhou_ent.magnesium = 30;
        zhou_ent.inventory = r#"[{"item_id":"wt-good-019","qty":10}]"#.to_string();
        let _ = store.upsert_entity_full(zhou_ent);

        // 1. 原料僅剩 1 簍（< 2）且周萬生具有實體座標 (-2, -4) 時，
        // 觸發 RestockGrain，應沿途移動走向 (-2, -4)，在途嚴禁隔空扣款提料
        let (tx, ty, transit_act) = step_canonical_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            11,
            now,
        );
        assert_eq!((tx, ty), (-2, -4));
        assert!(transit_act.contains("步向大圳頭周萬生處商議補進赤鐵粗礦"));

        // 驗證沿途移動中嚴禁空中扣減鎂幣或預先入庫
        let smith_mid = store.get_entity(smith_id).unwrap();
        let zhou_mid = store.get_entity("周萬生").unwrap();
        assert_eq!(inv_count_item(&smith_mid.inventory, "wt-good-019"), 1);
        assert_eq!(inv_count_item(&zhou_mid.inventory, "wt-good-019"), 10);
        assert_eq!(smith_mid.magnesium, 40);
        assert_eq!(zhou_mid.magnesium, 30);

        // 2. 抵達周萬生座標 (-2, -4) 後，下一輪執行現場交割與雙向鎂幣轉移
        let _ = store.update_entity(smith_id, |e| {
            e.grid_x = Some(-2);
            e.grid_y = Some(-4);
        });
        let (_, _, onsite_act) = step_canonical_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            11,
            now + 60,
        );
        assert!(onsite_act.contains("周萬生") && onsite_act.contains("赤鐵粗礦"));

        let smith_final = store.get_entity(smith_id).unwrap();
        let zhou_final = store.get_entity("周萬生").unwrap();
        assert_eq!(inv_count_item(&smith_final.inventory, "wt-good-019"), 7);
        assert_eq!(inv_count_item(&zhou_final.inventory, "wt-good-019"), 4);
        assert_eq!(smith_final.magnesium, 28);
        assert_eq!(zhou_final.magnesium, 42);
        assert_eq!(smith_final.magnesium + zhou_final.magnesium, 70);
    }

    #[test]
    fn autonomous_choice_diverges_by_personality_and_triggers_dyadic_events() {
        let now = 1_800_300_000i64;
        // 1. 同境不同選：當手頭器物磨損 wear_points == 3 且恰逢早市契約（hour = 6, ricemilk_ob_open = true）：
        //    - 高匠心／守序角色（如秦水生 seed）拒絕帶著鈍鏨出工，選擇先 RepairTool；
        //    - 重信諾且累積深厚信任鉤子的角色，選擇先 DeliverContract 履約，回來再磨刀。
        let craft_choice = choose_resident_life_action(
            "老堰頭水輪石磨首座|秦水生",
            "老堰頭",
            4_088_606_620_229_995_941,
            85,
            30,
            6,
            6,
            3,
            true,
            6,
            crate::game::ZoneWeatherPhase::ClearBreeze,
            crate::game::ZoneTidePhase::SlackTide,
            120,
            &[],
        );
        assert_eq!(craft_choice, ResidentLifeActionChoice::RepairTool);

        let trust_hooks = vec![crate::npc::causal_core::RelationalHook {
            target_id: "陳萬福".to_string(),
            target_name: "陳萬福".to_string(),
            kind_label: "守諾交割".to_string(),
            summary: "長年守信交割".to_string(),
            intensity: 45,
            updated_at_unix: now,
        }];
        let honor_choice = choose_resident_life_action(
            "古道行商|馬山行",
            "文曜",
            7,
            85,
            4,
            6,
            6,
            3,
            true,
            6,
            crate::game::ZoneWeatherPhase::ClearBreeze,
            crate::game::ZoneTidePhase::SlackTide,
            5,
            &trust_hooks,
        );
        assert_eq!(honor_choice, ResidentLifeActionChoice::DeliverContract);

        // 2. 自主尋商（傲骨拒向仇家買料）＋ 甲的行為變成乙的事件：
        //    鐵匠缺赤鐵粗礦（qty = 1），場上有兩個擁有赤鐵粗礦的賣家：
        //    - 「惡霸礦頭」（距離近，但有嫌隙結怨鉤子）
        //    - 「石萬鈞」（有信任關係）
        //    鐵匠自主避開仇家，選擇向石萬鈞採買；採買完成後，石萬鈞身上立即產生對應交易思緒與關係鉤子。
        let smith_id = "岑守岳_test_autonomy";
        crate::npc::causal_core::upsert_relational_hook(
            smith_id,
            "惡霸礦頭",
            "惡霸礦頭",
            "結怨嫌隙",
            "曾以次充好刁難礦價".to_string(),
            35,
            now,
        );
        let mut store = crate::store::Store::new_in_memory();
        let mut smith = crate::store::Entity::new_npc(smith_id, "岑");
        smith.display_title = "文曜鐵坊掌錘|岑守岳".to_string();
        smith.grid_x = Some(6);
        smith.grid_y = Some(4);
        smith.magnesium = 36;
        smith.equipment_slots = r#"{"hold_r":"wt_blacksmith_hammer"}"#.to_string();
        smith.inventory = r#"[{"item_id":"wt_cat_0387","qty":4},{"item_id":"wt-good-019","qty":1}]"#.to_string();
        let _ = store.upsert_entity_full(smith);

        let mut bully = crate::store::Entity::new_npc("惡霸礦頭", "惡");
        bully.magnesium = 50;
        bully.inventory = r#"[{"item_id":"wt-good-019","qty":10}]"#.to_string();
        let _ = store.upsert_entity_full(bully);

        let mut shi = crate::store::Entity::new_npc("石萬鈞", "石");
        shi.magnesium = 20;
        shi.inventory = r#"[{"item_id":"wt-good-019","qty":10}]"#.to_string();
        let _ = store.upsert_entity_full(shi);

        let (_, _, act) = step_canonical_resident_human_life_loop(
            &mut store,
            smith_id,
            6,
            4,
            "文曜・百鍛鐵工坊",
            "在百鍛鐵工坊揮錘鍛打赤鐵粗礦",
            11,
            now + 60,
        );
        assert!(act.contains("石萬鈞"), "expected autonomous choice of 石萬鈞 over grudge supplier: {act}");
        assert!(!act.contains("惡霸礦頭"));
        let shi_after = store.get_entity("石萬鈞").unwrap();
        assert_eq!(shi_after.magnesium, 32);
        assert!(
            shi_after.thoughts.iter().any(|t| t.flavor.as_deref().unwrap_or("").contains("赤鐵粗礦")),
            "expected supplier 石萬鈞 to receive dyadic event thought from buyer's action"
        );
    }
    #[test]
    fn test_need_decision_spatial_tag_and_uniform_counter_discovery() {
        use crate::grid::{GridCell, SquareCoord, SquareGrid, Terrain};
        use crate::store::Entity;

        let forbidden_names = [
            "周萬生", "陳萬福", "石萬鈞", "梁千木", "葉青囊", "馬山行",
            "顧海平", "蘇錦心", "田百稼", "柯成器", "溫懷瑾", "羅烈",
            "許海音", "沈清河",
        ];

        // 1. 驗證 ResidentIntrinsicFactors::resolve 在 Store 無任何實體時，
        //    產生的 supplier_id 與 counterparty_id 完全為空，絕不包含任何寫死正典 NPC 姓名
        let test_npc_definitions = [
            ("秦水生", "水", "老堰頭・水輪石磨工坊", "碾米"),
            ("岑守岳", "岑", "文曜・百鍛鐵工坊", "鍛打鐵器"),
            ("柯成器", "柯", "文曜・巧木居所", "刨削木料"),
            ("葉青囊", "葉", "文曜・百草診所", "分揀草藥"),
            ("馬山行", "馬", "文曜・皮革行", "熟化鹿皮"),
            ("顧海平", "顧", "文曜・水產行", "醃漬鮮魚"),
            ("蘇錦心", "蘇", "文曜・雲錦織造", "裁剪布料"),
        ];

        for (id, char_sym, bldg, act) in test_npc_definitions {
            let ent = Entity::new_npc(id, char_sym);
            let factors = ResidentIntrinsicFactors::resolve(&ent, 0, 0, bldg, act);

            assert!(
                factors.supplier_id.is_empty(),
                "factors.supplier_id for {id} must be empty by default, got: {}",
                factors.supplier_id
            );
            assert!(
                factors.counterparty_id.is_empty(),
                "factors.counterparty_id for {id} must be empty by default, got: {}",
                factors.counterparty_id
            );

            for forbidden in &forbidden_names {
                assert!(
                    !factors.supplier_id.contains(forbidden),
                    "factors.supplier_id for {id} contains hardcoded name: {forbidden}"
                );
                assert!(
                    !factors.counterparty_id.contains(forbidden),
                    "factors.counterparty_id for {id} contains hardcoded name: {forbidden}"
                );
            }
            assert!(!factors.need_supply_tag.is_empty());
            assert!(!factors.need_buyer_tag.is_empty());
        }

        // 2. 驗證空間距離加權：沙鹿巷口便利小鋪（距離 2） vs concat!("竹北高", "鐵便利大行")（距離 200）
        //    無論竹北物資多豐厚，角色絕不從沙鹿遠遁竹北尋小七，必定向就近之沙鹿巷口小七補料
        let mut grid = SquareGrid::new();
        let near_coord = SquareCoord::new(2, 0);
        let mut near_room = GridCell::new(near_coord, Terrain::GeneralStore, "沙鹿巷口便利小鋪");
        near_room.tags.push("穀物".to_string());
        grid.insert(near_room);

        let far_coord = SquareCoord::new(200, 0);
        let mut far_room = GridCell::new(far_coord, Terrain::GeneralStore, concat!("竹北高", "鐵便利大行"));
        far_room.tags.push("穀物".to_string());
        grid.insert(far_room);

        let store = crate::store::Store::new_in_memory();
        let qin_ent = Entity::new_npc("秦水生", "水");
        let qin_factors = ResidentIntrinsicFactors::resolve(&qin_ent, 0, 0, "老堰頭・水輪石磨工坊", "碾米");
        let pw = crate::npc::causal_core::PluralPersonalityWeights::default();

        let chosen_supplier = resolve_need_decision_supplier(
            &store,
            "秦水生",
            (0, 0), // 當前座標 (0, 0)
            &qin_factors,
            &pw,
            &[],
            Some(&grid),
        );

        assert!(chosen_supplier.is_some(), "expected to find a matching room supplier");
        let (sup_id, sup_place, sup_coord) = chosen_supplier.unwrap();
        assert_eq!(
            sup_coord, (2, 0),
            "character must choose near room at (2,0) over faraway room at (200,0)"
        );
        assert!(
            sup_id.contains("沙鹿") || sup_place.contains("沙鹿"),
            "supplier must be the nearby 沙鹿 room, got id: {sup_id}, place: {sup_place}"
        );
        assert!(!sup_id.contains("竹北") && !sup_place.contains("竹北"));

        // 3. 驗證進店辨識：區分店內閒聊路人顧客 vs 穿制服店員 vs 穿便服但站在非夥計禁止入內櫃檯區的掌事
        let room_coord = (2, 0);
        let needed_item = "wt-good-009";

        // (a) 路人顧客：無制服、不在櫃檯內場、無店家原料庫存
        let mut passerby = Entity::new_npc("路人顧客_阿明", "客");
        passerby.display_title = "店內喝茶閒聊的茶客顧客|阿明".to_string();
        passerby.grid_x = Some(2);
        passerby.grid_y = Some(0);
        let (u_passerby, c_passerby) = is_recognizable_room_staff_or_counter_holder_entity(&passerby, room_coord, needed_item);
        assert!(
            !u_passerby && !c_passerby,
            "passerby customer must NOT be recognized as staff or counter holder"
        );

        // (b) 穿制服店員：身穿制服
        let mut clerk = Entity::new_npc("鋪子夥計_小張", "員");
        clerk.display_title = "身穿制服店員|小張".to_string();
        clerk.grid_x = Some(2);
        clerk.grid_y = Some(0);
        let (u_clerk, _c_clerk) = is_recognizable_room_staff_or_counter_holder_entity(&clerk, room_coord, needed_item);
        assert!(
            u_clerk,
            "uniformed clerk must have visible uniform"
        );

        // (c) 穿便服但站在非夥計禁止入內櫃檯區的掌事：
        //     即便身穿便服（無制服），因站於非夥計禁止入內櫃檯管制區，必定被精準辨識為掌事
        let mut manager = Entity::new_npc("沙鹿掌事_老王", "王");
        manager.display_title = "穿便服但站在非夥計禁止入內櫃檯區的掌事|老王".to_string();
        manager.grid_x = Some(2);
        manager.grid_y = Some(0);
        let (u_manager, c_manager) = is_recognizable_room_staff_or_counter_holder_entity(&manager, room_coord, needed_item);
        assert!(
            !u_manager,
            "manager in plain clothes has no visible uniform"
        );
        assert!(
            c_manager,
            "manager standing in staff-only counter zone MUST be recognized as counter holder"
        );
    }
    #[test]
    fn test_all_1785_residents_receive_manuals_familiarity_breath_and_lineage() {
        let now = 1_800_200_000i64;
        seed_flagship_specimen_qin_shuisheng_once(now);

        let wenyao_name = crate::store::WENYAO_HOUSEHOLD_SLOTS[10].name();
        let outer_name = crate::store::OUTER_HOUSEHOLD_SLOTS[10].name();

        // Wenyao resident parity checks
        let wy_manuals = get_entity_studied_manuals(wenyao_name);
        assert!(
            !wy_manuals.is_empty(),
            "wenyao resident {wenyao_name} must receive studied manuals"
        );
        let wy_fam = get_entity_anatomy_familiarity_total(wenyao_name);
        assert!(
            wy_fam >= 24,
            "wenyao resident {wenyao_name} familiarity must be at least 24, got {wy_fam}"
        );
        let wy_lineage = get_entity_lineage_imprints(wenyao_name);
        assert!(
            !wy_lineage.is_empty(),
            "wenyao resident {wenyao_name} must receive lineage imprints"
        );

        // Outer resident parity checks
        let out_manuals = get_entity_studied_manuals(outer_name);
        assert!(
            !out_manuals.is_empty(),
            "outer resident {outer_name} must receive studied manuals"
        );
        let out_fam = get_entity_anatomy_familiarity_total(outer_name);
        assert!(
            out_fam >= 24,
            "outer resident {outer_name} familiarity must be at least 24, got {out_fam}"
        );
        let out_lineage = get_entity_lineage_imprints(outer_name);
        assert!(
            !out_lineage.is_empty(),
            "outer resident {outer_name} must receive lineage imprints"
        );
    }

}

const ALL_GATHER_CORRECTION_ID: &str = "living_world_v1_all_gather_correction";


/// 判斷物品是否為武器或工器具（排斥消耗品、食材、茶水、藥材、素材等）
pub fn is_weapon_or_tool_item(item_id: &str, item_name: &str) -> bool {
    let id_lower = item_id.to_lowercase();
    let name_lower = item_name.to_lowercase();

    // 先查物品定義與千物總典推導之穿戴槽位（hold_r/hold_l 為手持兵刃或工器具，body/head/feet/accessory 為衣帽護具配飾）
    if let Some(def) = crate::db::get_item_def(item_id) {
        if def.slot.starts_with("hold") || def.item_type == "weapon" || def.item_type == "tool" {
            return true;
        }
        if !def.slot.is_empty() {
            return false;
        }
    }
    // 排除名單：消耗品、食品、茶、藥、生肉、素材殘渣、空白手札、服飾等
    const EXCLUDE_SUBSTRINGS: &[&str] = &[
        "手札", "茶", "湯", "肉", "餅", "果", "藥", "散", "膏", "水", "漿", "燈", "曆", "袍", "衫", "襖",
        "火把", "匣", "斗篷",
        "corpse_", "mat_", "blank_manual", "food", "potion", "herb", "ingot",
        "torch", "box", "cloak", "lantern", "almanac",
    ];
    for &ex in EXCLUDE_SUBSTRINGS {
        if id_lower.contains(ex) || name_lower.contains(ex) {
            return false;
        }
    }

    // 包含名單：刀、劍、匕、斧、鎬、槍、錘、杖、針、刃、弓、盾、鑿、鋸、鏟等武器或工器具
    const WEAPON_TOOL_KEYWORDS: &[&str] = &[
        "刀", "劍", "匕", "斧", "鎬", "槍", "錘", "杖", "針", "刃", "弓", "盾", "鑿", "鋸", "鏟", "鏨", "篩架", "旋鑿", "刨",
        "竿", "剪", "鐮", "鍘", "槳", "篙", "扁擔", "算盤", "尺", "墨斗", "鉗", "傘", "杵", "臼", "網", "叉", "耙",
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
    pub first_interaction_at: i64,
    pub last_interaction_at: i64,
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


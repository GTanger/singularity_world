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

pub fn adjust_relationship(
    pool: &DbPool,
    entity_id: &str,
    other_entity_id: &str,
    relation_kind: &str,
    delta_trust: i32,
    delta_favor: i32,
) -> anyhow::Result<()> {
    let now = crate::game::now_unix();
    let delta = (delta_trust + delta_favor).max(1);
    record_relationship_delta(pool, entity_id, other_entity_id, relation_kind, delta, "npc_npc_dialogue", now)?;
    record_relationship_delta(pool, other_entity_id, entity_id, relation_kind, delta, "npc_npc_dialogue", now)?;
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
        "SELECT relation_kind, evidence_count, COALESCE(last_interaction_at, updated_at, 0) FROM resident_relationships WHERE entity_id = $1 AND other_entity_id = $2",
        &[&entity_id, &other_entity_id],
    )?;
    let mut active_kinds = Vec::new();
    let mut counts: HashMap<String, i32> = HashMap::new();
    let mut first_interaction_at: i64 = 0;
    let mut last_interaction_at: i64 = 0;
    for row in rows {
        let kind: String = row.get(0);
        let count: i32 = row.get(1);
        let ts: i64 = row.get(2);
        if ts > 0 {
            if first_interaction_at == 0 || ts < first_interaction_at {
                first_interaction_at = ts;
            }
            if ts > last_interaction_at {
                last_interaction_at = ts;
            }
        }
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
        first_interaction_at,
        last_interaction_at,
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
            .or_else(|| {
            crate::grid::parse_grid_room_id(place_id)
                .map(crate::world::place::legacy_place_id)
        })
        .unwrap_or_else(|| {
            let act_room = crate::store::get_store()
                .and_then(|s| {
                    s.read()
                        .entities
                        .get(actor_id)
                        .map(|e| format!("grid:{}:{}", e.x, e.y))
                })
                .unwrap_or_default();
            let s = crate::world::materials::infer_origin_settlement(if act_room.is_empty() { place_id } else { &act_room });
            if !s.is_empty() {
                format!("{s}・公共場所")
            } else {
                "文曜・公共場所".to_string()
            }
        })
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


pub fn recent_resident_knowledge_facts(entity_id: &str, limit: usize) -> Vec<String> {
    let Some(pool) = super::pool() else {
        return Vec::new();
    };
    let Ok(mut client) = pool.get() else {
        return Vec::new();
    };
    let limit_i64 = limit as i64;
    let rows = client
        .query(
            "SELECT value, COALESCE(updated_at, learned_at, 0) FROM resident_knowledge WHERE entity_id = $1 AND predicate != 'last_seen_at' ORDER BY certainty DESC, updated_at DESC LIMIT $2",
            &[&entity_id, &limit_i64],
        )
        .or_else(|_| {
            client.query(
                "SELECT value, COALESCE(updated_at, learned_at, 0) FROM resident_knowledge WHERE entity_id = $1 ORDER BY certainty DESC, updated_at DESC LIMIT $2",
                &[&entity_id, &limit_i64],
            )
        })
        .or_else(|_| {
            client.query(
                "SELECT fact_text, COALESCE(updated_at, learned_at, 0) FROM resident_knowledge WHERE entity_id = $1 ORDER BY confidence DESC, updated_at DESC LIMIT $2",
                &[&entity_id, &limit_i64],
            )
        });

    if let Ok(rows) = rows {
        let now = crate::game::now_unix();
        rows.into_iter()
            .filter_map(|r| {
                let val: String = r.try_get(0).ok()?;
                let ts: i64 = r.try_get(1).unwrap_or(0);
                let trimmed = val.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    let tag = crate::store::format_short_relative_time(ts, now);
                    if tag.is_empty() {
                        Some(trimmed.to_string())
                    } else {
                        Some(format!("{tag}{trimmed}"))
                    }
                }
            })
            .collect()
    } else {
        Vec::new()
    }
}

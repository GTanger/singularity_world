//! 市場工作場所的原子工作／報酬入口。
//!
//! 報酬只會從 Azgaar 中心聚落財庫投影出的有限工作場所財庫移給人物；不在
//! 每次工作時生成鎂。玩家與 NPC 只要具有同一正式指派，就使用同一入口。

use r2d2_postgres::postgres::GenericClient;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::store::sql::DbPool;
use crate::world::place::WHITE_TOWER_WORLD_ID;

const PAID_PERIOD_SECONDS: i64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkResult {
    pub work_id: Option<String>,
    pub action_id: Option<String>,
    pub entity_id: String,
    pub venue_id: String,
    pub place_id: String,
    pub occupation_id: String,
    pub wage_mg: i32,
    pub entity_magnesium: i32,
    pub treasury_mg: i32,
    pub inventory: String,
    pub current_activity: String,
    pub supply: Option<super::world_supply::UpstreamSupplyOutput>,
}

pub fn ensure_schema(client: &mut impl GenericClient) -> anyhow::Result<()> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS world_work_ledger (
            work_id TEXT PRIMARY KEY,
            world_id TEXT NOT NULL,
            runtime_id TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            venue_id TEXT NOT NULL,
            treasury_venue_id TEXT NOT NULL,
            place_id TEXT NOT NULL,
            occupation_id TEXT NOT NULL,
            work_period BIGINT NOT NULL,
            wage_mg INTEGER NOT NULL CHECK(wage_mg > 0),
            entity_mg_before INTEGER NOT NULL CHECK(entity_mg_before >= 0),
            entity_mg_after INTEGER NOT NULL CHECK(entity_mg_after >= 0),
            treasury_mg_before INTEGER NOT NULL CHECK(treasury_mg_before >= 0),
            treasury_mg_after INTEGER NOT NULL CHECK(treasury_mg_after >= 0),
            source_ref TEXT NOT NULL,
            occurred_at BIGINT NOT NULL,
            UNIQUE(entity_id,work_period),
            CHECK(entity_mg_before + treasury_mg_before =
                  entity_mg_after + treasury_mg_after)
        );
        CREATE INDEX IF NOT EXISTS idx_world_work_ledger_venue
        ON world_work_ledger(venue_id,occurred_at DESC);
        ALTER TABLE world_work_ledger
          ADD COLUMN IF NOT EXISTS treasury_venue_id TEXT;
        UPDATE world_work_ledger SET treasury_venue_id=venue_id
          WHERE treasury_venue_id IS NULL;
        ALTER TABLE world_work_ledger
          ALTER COLUMN treasury_venue_id SET NOT NULL;",
    )?;
    Ok(())
}

pub fn perform_shift(
    pool: &DbPool,
    entity_id: &str,
    now_unix: i64,
    source_ref: &str,
) -> anyhow::Result<Option<WorkResult>> {
    anyhow::ensure!(!entity_id.trim().is_empty(), "工作人物不可空白");
    let work_period = now_unix.div_euclid(PAID_PERIOD_SECONDS);
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let Some(row) = tx.query_opt(
        "SELECT e.kind,e.vit,e.magnesium,e.inventory,e.place_id,e.current_activity,
                a.venue_id,w.occupation_id,w.wage_mg,w.treasury_mg,w.runtime_id,
                w.site_kind
         FROM entities e
         JOIN assignments a ON a.entity_id=e.id
         JOIN world_work_sites w ON w.venue_id=a.venue_id AND w.place_id=e.place_id
         WHERE e.id=$1
         ORDER BY w.venue_id LIMIT 1 FOR UPDATE OF e,w",
        &[&entity_id],
    )?
    else {
        return Ok(None);
    };
    let kind: String = row.get(0);
    let vit: i32 = row.get(1);
    let entity_mg_before: i32 = row.get(2);
    let inventory: String = row.get(3);
    let place_id: String = row
        .get::<_, Option<String>>(4)
        .ok_or_else(|| anyhow::anyhow!("工作人物沒有正式位置"))?;
    let previous_activity: String = row.get(5);
    let venue_id: String = row.get(6);
    let occupation_id: String = row.get(7);
    let wage_mg: i32 = row.get(8);
    let treasury_mg_before: i32 = row.get(9);
    let runtime_id: String = row.get(10);
    let site_kind: String = row.get(11);
    anyhow::ensure!(matches!(kind.as_str(), "player" | "npc"), "非人物不可工作");
    anyhow::ensure!(vit > 0, "人物已無法工作");
    if tx
        .query_opt(
            "SELECT 1 FROM world_work_ledger WHERE entity_id=$1 AND work_period=$2",
            &[&entity_id, &work_period],
        )?
        .is_some()
    {
        return Ok(None);
    }

    let prepared_supply = if site_kind == "producer" {
        super::world_supply::prepare_shift_in_tx(
            &mut tx,
            &runtime_id,
            &place_id,
            entity_id,
            work_period,
            wage_mg,
        )?
    } else {
        None
    };
    if site_kind == "producer" && prepared_supply.is_none() {
        let current_activity = if kind == "npc" {
            "供應工作暫停，正在等候市場需求或款項".to_string()
        } else {
            previous_activity
        };
        tx.execute(
            "UPDATE entities SET current_activity=$1 WHERE id=$2",
            &[&current_activity, &entity_id],
        )?;
        tx.commit()?;
        return Ok(Some(WorkResult {
            work_id: None,
            action_id: None,
            entity_id: entity_id.to_string(),
            venue_id,
            place_id,
            occupation_id,
            wage_mg: 0,
            entity_magnesium: entity_mg_before,
            treasury_mg: 0,
            inventory,
            current_activity,
            supply: None,
        }));
    }
    let (treasury_venue_id, treasury_mg_before) = prepared_supply
        .as_ref()
        .map(|prepared| {
            (
                prepared.treasury_venue_id.clone(),
                prepared.treasury_mg_before,
            )
        })
        .unwrap_or_else(|| (venue_id.clone(), treasury_mg_before));
    if treasury_mg_before < wage_mg {
        let current_activity = if kind == "npc" {
            "工作暫停，正在等候薪資結算".to_string()
        } else {
            previous_activity
        };
        tx.execute(
            "UPDATE entities SET current_activity=$1 WHERE id=$2",
            &[&current_activity, &entity_id],
        )?;
        tx.commit()?;
        return Ok(Some(WorkResult {
            work_id: None,
            action_id: None,
            entity_id: entity_id.to_string(),
            venue_id,
            place_id,
            occupation_id,
            wage_mg: 0,
            entity_magnesium: entity_mg_before,
            treasury_mg: treasury_mg_before,
            inventory,
            current_activity,
            supply: None,
        }));
    }

    let entity_mg_after = entity_mg_before
        .checked_add(wage_mg)
        .ok_or_else(|| anyhow::anyhow!("人物鎂數溢位"))?;
    let treasury_mg_after = treasury_mg_before - wage_mg;
    let mut current_activity = if kind == "npc" {
        format!("正在從事{occupation_id}")
    } else {
        previous_activity
    };
    tx.execute(
        "UPDATE entities SET magnesium=$1,current_activity=$2 WHERE id=$3",
        &[&entity_mg_after, &current_activity, &entity_id],
    )?;
    tx.execute(
        "UPDATE world_work_sites SET treasury_mg=$1,updated_at=$2
         WHERE runtime_id=$3 AND venue_id=$4",
        &[
            &treasury_mg_after,
            &now_unix,
            &runtime_id,
            &treasury_venue_id,
        ],
    )?;
    let work_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO world_work_ledger
         (work_id,world_id,runtime_id,entity_id,venue_id,place_id,occupation_id,
          treasury_venue_id,work_period,wage_mg,entity_mg_before,entity_mg_after,
          treasury_mg_before,treasury_mg_after,source_ref,occurred_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)",
        &[
            &work_id,
            &WHITE_TOWER_WORLD_ID,
            &runtime_id,
            &entity_id,
            &venue_id,
            &place_id,
            &occupation_id,
            &treasury_venue_id,
            &work_period,
            &wage_mg,
            &entity_mg_before,
            &entity_mg_after,
            &treasury_mg_before,
            &treasury_mg_after,
            &source_ref,
            &now_unix,
        ],
    )?;
    let supply = prepared_supply
        .map(|prepared| {
            super::world_supply::dispatch_in_tx(
                &mut tx,
                prepared,
                entity_id,
                &work_id,
                work_period,
                now_unix,
            )
        })
        .transpose()?;
    if let Some(supply) = &supply
        && supply.delivery_kind == "shipment"
    {
        current_activity = format!(
            "正運送{}份{}前往目的市場",
            supply.quantity, supply.display_name
        );
        tx.execute(
            "UPDATE entities SET current_activity=$1 WHERE id=$2",
            &[&current_activity, &entity_id],
        )?;
    }
    let action_id = crate::pg::living_world::record_action_in_tx(
        &mut tx,
        crate::pg::living_world::ActionDraft {
            actor_id: entity_id,
            action_key: "work",
            target_ref: &venue_id,
            place_id: &place_id,
            preconditions: serde_json::json!({
                "work_id": work_id,
                "assigned_venue": venue_id,
                "actor_at_work_site": true,
                "treasury_mg": treasury_mg_before
            }),
            effects: serde_json::json!({
                "work_id": work_id,
                "occupation_id": occupation_id,
                "wage_mg": wage_mg,
                "entity_magnesium": {"before": entity_mg_before, "after": entity_mg_after},
                "treasury_magnesium": {"before": treasury_mg_before, "after": treasury_mg_after},
                "upstream_supply": supply
            }),
            outcome: if supply.is_some() {
                "upstream_supply_started_and_paid"
            } else {
                "work_completed_and_paid"
            },
            source_ref,
            occurred_at: now_unix,
        },
    )?;
    tx.commit()?;
    Ok(Some(WorkResult {
        work_id: Some(work_id),
        action_id: Some(action_id),
        entity_id: entity_id.to_string(),
        venue_id,
        place_id,
        occupation_id,
        wage_mg,
        entity_magnesium: entity_mg_after,
        treasury_mg: treasury_mg_after,
        inventory,
        current_activity,
        supply,
    }))
}

pub fn ledger_balance(client: &mut impl GenericClient) -> anyhow::Result<(i64, i64, i64)> {
    let row = client.query_one(
        "SELECT COUNT(*)::bigint,COALESCE(SUM(wage_mg),0)::bigint,
                COUNT(*) FILTER (WHERE entity_mg_before+treasury_mg_before<>
                  entity_mg_after+treasury_mg_after)::bigint
         FROM world_work_ledger",
        &[],
    )?;
    Ok((row.get(0), row.get(1), row.get(2)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paid_period_is_stable_for_one_five_minute_window() {
        assert_eq!(
            301_i64.div_euclid(PAID_PERIOD_SECONDS),
            599_i64.div_euclid(PAID_PERIOD_SECONDS)
        );
        assert_ne!(
            599_i64.div_euclid(PAID_PERIOD_SECONDS),
            600_i64.div_euclid(PAID_PERIOD_SECONDS)
        );
    }
}

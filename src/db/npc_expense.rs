//! NPC 每日食宿消耗；一個遊戲日只結算一次，重啟不重扣。

use crate::store;

use super::disposition::{add_thought_to_entity, add_thought_to_entity_with_flavor};
use super::{ErrNoStore, ThoughtSource};

pub const DAILY_EXPENSE_BASE: i32 = 8;

pub fn deduct_daily_expense(game_day: i32, now_unix: i64) -> anyhow::Result<i32> {
    let pool = crate::pg::pool().ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
    let Some(results) = crate::pg::economy::apply_daily_expense_once(
        &pool,
        game_day,
        now_unix,
        DAILY_EXPENSE_BASE,
    )?
    else {
        return Ok(0);
    };
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let (pool, snapshots) = {
        let mut store = arc.write();
        let pool = store
            .db_pool
            .clone()
            .ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
        let mut snapshots = Vec::with_capacity(results.len());
        for result in &results {
            let Some(entity) = store.entities.get_mut(&result.entity_id) else {
                continue;
            };
            entity.magnesium = result.magnesium;
            if entity.magnesium == 0 {
                let last_is_broke = entity.thoughts.last().map(|t| t.source == "Broke").unwrap_or(false);
                if !last_is_broke {
                    add_thought_to_entity(entity, ThoughtSource::Broke, now_unix);
                }
            } else if entity.magnesium > 0 && entity.magnesium < 35 {
                add_thought_to_entity_with_flavor(
                    entity,
                    ThoughtSource::Daily,
                    now_unix,
                    "手頭鎂幣漸緊，得盤算多接幾趟差事",
                );
            } else if entity.magnesium >= 80 && (entity.thoughts.is_empty() || entity.disposition < 0) {
                add_thought_to_entity_with_flavor(
                    entity,
                    ThoughtSource::WorkDone,
                    now_unix,
                    "手頭銀錢充裕、食宿安穩，心裡頗為踏實",
                );
            }
            snapshots.push(entity.clone());
        }
        (pool, snapshots)
    };
    crate::pg::entity::upsert_batch(&pool, &snapshots)?;
    Ok(i32::try_from(
        results.iter().filter(|result| result.charged).count(),
    )?)
}

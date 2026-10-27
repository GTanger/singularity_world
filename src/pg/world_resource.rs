//! 白塔半島場所資源：來源庫存、原子採集與守衡帳。

use r2d2_postgres::postgres::{GenericClient, Transaction};
use serde::{Deserialize, Serialize};

use crate::store::sql::DbPool;
use crate::world::runtime_world::{WHITE_TOWER_RUNTIME_ID, WhiteTowerRuntimeWorld};

/// 舊 P0「遍地可撿柴枝／粗礦」只保留為既有資料與帳本證據，不再是白塔
/// runtime 的可見或可執行物資層。真正生產來源另由現代產業節點接替。
pub const LEGACY_P0_RUNTIME_ENABLED: bool = false;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceState {
    pub place_id: String,
    pub item_id: String,
    pub name: String,
    pub origin_label: String,
    pub capacity: i32,
    pub current_stock: i32,
    pub regeneration_per_day: i32,
    pub last_regenerated_day: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatherResult {
    pub action_id: String,
    pub place_id: String,
    pub entity_id: String,
    pub item_id: String,
    pub name: String,
    pub quantity: i32,
    pub source_stock_after: i32,
    pub entity_quantity_after: i32,
    pub inventory: String,
    pub narrative: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegenerationReport {
    pub sources_changed: usize,
    pub quantity_created: i64,
}

pub fn ensure_schema(client: &mut impl GenericClient) -> anyhow::Result<()> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS world_resource_sources (
            runtime_id TEXT NOT NULL,
            place_id TEXT NOT NULL,
            item_id TEXT NOT NULL,
            name TEXT NOT NULL,
            origin_label TEXT NOT NULL,
            capacity INTEGER NOT NULL CHECK(capacity >= 0),
            current_stock INTEGER NOT NULL CHECK(current_stock >= 0),
            regeneration_per_day INTEGER NOT NULL CHECK(regeneration_per_day >= 0),
            last_regenerated_day INTEGER NOT NULL DEFAULT 0,
            updated_at BIGINT NOT NULL,
            PRIMARY KEY(runtime_id,place_id,item_id)
        );
        CREATE TABLE IF NOT EXISTS world_resource_ledger (
            id BIGSERIAL PRIMARY KEY,
            runtime_id TEXT NOT NULL,
            place_id TEXT NOT NULL,
            entity_id TEXT,
            item_id TEXT NOT NULL,
            source_delta INTEGER NOT NULL,
            inventory_delta INTEGER NOT NULL,
            source_stock_after INTEGER NOT NULL,
            entity_quantity_after INTEGER,
            cause TEXT NOT NULL,
            game_day INTEGER NOT NULL,
            created_at BIGINT NOT NULL,
            CHECK(source_delta + inventory_delta = 0 OR cause='regeneration')
        );
        CREATE INDEX IF NOT EXISTS idx_world_resource_ledger_place
        ON world_resource_ledger(runtime_id,place_id,id);",
    )?;
    Ok(())
}

pub(super) fn upsert_sources(
    tx: &mut Transaction<'_>,
    world: &WhiteTowerRuntimeWorld,
    now_unix: i64,
) -> anyhow::Result<()> {
    for source in &world.resources {
        tx.execute(
            "INSERT INTO world_resource_sources
             (runtime_id,place_id,item_id,name,origin_label,capacity,current_stock,
              regeneration_per_day,last_regenerated_day,updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$6,$7,0,$8)
             ON CONFLICT(runtime_id,place_id,item_id) DO UPDATE SET
             name=EXCLUDED.name,origin_label=EXCLUDED.origin_label,capacity=EXCLUDED.capacity,
             current_stock=LEAST(world_resource_sources.current_stock,EXCLUDED.capacity),
             regeneration_per_day=EXCLUDED.regeneration_per_day,updated_at=EXCLUDED.updated_at",
            &[
                &WHITE_TOWER_RUNTIME_ID,
                &source.place_id,
                &source.item_id,
                &source.name,
                &source.origin_label,
                &source.capacity,
                &source.regeneration_per_day,
                &now_unix,
            ],
        )?;
    }
    Ok(())
}

pub fn get_at_place(pool: &DbPool, place_id: &str) -> anyhow::Result<Option<ResourceState>> {
    if !LEGACY_P0_RUNTIME_ENABLED {
        return Ok(None);
    }
    let mut conn = pool.get()?;
    conn.query_opt(
        "SELECT place_id,item_id,name,origin_label,capacity,current_stock,
                regeneration_per_day,last_regenerated_day
         FROM world_resource_sources WHERE runtime_id=$1 AND place_id=$2 ORDER BY item_id LIMIT 1",
        &[&WHITE_TOWER_RUNTIME_ID, &place_id],
    )?
    .map(resource_from_row)
    .transpose()
}

/// 把來源物質移入人物背包；來源扣量、背包加量與帳本同一交易完成。
pub fn gather(
    pool: &DbPool,
    entity_id: &str,
    place_id: &str,
    requested: i32,
    game_day: i32,
    now_unix: i64,
    cause: &str,
) -> anyhow::Result<Option<GatherResult>> {
    anyhow::ensure!(requested > 0, "採集數量必須大於零");
    if !LEGACY_P0_RUNTIME_ENABLED {
        return Ok(None);
    }
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let Some(mut state) = lock_resource(&mut tx, place_id)? else {
        return Ok(None);
    };
    regenerate_locked(&mut tx, &mut state, game_day, now_unix)?;
    if state.current_stock <= 0 {
        tx.commit()?;
        return Ok(None);
    }
    let quantity = requested.min(state.current_stock);
    let Some(row) = tx.query_opt(
        "SELECT inventory FROM entities WHERE id=$1 AND place_id=$2 FOR UPDATE",
        &[&entity_id, &place_id],
    )?
    else {
        anyhow::bail!("人物不在資源所在場所：{entity_id} / {place_id}");
    };
    let inventory_before: String = row.get(0);
    let (inventory, entity_quantity_after) =
        add_inventory_quantity(&inventory_before, &state.item_id, quantity)?;
    let source_stock_before = state.current_stock;
    let entity_quantity_before = entity_quantity_after - quantity;
    state.current_stock -= quantity;
    tx.execute(
        "UPDATE world_resource_sources SET current_stock=$1,updated_at=$2
         WHERE runtime_id=$3 AND place_id=$4 AND item_id=$5",
        &[
            &state.current_stock,
            &now_unix,
            &WHITE_TOWER_RUNTIME_ID,
            &place_id,
            &state.item_id,
        ],
    )?;
    tx.execute(
        "UPDATE entities SET inventory=$1 WHERE id=$2",
        &[&inventory, &entity_id],
    )?;
    insert_ledger(
        &mut tx,
        &state,
        Some(entity_id),
        -quantity,
        quantity,
        Some(entity_quantity_after),
        cause,
        game_day,
        now_unix,
    )?;
    let target_ref = format!("resource:{}:{}", state.place_id, state.item_id);
    let action_id = crate::pg::living_world::record_action_in_tx(
        &mut tx,
        crate::pg::living_world::ActionDraft {
            actor_id: entity_id,
            action_key: "gather",
            target_ref: &target_ref,
            place_id,
            preconditions: serde_json::json!({
                "actor_place_id": place_id,
                "source_stock": source_stock_before,
                "requested": requested
            }),
            effects: serde_json::json!({
                "source_stock": {"before": source_stock_before, "after": state.current_stock},
                "inventory_quantity": {
                    "before": entity_quantity_before,
                    "after": entity_quantity_after
                },
                "item_id": state.item_id
            }),
            outcome: "gathered",
            source_ref: cause,
            occurred_at: now_unix,
        },
    )?;
    crate::pg::living_world::remember_fact(
        &mut tx,
        entity_id,
        &target_ref,
        "observed_stock_after_action",
        &state.current_stock.to_string(),
        100,
        &action_id,
        now_unix,
    )?;
    tx.commit()?;
    Ok(Some(GatherResult {
        action_id,
        place_id: place_id.to_string(),
        entity_id: entity_id.to_string(),
        item_id: state.item_id.clone(),
        name: state.name.clone(),
        quantity,
        source_stock_after: state.current_stock,
        entity_quantity_after,
        inventory,
        narrative: format!(
            "從{}取走{}{}份；原地還看得見{}份。",
            state.origin_label, state.name, quantity, state.current_stock
        ),
    }))
}

/// 依遊戲日補回所有來源；同一天重複呼叫不會重複生成。
pub fn regenerate_all(
    pool: &DbPool,
    game_day: i32,
    now_unix: i64,
) -> anyhow::Result<RegenerationReport> {
    if !LEGACY_P0_RUNTIME_ENABLED {
        return Ok(RegenerationReport {
            sources_changed: 0,
            quantity_created: 0,
        });
    }
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let rows = tx.query(
        "SELECT place_id FROM world_resource_sources
         WHERE runtime_id=$1 AND last_regenerated_day<$2 ORDER BY place_id FOR UPDATE",
        &[&WHITE_TOWER_RUNTIME_ID, &game_day],
    )?;
    let mut sources_changed = 0;
    let mut quantity_created = 0_i64;
    for row in rows {
        let place_id: String = row.get(0);
        let Some(mut state) = lock_resource(&mut tx, &place_id)? else {
            continue;
        };
        let before = state.current_stock;
        regenerate_locked(&mut tx, &mut state, game_day, now_unix)?;
        let created = state.current_stock - before;
        if created > 0 {
            sources_changed += 1;
            quantity_created += i64::from(created);
        }
    }
    tx.commit()?;
    Ok(RegenerationReport {
        sources_changed,
        quantity_created,
    })
}

pub fn ledger_balance(pool: &DbPool) -> anyhow::Result<(i64, i64, i64)> {
    let mut conn = pool.get()?;
    let row = conn.query_one(
        "SELECT COALESCE(SUM(source_delta),0)::bigint,
                COALESCE(SUM(inventory_delta),0)::bigint,
                COALESCE(SUM(CASE WHEN cause='regeneration' THEN source_delta ELSE 0 END),0)::bigint
         FROM world_resource_ledger WHERE runtime_id=$1",
        &[&WHITE_TOWER_RUNTIME_ID],
    )?;
    Ok((row.get(0), row.get(1), row.get(2)))
}

fn lock_resource(
    tx: &mut Transaction<'_>,
    place_id: &str,
) -> anyhow::Result<Option<ResourceState>> {
    tx.query_opt(
        "SELECT place_id,item_id,name,origin_label,capacity,current_stock,
                regeneration_per_day,last_regenerated_day
         FROM world_resource_sources WHERE runtime_id=$1 AND place_id=$2
         ORDER BY item_id LIMIT 1 FOR UPDATE",
        &[&WHITE_TOWER_RUNTIME_ID, &place_id],
    )?
    .map(resource_from_row)
    .transpose()
}

fn resource_from_row(row: r2d2_postgres::postgres::Row) -> anyhow::Result<ResourceState> {
    let state = ResourceState {
        place_id: row.get(0),
        item_id: row.get(1),
        name: row.get(2),
        origin_label: row.get(3),
        capacity: row.get(4),
        current_stock: row.get(5),
        regeneration_per_day: row.get(6),
        last_regenerated_day: row.get(7),
    };
    anyhow::ensure!(state.current_stock <= state.capacity, "資源庫存超過容量");
    Ok(state)
}

fn regenerate_locked(
    tx: &mut Transaction<'_>,
    state: &mut ResourceState,
    game_day: i32,
    now_unix: i64,
) -> anyhow::Result<()> {
    if game_day <= state.last_regenerated_day {
        return Ok(());
    }
    let elapsed = game_day - state.last_regenerated_day;
    let before = state.current_stock;
    state.current_stock = state
        .current_stock
        .saturating_add(state.regeneration_per_day.saturating_mul(elapsed))
        .min(state.capacity);
    state.last_regenerated_day = game_day;
    tx.execute(
        "UPDATE world_resource_sources SET current_stock=$1,last_regenerated_day=$2,updated_at=$3
         WHERE runtime_id=$4 AND place_id=$5 AND item_id=$6",
        &[
            &state.current_stock,
            &game_day,
            &now_unix,
            &WHITE_TOWER_RUNTIME_ID,
            &state.place_id,
            &state.item_id,
        ],
    )?;
    let created = state.current_stock - before;
    if created > 0 {
        insert_ledger(
            tx,
            state,
            None,
            created,
            0,
            None,
            "regeneration",
            game_day,
            now_unix,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_ledger(
    tx: &mut Transaction<'_>,
    state: &ResourceState,
    entity_id: Option<&str>,
    source_delta: i32,
    inventory_delta: i32,
    entity_quantity_after: Option<i32>,
    cause: &str,
    game_day: i32,
    now_unix: i64,
) -> anyhow::Result<()> {
    tx.execute(
        "INSERT INTO world_resource_ledger
         (runtime_id,place_id,entity_id,item_id,source_delta,inventory_delta,
          source_stock_after,entity_quantity_after,cause,game_day,created_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        &[
            &WHITE_TOWER_RUNTIME_ID,
            &state.place_id,
            &entity_id,
            &state.item_id,
            &source_delta,
            &inventory_delta,
            &state.current_stock,
            &entity_quantity_after,
            &cause,
            &game_day,
            &now_unix,
        ],
    )?;
    Ok(())
}

fn add_inventory_quantity(
    raw: &str,
    item_id: &str,
    quantity: i32,
) -> anyhow::Result<(String, i32)> {
    let mut value: serde_json::Value = if raw.trim().is_empty() {
        serde_json::json!([])
    } else {
        serde_json::from_str(raw)?
    };
    let array = value
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("人物背包不是 JSON 陣列"))?;
    for entry in array.iter_mut() {
        if entry.get("item_id").and_then(serde_json::Value::as_str) != Some(item_id) {
            continue;
        }
        let before = entry
            .get("qty")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        let after = before.saturating_add(i64::from(quantity));
        anyhow::ensure!(after <= i64::from(i32::MAX), "背包數量溢位");
        entry["qty"] = serde_json::json!(after);
        return Ok((serde_json::to_string(&value)?, after as i32));
    }
    array.push(serde_json::json!({"item_id": item_id, "qty": quantity}));
    Ok((serde_json::to_string(&value)?, quantity))
}

#[cfg(test)]
mod tests {
    use super::add_inventory_quantity;

    #[test]
    fn inventory_add_preserves_unrelated_fields() {
        let raw = r#"[{"item_id":"mat_hay_bundle","qty":2,"quality":"dry"}]"#;
        let (updated, qty) = add_inventory_quantity(raw, "mat_hay_bundle", 1).expect("add");
        assert_eq!(qty, 3);
        assert!(updated.contains("\"quality\":\"dry\""));
    }

    #[test]
    fn inventory_add_creates_stack() {
        let (updated, qty) = add_inventory_quantity("[]", "mat_reed_bundle", 2).expect("add");
        assert_eq!(qty, 2);
        assert_eq!(updated, r#"[{"item_id":"mat_reed_bundle","qty":2}]"#);
    }
}

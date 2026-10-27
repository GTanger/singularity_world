//! 市場間貨物的實體運送。
//!
//! 出發時貨物從來源市場轉入 shipment，抵達時才進目的市場；承運居民必須真的沿
//! `world_routes` 行走。重啟、死亡或中斷不會把在途貨物憑空補回任一市場。

use r2d2_postgres::postgres::GenericClient;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::store::sql::DbPool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryAssignment {
    pub shipment_id: String,
    pub destination_place_id: String,
    pub display_name: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShipmentDispatch {
    pub shipment_id: String,
    pub carrier_id: String,
    pub origin_place_id: String,
    pub destination_place_id: String,
    pub item_id: String,
    pub display_name: String,
    pub quantity: i32,
    pub origin_quantity_before: i32,
    pub origin_quantity_after: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryResult {
    pub shipment_id: String,
    pub carrier_id: String,
    pub destination_place_id: String,
    pub item_id: String,
    pub display_name: String,
    pub quantity: i32,
    pub destination_quantity_before: i32,
    pub destination_quantity_after: i32,
    pub action_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProducerShipmentProbe {
    pub shipment_id: String,
    pub carrier_id: Option<String>,
    pub origin_place_id: String,
    pub destination_place_id: String,
    pub current_place_id: String,
    pub status: String,
    pub display_name: String,
    pub quantity: i32,
    pub carrier_place_id: Option<String>,
    pub carrier_activity: Option<String>,
    pub carrier_intent: Option<String>,
    pub carrier_destination_place_id: Option<String>,
    pub carrier_next_wake_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogisticsStatus {
    pub total: i64,
    pub market_origin: i64,
    pub producer_origin: i64,
    pub in_transit: i64,
    pub stranded: i64,
    pub delivered: i64,
    pub invalid_origin_balances: i64,
    pub invalid_destination_balances: i64,
}

pub fn ensure_schema(client: &mut impl GenericClient) -> anyhow::Result<()> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS world_market_shipments (
            shipment_id TEXT PRIMARY KEY,
            runtime_id TEXT NOT NULL,
            origin_kind TEXT NOT NULL DEFAULT 'market'
              CONSTRAINT world_market_shipments_origin_kind_valid
              CHECK(origin_kind IN ('market','producer')),
            source_deal_id INTEGER NOT NULL,
            carrier_id TEXT,
            from_market_id INTEGER NOT NULL,
            to_market_id INTEGER NOT NULL,
            source_good_id INTEGER NOT NULL,
            item_id TEXT NOT NULL,
            origin_place_id TEXT NOT NULL,
            destination_place_id TEXT NOT NULL,
            current_place_id TEXT NOT NULL,
            quantity INTEGER NOT NULL CHECK(quantity > 0),
            status TEXT NOT NULL CHECK(status IN ('in_transit','stranded','delivered','failed')),
            origin_quantity_before INTEGER NOT NULL CHECK(origin_quantity_before >= 0),
            origin_quantity_after INTEGER NOT NULL CHECK(origin_quantity_after >= 0),
            destination_quantity_before INTEGER,
            destination_quantity_after INTEGER,
            planned_path TEXT NOT NULL,
            travel_cost INTEGER NOT NULL CHECK(travel_cost > 0),
            failure_reason TEXT NOT NULL DEFAULT '',
            source_ref TEXT NOT NULL,
            dispatched_at BIGINT NOT NULL,
            updated_at BIGINT NOT NULL,
            delivered_at BIGINT,
            CHECK(from_market_id <> to_market_id),
            CHECK(origin_quantity_before = origin_quantity_after + quantity),
            CHECK((status <> 'delivered') OR
                  (destination_quantity_before IS NOT NULL AND
                   destination_quantity_after = destination_quantity_before + quantity))
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_world_market_shipments_active_carrier
        ON world_market_shipments(carrier_id)
        WHERE status='in_transit' AND carrier_id IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_world_market_shipments_destination
        ON world_market_shipments(status,destination_place_id,updated_at);
        CREATE INDEX IF NOT EXISTS idx_world_market_shipments_lane
        ON world_market_shipments(runtime_id,source_deal_id,status);
        ALTER TABLE world_market_shipments ALTER COLUMN carrier_id DROP NOT NULL;
        ALTER TABLE world_market_shipments
          ADD COLUMN IF NOT EXISTS origin_kind TEXT NOT NULL DEFAULT 'market';
        DO $migration$
        BEGIN
          IF NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conname='world_market_shipments_origin_kind_valid'
          ) THEN
            ALTER TABLE world_market_shipments
              ADD CONSTRAINT world_market_shipments_origin_kind_valid
              CHECK(origin_kind IN ('market','producer'));
          END IF;
        END
        $migration$;",
    )?;
    Ok(())
}

pub fn dispatch_one(pool: &DbPool, now_unix: i64) -> anyhow::Result<Option<ShipmentDispatch>> {
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let Some(row) = tx.query_opt(
        "SELECT l.runtime_id,l.source_deal_id,l.from_market_id,l.to_market_id,
                l.source_good_id,l.evidence_units,l.planned_path,l.travel_cost,l.source_ref,
                origin.quantity,origin.target_quantity,origin.reference_price_mg,
                destination.quantity,destination.target_quantity,
                origin.item_id,g.display_name,from_market.place_id,to_market.place_id
         FROM world_market_flow_lanes l
         JOIN world_market_inventory origin ON origin.runtime_id=l.runtime_id
              AND origin.source_market_id=l.from_market_id
              AND origin.source_good_id=l.source_good_id
         JOIN world_market_inventory destination ON destination.runtime_id=l.runtime_id
              AND destination.source_market_id=l.to_market_id
              AND destination.source_good_id=l.source_good_id
         JOIN world_goods g ON g.runtime_id=l.runtime_id AND g.source_good_id=l.source_good_id
         JOIN world_markets from_market ON from_market.runtime_id=l.runtime_id
              AND from_market.source_market_id=l.from_market_id
         JOIN world_markets to_market ON to_market.runtime_id=l.runtime_id
              AND to_market.source_market_id=l.to_market_id
         WHERE l.active AND origin.quantity>origin.target_quantity
           AND destination.quantity<destination.target_quantity
           AND NOT EXISTS (
             SELECT 1 FROM world_market_shipments shipment
             WHERE shipment.runtime_id=l.runtime_id AND shipment.source_deal_id=l.source_deal_id
               AND shipment.status IN ('in_transit','stranded'))
         ORDER BY (destination.target_quantity-destination.quantity)*l.evidence_units DESC,
                  abs(hashtextextended(l.source_deal_id::text||':'||($1::bigint/15)::text,0))
         LIMIT 1 FOR UPDATE OF origin,destination SKIP LOCKED",
        &[&now_unix],
    )?
    else {
        tx.commit()?;
        return Ok(None);
    };
    let runtime_id: String = row.get(0);
    let source_deal_id: i32 = row.get(1);
    let from_market_id: i32 = row.get(2);
    let to_market_id: i32 = row.get(3);
    let source_good_id: i32 = row.get(4);
    let evidence_units: f64 = row.get(5);
    let planned_path: String = row.get(6);
    let travel_cost: i32 = row.get(7);
    let lane_source_ref: String = row.get(8);
    let origin_before: i32 = row.get(9);
    let origin_target: i32 = row.get(10);
    let origin_reference_price: i32 = row.get(11);
    let destination_before: i32 = row.get(12);
    let destination_target: i32 = row.get(13);
    let item_id: String = row.get(14);
    let display_name: String = row.get(15);
    let origin_place_id: String = row.get(16);
    let destination_place_id: String = row.get(17);

    let Some(carrier_row) = tx.query_opt(
        "SELECT e.id
         FROM world_work_sites w
         JOIN assignments a ON a.venue_id=w.venue_id
         JOIN entities e ON e.id=a.entity_id AND e.place_id=w.place_id
         LEFT JOIN entity_journeys j ON j.entity_id=e.id
         WHERE w.runtime_id=$1 AND w.site_kind='market' AND w.source_market_id=$2
           AND e.kind='npc' AND e.vit>0
           AND COALESCE(j.planned_path,'[]')='[]'
           AND NOT EXISTS (
             SELECT 1 FROM world_market_shipments s
             WHERE s.carrier_id=e.id AND s.status='in_transit')
         ORDER BY abs(hashtextextended(e.id||':'||$3::integer::text||':'||($4::bigint/15)::text,0)),e.id
         LIMIT 1 FOR UPDATE OF e SKIP LOCKED",
        &[&runtime_id, &from_market_id, &source_deal_id, &now_unix],
    )?
    else {
        tx.commit()?;
        return Ok(None);
    };
    let carrier_id: String = carrier_row.get(0);
    let surplus = origin_before - origin_target;
    let shortage = destination_target - destination_before;
    let quantity = (evidence_units.ceil().clamp(1.0, 3.0) as i32)
        .min(surplus)
        .min(shortage);
    if quantity <= 0 {
        tx.commit()?;
        return Ok(None);
    }
    let origin_after = origin_before - quantity;
    let origin_price =
        super::world_market::price_for_stock(origin_reference_price, origin_after, origin_target);
    tx.execute(
        "UPDATE world_market_inventory SET quantity=$1,price_mg=$2,updated_at=$3
         WHERE runtime_id=$4 AND source_market_id=$5 AND source_good_id=$6",
        &[
            &origin_after,
            &origin_price,
            &now_unix,
            &runtime_id,
            &from_market_id,
            &source_good_id,
        ],
    )?;
    let shipment_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO world_market_shipments
         (shipment_id,runtime_id,origin_kind,source_deal_id,carrier_id,from_market_id,to_market_id,
          source_good_id,item_id,origin_place_id,destination_place_id,current_place_id,
          quantity,status,origin_quantity_before,origin_quantity_after,
          destination_quantity_before,destination_quantity_after,planned_path,travel_cost,
          failure_reason,source_ref,dispatched_at,updated_at,delivered_at)
         VALUES ($1,$2,'market',$3,$4,$5,$6,$7,$8,$9,$10,$9,$11,'in_transit',$12,$13,
                 NULL,NULL,$14,$15,'',$16,$17,$17,NULL)",
        &[
            &shipment_id,
            &runtime_id,
            &source_deal_id,
            &carrier_id,
            &from_market_id,
            &to_market_id,
            &source_good_id,
            &item_id,
            &origin_place_id,
            &destination_place_id,
            &quantity,
            &origin_before,
            &origin_after,
            &planned_path,
            &travel_cost,
            &lane_source_ref,
            &now_unix,
        ],
    )?;
    upsert_delivery_obligation(
        &mut tx,
        &shipment_id,
        &carrier_id,
        &lane_source_ref,
        now_unix,
    )?;
    tx.execute(
        "UPDATE resident_continuity SET current_intent='work',
           intent_reason='已有在途市場貨物必須送達',destination_place_id=$1,
           blocked_by=NULL,next_wake_at=LEAST(COALESCE(next_wake_at,$2),$2),
           revision=revision+1,updated_at=$2,source_kind='runtime_logistics',
           source_ref=$3 WHERE entity_id=$4",
        &[
            &destination_place_id,
            &now_unix,
            &lane_source_ref,
            &carrier_id,
        ],
    )?;
    tx.commit()?;
    Ok(Some(ShipmentDispatch {
        shipment_id,
        carrier_id,
        origin_place_id,
        destination_place_id,
        item_id,
        display_name,
        quantity,
        origin_quantity_before: origin_before,
        origin_quantity_after: origin_after,
    }))
}

pub fn active_delivery_target(
    pool: &DbPool,
    carrier_id: &str,
) -> anyhow::Result<Option<DeliveryAssignment>> {
    let mut conn = pool.get()?;
    Ok(conn
        .query_opt(
            "SELECT s.shipment_id,s.destination_place_id,g.display_name,s.quantity
             FROM world_market_shipments s
             JOIN world_goods g ON g.runtime_id=s.runtime_id AND g.source_good_id=s.source_good_id
             WHERE s.carrier_id=$1 AND s.status='in_transit'",
            &[&carrier_id],
        )?
        .map(|row| DeliveryAssignment {
            shipment_id: row.get(0),
            destination_place_id: row.get(1),
            display_name: row.get(2),
            quantity: row.get(3),
        }))
}

pub fn complete_if_arrived(
    pool: &DbPool,
    carrier_id: &str,
    place_id: &str,
    now_unix: i64,
) -> anyhow::Result<Option<DeliveryResult>> {
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let Some(row) = tx.query_opt(
        "SELECT s.shipment_id,s.runtime_id,s.to_market_id,s.source_good_id,s.item_id,
                s.destination_place_id,s.quantity,g.display_name
         FROM world_market_shipments s
         JOIN world_goods g ON g.runtime_id=s.runtime_id AND g.source_good_id=s.source_good_id
         WHERE s.carrier_id=$1 AND s.status='in_transit' FOR UPDATE OF s",
        &[&carrier_id],
    )?
    else {
        tx.commit()?;
        return Ok(None);
    };
    let shipment_id: String = row.get(0);
    let runtime_id: String = row.get(1);
    let to_market_id: i32 = row.get(2);
    let source_good_id: i32 = row.get(3);
    let item_id: String = row.get(4);
    let destination_place_id: String = row.get(5);
    let quantity: i32 = row.get(6);
    let display_name: String = row.get(7);
    if place_id != destination_place_id {
        tx.execute(
            "UPDATE world_market_shipments SET current_place_id=$1,updated_at=$2
             WHERE shipment_id=$3",
            &[&place_id, &now_unix, &shipment_id],
        )?;
        tx.commit()?;
        return Ok(None);
    }
    let inventory_row = tx.query_one(
        "SELECT quantity,target_quantity,reference_price_mg
         FROM world_market_inventory
         WHERE runtime_id=$1 AND source_market_id=$2 AND source_good_id=$3 FOR UPDATE",
        &[&runtime_id, &to_market_id, &source_good_id],
    )?;
    let destination_before: i32 = inventory_row.get(0);
    let target: i32 = inventory_row.get(1);
    let reference_price: i32 = inventory_row.get(2);
    let destination_after = destination_before
        .checked_add(quantity)
        .ok_or_else(|| anyhow::anyhow!("目的市場庫存溢位"))?;
    let destination_price =
        super::world_market::price_for_stock(reference_price, destination_after, target);
    tx.execute(
        "UPDATE world_market_inventory SET quantity=$1,price_mg=$2,updated_at=$3
         WHERE runtime_id=$4 AND source_market_id=$5 AND source_good_id=$6",
        &[
            &destination_after,
            &destination_price,
            &now_unix,
            &runtime_id,
            &to_market_id,
            &source_good_id,
        ],
    )?;
    tx.execute(
        "UPDATE world_market_shipments SET status='delivered',current_place_id=$1,
           destination_quantity_before=$2,destination_quantity_after=$3,
           updated_at=$4,delivered_at=$4 WHERE shipment_id=$5",
        &[
            &place_id,
            &destination_before,
            &destination_after,
            &now_unix,
            &shipment_id,
        ],
    )?;
    tx.execute(
        "UPDATE resident_obligations SET status='completed',updated_at=$1
         WHERE obligation_id='shipment:'||$2",
        &[&now_unix, &shipment_id],
    )?;
    let action_id = super::living_world::record_action_in_tx(
        &mut tx,
        super::living_world::ActionDraft {
            actor_id: carrier_id,
            action_key: "work",
            target_ref: &shipment_id,
            place_id,
            preconditions: serde_json::json!({
                "shipment_id": shipment_id,
                "carrier_at_destination": true,
                "destination_quantity": destination_before
            }),
            effects: serde_json::json!({
                "shipment_id": shipment_id,
                "item_id": item_id,
                "quantity": quantity,
                "destination_quantity": {
                    "before": destination_before,
                    "after": destination_after
                }
            }),
            outcome: "shipment_delivered",
            source_ref: "runtime:market-logistics",
            occurred_at: now_unix,
        },
    )?;
    tx.commit()?;
    Ok(Some(DeliveryResult {
        shipment_id,
        carrier_id: carrier_id.to_string(),
        destination_place_id,
        item_id,
        display_name,
        quantity,
        destination_quantity_before: destination_before,
        destination_quantity_after: destination_after,
        action_id,
    }))
}

/// 將失去承運人的貨物標成滯留，並在同地找到真實居民時重新指派；貨物始終留在 shipment。
pub fn reconcile_one(pool: &DbPool, now_unix: i64) -> anyhow::Result<bool> {
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let broken = tx.query_opt(
        "SELECT s.shipment_id,s.carrier_id,COALESCE(e.place_id,s.current_place_id)
         FROM world_market_shipments s
         LEFT JOIN entities e ON e.id=s.carrier_id
         WHERE s.status='in_transit' AND (e.id IS NULL OR e.vit<=0)
         ORDER BY s.updated_at,s.shipment_id LIMIT 1 FOR UPDATE OF s",
        &[],
    )?;
    if let Some(row) = broken {
        let shipment_id: String = row.get(0);
        let carrier_id: Option<String> = row.get(1);
        let current_place_id: String = row.get(2);
        tx.execute(
            "UPDATE world_market_shipments SET status='stranded',carrier_id=NULL,
             current_place_id=$1,failure_reason='carrier_unavailable',updated_at=$2
             WHERE shipment_id=$3",
            &[&current_place_id, &now_unix, &shipment_id],
        )?;
        if let Some(carrier_id) = carrier_id {
            tx.execute(
                "UPDATE resident_obligations SET status='failed',updated_at=$1
                 WHERE obligation_id='shipment:'||$2 AND entity_id=$3",
                &[&now_unix, &shipment_id, &carrier_id],
            )?;
        }
        tx.commit()?;
        return Ok(true);
    }
    let Some(stranded) = tx.query_opt(
        "SELECT shipment_id,current_place_id,source_ref
         FROM world_market_shipments WHERE status='stranded'
         ORDER BY updated_at,shipment_id LIMIT 1 FOR UPDATE",
        &[],
    )?
    else {
        tx.commit()?;
        return Ok(false);
    };
    let shipment_id: String = stranded.get(0);
    let current_place_id: String = stranded.get(1);
    let source_ref: String = stranded.get(2);
    let Some(carrier) = tx.query_opt(
        "SELECT e.id FROM entities e
         LEFT JOIN entity_journeys j ON j.entity_id=e.id
         WHERE e.kind='npc' AND e.vit>0 AND e.place_id=$1
           AND COALESCE(j.planned_path,'[]')='[]'
           AND NOT EXISTS (
             SELECT 1 FROM world_market_shipments s
             WHERE s.carrier_id=e.id AND s.status='in_transit')
         ORDER BY abs(hashtextextended(e.id||':'||$2,0)),e.id
         LIMIT 1 FOR UPDATE OF e SKIP LOCKED",
        &[&current_place_id, &shipment_id],
    )?
    else {
        tx.commit()?;
        return Ok(false);
    };
    let carrier_id: String = carrier.get(0);
    tx.execute(
        "UPDATE world_market_shipments SET status='in_transit',carrier_id=$1,
         failure_reason='',updated_at=$2 WHERE shipment_id=$3",
        &[&carrier_id, &now_unix, &shipment_id],
    )?;
    upsert_delivery_obligation(&mut tx, &shipment_id, &carrier_id, &source_ref, now_unix)?;
    tx.commit()?;
    Ok(true)
}

pub fn status(client: &mut impl GenericClient) -> anyhow::Result<LogisticsStatus> {
    ensure_schema(client)?;
    let row = client.query_one(
        "SELECT COUNT(*)::bigint,
                COUNT(*) FILTER (WHERE origin_kind='market')::bigint,
                COUNT(*) FILTER (WHERE origin_kind='producer')::bigint,
                COUNT(*) FILTER (WHERE status='in_transit')::bigint,
                COUNT(*) FILTER (WHERE status='stranded')::bigint,
                COUNT(*) FILTER (WHERE status='delivered')::bigint,
                COUNT(*) FILTER (WHERE origin_quantity_before<>
                  origin_quantity_after+quantity)::bigint,
                COUNT(*) FILTER (WHERE status='delivered' AND
                  destination_quantity_after<>destination_quantity_before+quantity)::bigint
         FROM world_market_shipments",
        &[],
    )?;
    Ok(LogisticsStatus {
        total: row.get(0),
        market_origin: row.get(1),
        producer_origin: row.get(2),
        in_transit: row.get(3),
        stranded: row.get(4),
        delivered: row.get(5),
        invalid_origin_balances: row.get(6),
        invalid_destination_balances: row.get(7),
    })
}

pub fn producer_shipment_probe(
    client: &mut impl GenericClient,
) -> anyhow::Result<Option<ProducerShipmentProbe>> {
    ensure_schema(client)?;
    let row = client.query_opt(
        "SELECT s.shipment_id,s.carrier_id,s.origin_place_id,s.destination_place_id,
                s.current_place_id,s.status,g.display_name,s.quantity,e.place_id,
                e.current_activity,c.current_intent,c.destination_place_id,c.next_wake_at
         FROM world_market_shipments s
         JOIN world_goods g ON g.runtime_id=s.runtime_id AND g.source_good_id=s.source_good_id
         LEFT JOIN entities e ON e.id=s.carrier_id
         LEFT JOIN resident_continuity c ON c.entity_id=s.carrier_id
         WHERE s.origin_kind='producer'
         ORDER BY s.dispatched_at DESC,s.shipment_id DESC LIMIT 1",
        &[],
    )?;
    Ok(row.map(|row| ProducerShipmentProbe {
        shipment_id: row.get(0),
        carrier_id: row.get(1),
        origin_place_id: row.get(2),
        destination_place_id: row.get(3),
        current_place_id: row.get(4),
        status: row.get(5),
        display_name: row.get(6),
        quantity: row.get(7),
        carrier_place_id: row.get(8),
        carrier_activity: row.get(9),
        carrier_intent: row.get(10),
        carrier_destination_place_id: row.get(11),
        carrier_next_wake_at: row.get(12),
    }))
}

pub(crate) fn upsert_delivery_obligation(
    client: &mut impl GenericClient,
    shipment_id: &str,
    carrier_id: &str,
    source_ref: &str,
    now_unix: i64,
) -> anyhow::Result<()> {
    let obligation_id = format!("shipment:{shipment_id}");
    client.execute(
        "INSERT INTO resident_obligations
         (obligation_id,entity_id,obligation_kind,subject_ref,status,due_at,
          source_ref,created_at,updated_at)
         VALUES ($1,$2,'market_delivery',$3,'active',NULL,$4,$5,$5)
         ON CONFLICT(obligation_id) DO UPDATE SET entity_id=EXCLUDED.entity_id,
          status='active',source_ref=EXCLUDED.source_ref,updated_at=EXCLUDED.updated_at",
        &[
            &obligation_id,
            &carrier_id,
            &shipment_id,
            &source_ref,
            &now_unix,
        ],
    )?;
    Ok(())
}

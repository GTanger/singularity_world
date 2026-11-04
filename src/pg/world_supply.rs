//! Azgaar `burg -> market` 證據的實體上游供應。
//!
//! 來源 Burg 不直接升格為核心聚落；同一物理場所的來源交易會共用一個地方供應站。
//! 只有受派且親自在來源場所工作的居民，才能把有來源證據的商品交進同地市場，
//! 或把它裝成一批由自己承運的實體貨物。

use std::collections::{BTreeMap, BTreeSet};

use r2d2_postgres::postgres::{GenericClient, Transaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::world::author_economy::{WhiteTowerAuthorEconomy, runtime_good_item_id};
use crate::world::runtime_world::builtin_white_tower_runtime_world;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupplySeedReport {
    pub profiles: i64,
    pub producer_sites: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupplyDbStatus {
    pub profiles: i64,
    pub producer_sites: i64,
    pub assigned_workers: i64,
    pub workers_at_source: i64,
    pub funded_markets: i64,
    pub eligible_profiles: i64,
    pub eligible_workers_at_source: i64,
    pub waiting_workers: i64,
    pub supply_events: i64,
    pub local_events: i64,
    pub shipment_events: i64,
    pub invalid_ledger_rows: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EligibleSupplyWorkerProbe {
    pub entity_id: String,
    pub venue_id: String,
    pub source_place_id: String,
    pub market_place_id: String,
    pub source_deal_id: i32,
    pub travel_cost: i32,
    pub entity_kind: String,
    pub entity_vit: i32,
    pub current_intent: String,
    pub destination_place_id: Option<String>,
    pub next_wake_at: Option<i64>,
    pub current_activity: String,
    pub active_carrier_shipments: i64,
    pub active_deal_shipments: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpstreamSupplyOutput {
    pub supply_id: String,
    pub source_deal_id: i32,
    pub source_burg_id: i32,
    pub source_place_id: String,
    pub destination_market_id: i32,
    pub destination_place_id: String,
    pub source_good_id: i32,
    pub item_id: String,
    pub display_name: String,
    pub quantity: i32,
    pub delivery_kind: String,
    pub shipment_id: Option<String>,
    pub market_quantity_before: i32,
    pub market_quantity_after: i32,
    pub market_price_mg: i32,
    pub source_ref: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedSupply {
    runtime_id: String,
    source_deal_id: i32,
    source_burg_id: i32,
    source_place_id: String,
    destination_market_id: i32,
    destination_place_id: String,
    source_good_id: i32,
    item_id: String,
    pub(crate) display_name: String,
    evidence_units: f64,
    planned_path: String,
    travel_cost: i32,
    source_ref: String,
    market_quantity_before: i32,
    market_target_quantity: i32,
    market_initial_quantity: i32,
    market_reference_price_mg: i32,
    market_price_mg: i32,
    pub(crate) treasury_venue_id: String,
    pub(crate) treasury_mg_before: i32,
}

#[derive(Debug, Clone, PartialEq)]
struct CompiledSupplyProfile {
    runtime_id: String,
    source_deal_id: i32,
    source_burg_id: i32,
    source_place_id: String,
    destination_market_id: i32,
    destination_place_id: String,
    source_good_id: i32,
    item_id: String,
    evidence_units: f64,
    reference_price_mg: i32,
    planned_path: String,
    travel_cost: i32,
    source_ref: String,
}

pub fn ensure_schema(client: &mut impl GenericClient) -> anyhow::Result<()> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS world_supply_profiles (
            runtime_id TEXT NOT NULL,
            source_deal_id INTEGER NOT NULL,
            source_burg_id INTEGER NOT NULL,
            source_place_id TEXT NOT NULL,
            destination_market_id INTEGER NOT NULL,
            destination_place_id TEXT NOT NULL,
            source_good_id INTEGER NOT NULL,
            item_id TEXT NOT NULL,
            evidence_units DOUBLE PRECISION NOT NULL CHECK(evidence_units > 0),
            reference_price_mg INTEGER NOT NULL CHECK(reference_price_mg > 0),
            planned_path TEXT NOT NULL,
            travel_cost INTEGER NOT NULL CHECK(travel_cost >= 0),
            source_ref TEXT NOT NULL,
            active BOOLEAN NOT NULL,
            PRIMARY KEY(runtime_id,source_deal_id),
            CHECK(source_place_id=destination_place_id OR travel_cost>0)
        );
        CREATE INDEX IF NOT EXISTS idx_world_supply_profiles_source
        ON world_supply_profiles(runtime_id,active,source_place_id,source_good_id);
        CREATE INDEX IF NOT EXISTS idx_world_supply_profiles_destination
        ON world_supply_profiles(runtime_id,active,destination_market_id,source_good_id);

        CREATE TABLE IF NOT EXISTS world_upstream_supply_ledger (
            supply_id TEXT PRIMARY KEY,
            runtime_id TEXT NOT NULL,
            source_deal_id INTEGER NOT NULL,
            source_burg_id INTEGER NOT NULL,
            source_place_id TEXT NOT NULL,
            destination_market_id INTEGER NOT NULL,
            destination_place_id TEXT NOT NULL,
            source_good_id INTEGER NOT NULL,
            item_id TEXT NOT NULL,
            worker_id TEXT NOT NULL,
            work_id TEXT NOT NULL UNIQUE,
            work_period BIGINT NOT NULL,
            shipment_id TEXT,
            delivery_kind TEXT NOT NULL CHECK(delivery_kind IN ('local','shipment')),
            quantity INTEGER NOT NULL CHECK(quantity > 0),
            market_quantity_before INTEGER NOT NULL CHECK(market_quantity_before >= 0),
            market_quantity_after INTEGER NOT NULL CHECK(market_quantity_after >= 0),
            source_ref TEXT NOT NULL,
            occurred_at BIGINT NOT NULL,
            CHECK(
              (delivery_kind='local' AND shipment_id IS NULL
               AND market_quantity_before+quantity=market_quantity_after)
              OR
              (delivery_kind='shipment' AND shipment_id IS NOT NULL
               AND market_quantity_before=market_quantity_after)
            )
        );
        CREATE INDEX IF NOT EXISTS idx_world_upstream_supply_ledger_source
        ON world_upstream_supply_ledger(source_place_id,occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_world_upstream_supply_ledger_market
        ON world_upstream_supply_ledger(destination_market_id,occurred_at DESC);",
    )?;
    Ok(())
}

pub fn seed_counts(economy: &WhiteTowerAuthorEconomy) -> anyhow::Result<SupplySeedReport> {
    let profiles = compile_profiles(economy)?;
    let producer_sites = profiles
        .iter()
        .map(|profile| profile.source_place_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    Ok(SupplySeedReport {
        profiles: i64::try_from(profiles.len())?,
        producer_sites: i64::try_from(producer_sites)?,
    })
}

pub fn upsert_seed(
    client: &mut impl GenericClient,
    economy: &WhiteTowerAuthorEconomy,
    now_unix: i64,
) -> anyhow::Result<SupplySeedReport> {
    ensure_schema(client)?;
    let profiles = compile_profiles(economy)?;
    client.execute(
        "DELETE FROM world_supply_profiles WHERE runtime_id=$1",
        &[&economy.runtime_id],
    )?;
    let statement = client.prepare(
        "INSERT INTO world_supply_profiles
         (runtime_id,source_deal_id,source_burg_id,source_place_id,destination_market_id,
          destination_place_id,source_good_id,item_id,evidence_units,reference_price_mg,
          planned_path,travel_cost,source_ref,active)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,TRUE)",
    )?;
    let mut site_deal_counts = BTreeMap::<String, i32>::new();
    for profile in &profiles {
        client.execute(
            &statement,
            &[
                &profile.runtime_id,
                &profile.source_deal_id,
                &profile.source_burg_id,
                &profile.source_place_id,
                &profile.destination_market_id,
                &profile.destination_place_id,
                &profile.source_good_id,
                &profile.item_id,
                &profile.evidence_units,
                &profile.reference_price_mg,
                &profile.planned_path,
                &profile.travel_cost,
                &profile.source_ref,
            ],
        )?;
        let count = site_deal_counts
            .entry(profile.source_place_id.clone())
            .or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("地方供應證據數溢位"))?;
    }

    let world = builtin_white_tower_runtime_world()?;
    let place_names = world
        .places
        .iter()
        .map(|place| (place.id.as_str(), place.name.as_str()))
        .collect::<BTreeMap<_, _>>();
    for (place_id, evidence_deal_count) in &site_deal_counts {
        let venue_id = producer_venue_id(place_id)?;
        let raw_place_name = place_names
            .get(place_id.as_str())
            .copied()
            .ok_or_else(|| anyhow::anyhow!("供應來源沒有正式物理場所：{place_id}"))?;
        let place_name = super::living_world::peninsula_market_settlement_name(raw_place_name);
        let venue_name = format!("{place_name}地方供應站");
        let room_ids = serde_json::to_string(&[place_id.as_str()])?;
        client.execute(
            "INSERT INTO venues(id,name,room_ids,max_staff) VALUES ($1,$2,$3,1)
             ON CONFLICT(id) DO UPDATE SET name=EXCLUDED.name,room_ids=EXCLUDED.room_ids,
             max_staff=1",
            &[&venue_id, &venue_name, &room_ids],
        )?;
        let source_ref = format!("azgaar:burg-supply-place:{place_id}:employment");
        client.execute(
            "INSERT INTO world_work_sites
             (runtime_id,venue_id,site_kind,source_market_id,place_id,occupation_id,capacity,
              wage_mg,initial_treasury_mg,treasury_mg,evidence_deal_count,source_ref,updated_at)
             VALUES ($1,$2,'producer',0,$3,'地方供應人員',1,1,0,0,$4,$5,$6)
             ON CONFLICT(runtime_id,venue_id) DO UPDATE SET
              site_kind='producer',source_market_id=0,place_id=EXCLUDED.place_id,
              occupation_id=EXCLUDED.occupation_id,capacity=1,wage_mg=1,
              initial_treasury_mg=0,evidence_deal_count=EXCLUDED.evidence_deal_count,
              source_ref=EXCLUDED.source_ref,updated_at=EXCLUDED.updated_at",
            &[
                &economy.runtime_id,
                &venue_id,
                &place_id,
                &evidence_deal_count,
                &source_ref,
                &now_unix,
            ],
        )?;
    }

    let report = SupplySeedReport {
        profiles: i64::try_from(profiles.len())?,
        producer_sites: i64::try_from(site_deal_counts.len())?,
    };
    let loaded = status(client)?;
    anyhow::ensure!(
        loaded.profiles == report.profiles && loaded.producer_sites == report.producer_sites,
        "上游供應證據或地方供應站寫入不完整"
    );
    Ok(report)
}

pub fn status(client: &mut impl GenericClient) -> anyhow::Result<SupplyDbStatus> {
    ensure_schema(client)?;
    let row = client.query_one(
        "SELECT
          (SELECT COUNT(*)::bigint FROM world_supply_profiles WHERE active),
          (SELECT COUNT(*)::bigint FROM world_work_sites WHERE site_kind='producer'),
          (SELECT COUNT(*)::bigint
             FROM assignments a
             JOIN world_work_sites site ON site.venue_id=a.venue_id
             WHERE site.site_kind='producer'),
          (SELECT COUNT(*)::bigint
             FROM assignments a
             JOIN world_work_sites site ON site.venue_id=a.venue_id
             JOIN entities e ON e.id=a.entity_id AND e.place_id=site.place_id
             WHERE site.site_kind='producer'),
          (SELECT COUNT(*)::bigint FROM world_work_sites
             WHERE site_kind='market' AND treasury_mg>=1),
          (SELECT COUNT(*)::bigint
             FROM world_supply_profiles p
             JOIN world_market_inventory i ON i.runtime_id=p.runtime_id
                  AND i.source_market_id=p.destination_market_id
                  AND i.source_good_id=p.source_good_id
             JOIN world_work_sites sponsor ON sponsor.runtime_id=p.runtime_id
                  AND sponsor.site_kind='market'
                  AND sponsor.source_market_id=p.destination_market_id
             JOIN world_work_sites producer ON producer.runtime_id=p.runtime_id
                  AND producer.site_kind='producer'
                  AND producer.place_id=p.source_place_id
             WHERE p.active AND sponsor.treasury_mg>=producer.wage_mg
               AND i.quantity<GREATEST(i.target_quantity*2,i.initial_quantity+1,2)),
          (SELECT COUNT(DISTINCT e.id)::bigint
             FROM world_supply_profiles p
             JOIN world_market_inventory i ON i.runtime_id=p.runtime_id
                  AND i.source_market_id=p.destination_market_id
                  AND i.source_good_id=p.source_good_id
             JOIN world_work_sites sponsor ON sponsor.runtime_id=p.runtime_id
                  AND sponsor.site_kind='market'
                  AND sponsor.source_market_id=p.destination_market_id
             JOIN world_work_sites producer ON producer.runtime_id=p.runtime_id
                  AND producer.site_kind='producer'
                  AND producer.place_id=p.source_place_id
             JOIN assignments a ON a.venue_id=producer.venue_id
             JOIN entities e ON e.id=a.entity_id AND e.place_id=producer.place_id
             WHERE p.active AND sponsor.treasury_mg>=producer.wage_mg
               AND i.quantity<GREATEST(i.target_quantity*2,i.initial_quantity+1,2)),
          (SELECT COUNT(*)::bigint
             FROM assignments a
             JOIN world_work_sites site ON site.venue_id=a.venue_id
             JOIN entities e ON e.id=a.entity_id
             WHERE site.site_kind='producer'
               AND e.current_activity='供應工作暫停，正在等候市場需求或款項'),
          (SELECT COUNT(*)::bigint FROM world_upstream_supply_ledger),
          (SELECT COUNT(*)::bigint FROM world_upstream_supply_ledger
             WHERE delivery_kind='local'),
          (SELECT COUNT(*)::bigint FROM world_upstream_supply_ledger
             WHERE delivery_kind='shipment'),
          (SELECT COUNT(*)::bigint FROM world_upstream_supply_ledger
             WHERE NOT (
               (delivery_kind='local' AND shipment_id IS NULL
                AND market_quantity_before+quantity=market_quantity_after)
               OR
               (delivery_kind='shipment' AND shipment_id IS NOT NULL
                AND market_quantity_before=market_quantity_after)
             ))",
        &[],
    )?;
    Ok(SupplyDbStatus {
        profiles: row.get(0),
        producer_sites: row.get(1),
        assigned_workers: row.get(2),
        workers_at_source: row.get(3),
        funded_markets: row.get(4),
        eligible_profiles: row.get(5),
        eligible_workers_at_source: row.get(6),
        waiting_workers: row.get(7),
        supply_events: row.get(8),
        local_events: row.get(9),
        shipment_events: row.get(10),
        invalid_ledger_rows: row.get(11),
    })
}

pub fn eligible_worker_probe(
    client: &mut impl GenericClient,
) -> anyhow::Result<Option<EligibleSupplyWorkerProbe>> {
    ensure_schema(client)?;
    let row = client.query_opt(
        "SELECT a.entity_id,producer.venue_id,p.source_place_id,p.destination_place_id,
                p.source_deal_id,p.travel_cost,e.kind,e.vit,
                continuity.current_intent,continuity.destination_place_id,
                continuity.next_wake_at,e.current_activity,
                (SELECT COUNT(*)::bigint FROM world_market_shipments shipment
                   WHERE shipment.carrier_id=a.entity_id
                     AND shipment.status IN ('in_transit','stranded')),
                (SELECT COUNT(*)::bigint FROM world_market_shipments shipment
                   WHERE shipment.runtime_id=p.runtime_id
                     AND shipment.source_deal_id=p.source_deal_id
                     AND shipment.status IN ('in_transit','stranded'))
         FROM world_supply_profiles p
         JOIN world_market_inventory i ON i.runtime_id=p.runtime_id
              AND i.source_market_id=p.destination_market_id
              AND i.source_good_id=p.source_good_id
         JOIN world_work_sites sponsor ON sponsor.runtime_id=p.runtime_id
              AND sponsor.site_kind='market'
              AND sponsor.source_market_id=p.destination_market_id
         JOIN world_work_sites producer ON producer.runtime_id=p.runtime_id
              AND producer.site_kind='producer' AND producer.place_id=p.source_place_id
         JOIN assignments a ON a.venue_id=producer.venue_id
         JOIN entities e ON e.id=a.entity_id AND e.place_id=producer.place_id
         JOIN resident_continuity continuity ON continuity.entity_id=e.id
         WHERE p.active AND sponsor.treasury_mg>=producer.wage_mg
           AND i.quantity<GREATEST(i.target_quantity*2,i.initial_quantity+1,2)
         ORDER BY (p.source_place_id<>p.destination_place_id) DESC,
                  p.source_deal_id,a.entity_id LIMIT 1",
        &[],
    )?;
    Ok(row.map(|row| EligibleSupplyWorkerProbe {
        entity_id: row.get(0),
        venue_id: row.get(1),
        source_place_id: row.get(2),
        market_place_id: row.get(3),
        source_deal_id: row.get(4),
        travel_cost: row.get(5),
        entity_kind: row.get(6),
        entity_vit: row.get(7),
        current_intent: row.get(8),
        destination_place_id: row.get(9),
        next_wake_at: row.get(10),
        current_activity: row.get(11),
        active_carrier_shipments: row.get(12),
        active_deal_shipments: row.get(13),
    }))
}

pub(crate) fn prepare_shift_in_tx(
    tx: &mut Transaction<'_>,
    runtime_id: &str,
    source_place_id: &str,
    worker_id: &str,
    work_period: i64,
    wage_mg: i32,
) -> anyhow::Result<Option<PreparedSupply>> {
    let row = tx.query_opt(
        "SELECT p.runtime_id,p.source_deal_id,p.source_burg_id,p.source_place_id,
                p.destination_market_id,p.destination_place_id,p.source_good_id,p.item_id,
                g.display_name,p.evidence_units,p.planned_path,p.travel_cost,p.source_ref,
                i.quantity,i.target_quantity,i.initial_quantity,i.reference_price_mg,i.price_mg,
                sponsor.venue_id,sponsor.treasury_mg
         FROM world_supply_profiles p
         JOIN world_market_inventory i ON i.runtime_id=p.runtime_id
              AND i.source_market_id=p.destination_market_id
              AND i.source_good_id=p.source_good_id
         JOIN world_goods g ON g.runtime_id=p.runtime_id AND g.source_good_id=p.source_good_id
         JOIN world_work_sites sponsor ON sponsor.runtime_id=p.runtime_id
              AND sponsor.site_kind='market'
              AND sponsor.source_market_id=p.destination_market_id
         WHERE p.runtime_id=$1 AND p.source_place_id=$2 AND p.active
           AND sponsor.treasury_mg >= $5
           AND i.quantity < GREATEST(i.target_quantity*2,i.initial_quantity+1,2)
           AND NOT EXISTS (
             SELECT 1 FROM world_market_shipments shipment
             WHERE shipment.runtime_id=p.runtime_id
               AND shipment.source_deal_id=p.source_deal_id
               AND shipment.status IN ('in_transit','stranded'))
           AND NOT EXISTS (
             SELECT 1 FROM world_market_shipments shipment
             WHERE shipment.carrier_id=$3
               AND shipment.status IN ('in_transit','stranded'))
         ORDER BY
           (GREATEST(i.target_quantity*2,i.initial_quantity+1,2)-i.quantity)
             *p.evidence_units DESC,
           abs(hashtextextended($3||':'||$4::bigint::text||':'||
               p.source_deal_id::text,0)),p.source_deal_id
         LIMIT 1 FOR UPDATE OF i,sponsor SKIP LOCKED",
        &[
            &runtime_id,
            &source_place_id,
            &worker_id,
            &work_period,
            &wage_mg,
        ],
    )?;
    Ok(row.map(|row| PreparedSupply {
        runtime_id: row.get(0),
        source_deal_id: row.get(1),
        source_burg_id: row.get(2),
        source_place_id: row.get(3),
        destination_market_id: row.get(4),
        destination_place_id: row.get(5),
        source_good_id: row.get(6),
        item_id: row.get(7),
        display_name: row.get(8),
        evidence_units: row.get(9),
        planned_path: row.get(10),
        travel_cost: row.get(11),
        source_ref: row.get(12),
        market_quantity_before: row.get(13),
        market_target_quantity: row.get(14),
        market_initial_quantity: row.get(15),
        market_reference_price_mg: row.get(16),
        market_price_mg: row.get(17),
        treasury_venue_id: row.get(18),
        treasury_mg_before: row.get(19),
    }))
}

pub(crate) fn dispatch_in_tx(
    tx: &mut Transaction<'_>,
    prepared: PreparedSupply,
    worker_id: &str,
    work_id: &str,
    work_period: i64,
    now_unix: i64,
) -> anyhow::Result<UpstreamSupplyOutput> {
    let cap = prepared
        .market_target_quantity
        .saturating_mul(2)
        .max(prepared.market_initial_quantity.saturating_add(1))
        .max(2);
    let quantity = supply_quantity(
        prepared.evidence_units,
        cap - prepared.market_quantity_before,
    );
    anyhow::ensure!(quantity > 0, "目的市場已無可接收的供應量");

    let local = prepared.source_place_id == prepared.destination_place_id;
    let (delivery_kind, shipment_id, market_after, market_price_mg) = if local {
        let market_after = prepared
            .market_quantity_before
            .checked_add(quantity)
            .ok_or_else(|| anyhow::anyhow!("目的市場庫存溢位"))?;
        let price_mg = super::world_market::price_for_stock(
            prepared.market_reference_price_mg,
            market_after,
            prepared.market_target_quantity,
        );
        tx.execute(
            "UPDATE world_market_inventory SET quantity=$1,price_mg=$2,updated_at=$3
             WHERE runtime_id=$4 AND source_market_id=$5 AND source_good_id=$6",
            &[
                &market_after,
                &price_mg,
                &now_unix,
                &prepared.runtime_id,
                &prepared.destination_market_id,
                &prepared.source_good_id,
            ],
        )?;
        ("local", None, market_after, price_mg)
    } else {
        anyhow::ensure!(prepared.travel_cost > 0, "跨地供應缺少可通行的運輸成本");
        let shipment_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO world_market_shipments
             (shipment_id,runtime_id,origin_kind,source_deal_id,carrier_id,from_market_id,
              to_market_id,source_good_id,item_id,origin_place_id,destination_place_id,
              current_place_id,quantity,status,origin_quantity_before,origin_quantity_after,
              destination_quantity_before,destination_quantity_after,planned_path,travel_cost,
              failure_reason,source_ref,dispatched_at,updated_at,delivered_at)
             VALUES ($1,$2,'producer',$3,$4,0,$5,$6,$7,$8,$9,$8,$10,'in_transit',
                     $10,0,NULL,NULL,$11,$12,'',$13,$14,$14,NULL)",
            &[
                &shipment_id,
                &prepared.runtime_id,
                &prepared.source_deal_id,
                &worker_id,
                &prepared.destination_market_id,
                &prepared.source_good_id,
                &prepared.item_id,
                &prepared.source_place_id,
                &prepared.destination_place_id,
                &quantity,
                &prepared.planned_path,
                &prepared.travel_cost,
                &prepared.source_ref,
                &now_unix,
            ],
        )?;
        super::world_logistics::upsert_delivery_obligation(
            tx,
            &shipment_id,
            worker_id,
            &prepared.source_ref,
            now_unix,
        )?;
        let source_name = crate::db::get_room_name(&prepared.source_place_id).unwrap_or_default();
        let dest_name =
            crate::db::get_room_name(&prepared.destination_place_id).unwrap_or_default();
        let reason = format_venue_supply_dispatch_reason_and_advance_event_pack(
            &source_name,
            &dest_name,
            &prepared.item_id,
            &prepared.display_name,
        );
        tx.execute(
            "UPDATE resident_continuity SET current_intent='work',
               intent_reason=$5,
               destination_place_id=$1,blocked_by=NULL,
               next_wake_at=LEAST(COALESCE(next_wake_at,$2),$2),
               revision=revision+1,updated_at=$2,source_kind='runtime_upstream_supply',
               source_ref=$3 WHERE entity_id=$4",
            &[
                &prepared.destination_place_id,
                &now_unix,
                &prepared.source_ref,
                &worker_id,
                &reason,
            ],
        )?;
        (
            "shipment",
            Some(shipment_id),
            prepared.market_quantity_before,
            prepared.market_price_mg,
        )
    };

    let supply_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO world_upstream_supply_ledger
         (supply_id,runtime_id,source_deal_id,source_burg_id,source_place_id,
          destination_market_id,destination_place_id,source_good_id,item_id,worker_id,
          work_id,work_period,shipment_id,delivery_kind,quantity,market_quantity_before,
          market_quantity_after,source_ref,occurred_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)",
        &[
            &supply_id,
            &prepared.runtime_id,
            &prepared.source_deal_id,
            &prepared.source_burg_id,
            &prepared.source_place_id,
            &prepared.destination_market_id,
            &prepared.destination_place_id,
            &prepared.source_good_id,
            &prepared.item_id,
            &worker_id,
            &work_id,
            &work_period,
            &shipment_id,
            &delivery_kind,
            &quantity,
            &prepared.market_quantity_before,
            &market_after,
            &prepared.source_ref,
            &now_unix,
        ],
    )?;

    Ok(UpstreamSupplyOutput {
        supply_id,
        source_deal_id: prepared.source_deal_id,
        source_burg_id: prepared.source_burg_id,
        source_place_id: prepared.source_place_id,
        destination_market_id: prepared.destination_market_id,
        destination_place_id: prepared.destination_place_id,
        source_good_id: prepared.source_good_id,
        item_id: prepared.item_id,
        display_name: prepared.display_name,
        quantity,
        delivery_kind: delivery_kind.to_string(),
        shipment_id,
        market_quantity_before: prepared.market_quantity_before,
        market_quantity_after: market_after,
        market_price_mg,
        source_ref: prepared.source_ref,
    })
}

fn compile_profiles(
    economy: &WhiteTowerAuthorEconomy,
) -> anyhow::Result<Vec<CompiledSupplyProfile>> {
    let world = builtin_white_tower_runtime_world()?;
    let graph = world.graph()?;
    let markets = economy
        .markets
        .iter()
        .map(|market| (market.source_market_id, market))
        .collect::<BTreeMap<_, _>>();
    let allowed_goods = economy
        .goods
        .iter()
        .filter(|good| good.runtime_trade_allowed)
        .map(|good| good.source_good_id)
        .collect::<BTreeSet<_>>();
    let mut profiles = Vec::new();
    let mut deal_ids = BTreeSet::new();
    for deal in economy.trade_evidence.iter().filter(|deal| {
        deal.runtime_trade_allowed
            && deal.seller.source_type == "burg"
            && deal.buyer.source_type == "market"
    }) {
        anyhow::ensure!(
            deal.units.is_finite() && deal.units > 0.0,
            "上游供應數量無效：{}",
            deal.source_deal_id
        );
        anyhow::ensure!(
            deal_ids.insert(deal.source_deal_id),
            "上游供應交易 id 重複：{}",
            deal.source_deal_id
        );
        anyhow::ensure!(
            allowed_goods.contains(&deal.source_good_id),
            "上游供應引用禁售或不存在商品"
        );
        let market = markets
            .get(&deal.buyer.source_id)
            .ok_or_else(|| anyhow::anyhow!("上游供應目的市場不存在"))?;
        anyhow::ensure!(
            market.place_id == deal.buyer.place_id,
            "上游供應目的市場場所不一致"
        );
        let path = graph
            .find_path(&deal.seller.place_id, &deal.buyer.place_id)
            .ok_or_else(|| anyhow::anyhow!("上游供應沒有可通行路徑：{}", deal.source_deal_id))?;
        let travel_cost = path.windows(2).try_fold(0_i32, |total, pair| {
            let route = graph
                .route_to(&pair[0], &pair[1])
                .ok_or_else(|| anyhow::anyhow!("上游供應路徑引用不存在路線"))?;
            total
                .checked_add(route.travel_cost)
                .ok_or_else(|| anyhow::anyhow!("上游供應成本溢位"))
        })?;
        anyhow::ensure!(
            (deal.seller.place_id == deal.buyer.place_id && travel_cost == 0)
                || (deal.seller.place_id != deal.buyer.place_id && travel_cost > 0),
            "上游供應場所與運輸成本不一致"
        );
        profiles.push(CompiledSupplyProfile {
            runtime_id: economy.runtime_id.clone(),
            source_deal_id: i32::try_from(deal.source_deal_id)?,
            source_burg_id: i32::try_from(deal.seller.source_id)?,
            source_place_id: deal.seller.place_id.clone(),
            destination_market_id: i32::try_from(deal.buyer.source_id)?,
            destination_place_id: deal.buyer.place_id.clone(),
            source_good_id: i32::try_from(deal.source_good_id)?,
            item_id: runtime_good_item_id(deal.source_good_id),
            evidence_units: deal.units,
            reference_price_mg: rounded_price(deal.price)?,
            planned_path: serde_json::to_string(&path)?,
            travel_cost,
            source_ref: format!("azgaar:deal:{}:upstream-supply", deal.source_deal_id),
        });
    }
    Ok(profiles)
}

fn producer_venue_id(place_id: &str) -> anyhow::Result<String> {
    let suffix = place_id
        .strip_prefix("wt-physical-")
        .ok_or_else(|| anyhow::anyhow!("供應場所不是白塔正式物理區：{place_id}"))?;
    anyhow::ensure!(
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()),
        "供應場所 id 無法轉為穩定工作場所 id"
    );
    Ok(format!("wt-supply-{suffix}"))
}

fn rounded_price(value: f64) -> anyhow::Result<i32> {
    anyhow::ensure!(value.is_finite() && value > 0.0, "供應價格證據無效");
    Ok(value.round().clamp(1.0, f64::from(i32::MAX / 4)) as i32)
}

fn supply_quantity(evidence_units: f64, remaining_capacity: i32) -> i32 {
    (evidence_units.ceil().clamp(1.0, 3.0) as i32).min(remaining_capacity.max(0))
}

/// 依來源場所 `RoomVenueObject` 與貨物名稱組裝上游供應裝貨意向描述，並推進房間 `EventPack`／`ScriptPack`。
#[must_use]
pub fn format_venue_supply_dispatch_reason_and_advance_event_pack(
    source_room_name: &str,
    dest_room_name: &str,
    item_id: &str,
    display_name: &str,
) -> String {
    let src_trimmed = source_room_name.trim();
    if !src_trimmed.is_empty() {
        let object_hint = format!("{} {}", item_id.trim(), display_name.trim());
        let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
            src_trimmed,
            object_hint.trim(),
        );
    }
    let dest_label = if dest_room_name.trim().is_empty() {
        "鄰近市集"
    } else {
        dest_room_name.trim()
    };
    let item_label = display_name.trim();
    if let Some(venue) = crate::grid::resolve_any_room_venue_object(src_trimmed, "") {
        let zone_tag = if venue.has_counter_restricted_zone {
            "當值櫃檯內側"
        } else {
            "當值作業區"
        };
        if !item_label.is_empty() {
            return format!(
                "已於{}{}備妥【{}】，正將貨物運往{}",
                venue.venue_kind, zone_tag, item_label, dest_label
            );
        }
        return format!(
            "已於{}{}完成裝貨，正將貨物運往{}",
            venue.venue_kind, zone_tag, dest_label
        );
    }
    if !item_label.is_empty() {
        format!("已在供應地裝妥【{item_label}】，正將貨物運往{dest_label}")
    } else {
        format!("已在供應地裝貨，正將貨物運往{dest_label}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::author_economy::builtin_white_tower_author_economy;

    #[test]
    fn compiles_every_allowed_burg_supply_into_a_physical_profile() {
        let economy = builtin_white_tower_author_economy().expect("author economy");
        let profiles = compile_profiles(&economy).expect("supply profiles");
        let source_places = profiles
            .iter()
            .map(|profile| profile.source_place_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(profiles.len(), 4_976);
        assert_eq!(source_places.len(), 277);
        assert!(profiles.iter().all(|profile| {
            if profile.source_place_id == profile.destination_place_id {
                profile.travel_cost == 0
            } else {
                profile.travel_cost > 0
            }
        }));
    }

    #[test]
    fn supply_quantity_is_bounded_by_evidence_and_destination_capacity() {
        assert_eq!(supply_quantity(0.2, 9), 1);
        assert_eq!(supply_quantity(2.2, 9), 3);
        assert_eq!(supply_quantity(8.0, 2), 2);
        assert_eq!(supply_quantity(1.0, 0), 0);
    }

    #[test]
    fn producer_venue_ids_are_stable_and_do_not_promote_burg_ids() {
        assert_eq!(
            producer_venue_id("wt-physical-0042").unwrap(),
            "wt-supply-0042"
        );
        assert!(producer_venue_id("azgaar-burg-42").is_err());
    }

    #[test]
    fn test_format_venue_supply_dispatch_reason_and_advance_event_pack() {
        let reason = format_venue_supply_dispatch_reason_and_advance_event_pack(
            "老堰頭・水輪石磨工坊",
            "三水大食堂",
            "wt_cat_0388",
            "現磨溫米漿",
        );
        assert_eq!(
            reason,
            "已於水輪石磨工坊當值櫃檯內側備妥【現磨溫米漿】，正將貨物運往三水大食堂"
        );

        let fallback = format_venue_supply_dispatch_reason_and_advance_event_pack(
            "荒蕪礫石空地",
            "",
            "wt_cat_0001",
            "粗鹽",
        );
        assert_eq!(fallback, "已在供應地裝妥【粗鹽】，正將貨物運往鄰近市集");
    }
}

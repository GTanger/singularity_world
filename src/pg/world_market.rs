//! Azgaar 市場證據的可持續 runtime 投影。
//!
//! `world_market_goods` 永遠保留作者快照；本模組另建不會在重啟時回填的實際庫存、
//! 本地供需輪廓與跨市場物流線。任何增加、移動或消耗都必須另有帳本。

use std::collections::BTreeMap;

use r2d2_postgres::postgres::GenericClient;
use serde::{Deserialize, Serialize};

use crate::store::sql::DbPool;
use crate::world::author_economy::{AuthorGood, WhiteTowerAuthorEconomy, runtime_good_item_id};
use crate::world::runtime_world::builtin_white_tower_runtime_world;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketSeedReport {
    pub inventory_rows: i64,
    pub supply_profiles: i64,
    pub demand_profiles: i64,
    pub flow_lanes: i64,
    pub portable_goods: i64,
    pub consumable_goods: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketDbStatus {
    pub inventory_rows: i64,
    pub inventory_quantity: i64,
    pub supply_profiles: i64,
    pub demand_profiles: i64,
    pub flow_lanes: i64,
    pub portable_goods: i64,
    pub consumable_goods: i64,
    pub supply_events: i64,
    pub consumption_events: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketConservationStatus {
    pub initial_quantity: i64,
    pub supplied_quantity: i64,
    pub current_market_quantity: i64,
    pub in_transit_quantity: i64,
    pub entity_held_quantity: i64,
    pub consumed_quantity: i64,
    pub quantity_gap: i64,
    pub mismatched_goods: i64,
    pub invalid_supply_rows: i64,
    pub invalid_consumption_rows: i64,
    pub blocked_nonruntime_trades: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketOffer {
    pub source_market_id: i32,
    pub place_id: String,
    pub source_good_id: i32,
    pub item_id: String,
    pub display_name: String,
    pub source_unit: String,
    pub quantity: i32,
    pub target_quantity: i32,
    pub ask_mg: i32,
}

#[derive(Debug, Clone, Copy, Default)]
struct FlowProfile {
    supply_deals: i32,
    supply_units: f64,
    demand_deals: i32,
    demand_units: f64,
}

#[derive(Debug, Clone, Default)]
struct GoodBalance {
    initial: i64,
    supplied: i64,
    market: i64,
    in_transit: i64,
    held: i64,
    consumed: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct ActorInventoryEntry {
    item_id: String,
    qty: i32,
}

pub fn ensure_schema(client: &mut impl GenericClient) -> anyhow::Result<()> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS world_market_inventory (
            runtime_id TEXT NOT NULL,
            source_market_id INTEGER NOT NULL,
            source_good_id INTEGER NOT NULL,
            item_id TEXT NOT NULL,
            initial_quantity INTEGER NOT NULL CHECK(initial_quantity >= 0),
            quantity INTEGER NOT NULL CHECK(quantity >= 0),
            target_quantity INTEGER NOT NULL CHECK(target_quantity >= 0),
            reference_price_mg INTEGER NOT NULL CHECK(reference_price_mg > 0),
            price_mg INTEGER NOT NULL CHECK(price_mg > 0),
            local_supply_deals INTEGER NOT NULL CHECK(local_supply_deals >= 0),
            local_supply_units DOUBLE PRECISION NOT NULL CHECK(local_supply_units >= 0),
            local_demand_deals INTEGER NOT NULL CHECK(local_demand_deals >= 0),
            local_demand_units DOUBLE PRECISION NOT NULL CHECK(local_demand_units >= 0),
            portable BOOLEAN NOT NULL,
            consumable BOOLEAN NOT NULL,
            source_ref TEXT NOT NULL,
            updated_at BIGINT NOT NULL,
            PRIMARY KEY(runtime_id,source_market_id,source_good_id)
        );
        CREATE INDEX IF NOT EXISTS idx_world_market_inventory_shortage
        ON world_market_inventory(runtime_id,source_good_id,quantity,target_quantity);
        CREATE INDEX IF NOT EXISTS idx_world_market_inventory_item
        ON world_market_inventory(runtime_id,item_id,source_market_id);

        CREATE TABLE IF NOT EXISTS world_market_flow_lanes (
            runtime_id TEXT NOT NULL,
            source_deal_id INTEGER NOT NULL,
            from_market_id INTEGER NOT NULL,
            to_market_id INTEGER NOT NULL,
            source_good_id INTEGER NOT NULL,
            evidence_units DOUBLE PRECISION NOT NULL CHECK(evidence_units > 0),
            reference_price_mg INTEGER NOT NULL CHECK(reference_price_mg > 0),
            planned_path TEXT NOT NULL,
            travel_cost INTEGER NOT NULL CHECK(travel_cost > 0),
            source_ref TEXT NOT NULL,
            active BOOLEAN NOT NULL,
            PRIMARY KEY(runtime_id,source_deal_id),
            CHECK(from_market_id <> to_market_id)
        );
        CREATE INDEX IF NOT EXISTS idx_world_market_flow_lanes_dispatch
        ON world_market_flow_lanes(runtime_id,active,source_good_id,from_market_id,to_market_id);

        CREATE TABLE IF NOT EXISTS world_market_supply_ledger (
            supply_id TEXT PRIMARY KEY,
            runtime_id TEXT NOT NULL,
            source_market_id INTEGER NOT NULL,
            source_good_id INTEGER NOT NULL,
            item_id TEXT NOT NULL,
            worker_id TEXT NOT NULL,
            work_id TEXT NOT NULL UNIQUE,
            work_period BIGINT NOT NULL,
            quantity INTEGER NOT NULL CHECK(quantity > 0),
            market_quantity_before INTEGER NOT NULL CHECK(market_quantity_before >= 0),
            market_quantity_after INTEGER NOT NULL CHECK(market_quantity_after >= 0),
            source_rule TEXT NOT NULL,
            source_ref TEXT NOT NULL,
            occurred_at BIGINT NOT NULL,
            CHECK(market_quantity_before + quantity = market_quantity_after)
        );
        CREATE INDEX IF NOT EXISTS idx_world_market_supply_ledger_market
        ON world_market_supply_ledger(source_market_id,occurred_at DESC);

        CREATE TABLE IF NOT EXISTS world_market_adoptions (
            runtime_id TEXT PRIMARY KEY,
            activated_at BIGINT NOT NULL
        );",
    )?;
    Ok(())
}

/// 只在第一次採用時建立實際庫存；之後重啟只更新來源輪廓與價格基準，不回填數量。
pub fn upsert_seed(
    client: &mut impl GenericClient,
    economy: &WhiteTowerAuthorEconomy,
    now_unix: i64,
) -> anyhow::Result<MarketSeedReport> {
    ensure_schema(client)?;
    super::world_trade::ensure_schema(client)?;
    super::world_logistics::ensure_schema(client)?;
    client.execute(
        "INSERT INTO world_market_adoptions(runtime_id,activated_at) VALUES ($1,$2)
         ON CONFLICT(runtime_id) DO NOTHING",
        &[&economy.runtime_id, &now_unix],
    )?;
    upsert_item_definitions(client, economy)?;
    let flow = flow_profiles(economy)?;
    let inventory_statement = client.prepare(
        "INSERT INTO world_market_inventory
         (runtime_id,source_market_id,source_good_id,item_id,initial_quantity,quantity,
          target_quantity,reference_price_mg,price_mg,local_supply_deals,
          local_supply_units,local_demand_deals,local_demand_units,portable,consumable,
          source_ref,updated_at)
         VALUES ($1,$2,$3,$4,$5,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
         ON CONFLICT(runtime_id,source_market_id,source_good_id) DO UPDATE SET
          item_id=EXCLUDED.item_id,target_quantity=EXCLUDED.target_quantity,
          reference_price_mg=EXCLUDED.reference_price_mg,
          price_mg=GREATEST(1,LEAST(EXCLUDED.reference_price_mg*4,
            GREATEST((EXCLUDED.reference_price_mg+1)/2,
              ROUND(EXCLUDED.reference_price_mg::numeric*(EXCLUDED.target_quantity+1)::numeric/
                    (world_market_inventory.quantity+1)::numeric)::integer))),
          local_supply_deals=EXCLUDED.local_supply_deals,
          local_supply_units=EXCLUDED.local_supply_units,
          local_demand_deals=EXCLUDED.local_demand_deals,
          local_demand_units=EXCLUDED.local_demand_units,portable=EXCLUDED.portable,
          consumable=EXCLUDED.consumable,source_ref=EXCLUDED.source_ref,
          updated_at=EXCLUDED.updated_at",
    )?;
    for market in &economy.markets {
        let market_id = i32::try_from(market.source_market_id)?;
        for quote in &market.quotes {
            if !quote.runtime_trade_allowed {
                continue;
            }
            let good = economy
                .goods
                .iter()
                .find(|good| good.source_good_id == quote.source_good_id)
                .ok_or_else(|| anyhow::anyhow!("市場報價引用不存在商品"))?;
            let good_id = i32::try_from(good.source_good_id)?;
            let profile = flow
                .get(&(market.source_market_id, good.source_good_id))
                .copied()
                .unwrap_or_default();
            let initial_quantity = rounded_quantity(quote.stock)?;
            let target_quantity = target_quantity(initial_quantity, profile);
            let reference_price_mg = rounded_price(quote.price)?;
            let price_mg = price_for_stock(reference_price_mg, initial_quantity, target_quantity);
            let portable = is_portable(good);
            let consumable = is_consumable(good) && portable;
            let item_id = runtime_good_item_id(good.source_good_id);
            let source_ref = format!(
                "azgaar:market:{}:good:{}:operational",
                market.source_market_id, good.source_good_id
            );
            client.execute(
                &inventory_statement,
                &[
                    &economy.runtime_id,
                    &market_id,
                    &good_id,
                    &item_id,
                    &initial_quantity,
                    &target_quantity,
                    &reference_price_mg,
                    &price_mg,
                    &profile.supply_deals,
                    &profile.supply_units,
                    &profile.demand_deals,
                    &profile.demand_units,
                    &portable,
                    &consumable,
                    &source_ref,
                    &now_unix,
                ],
            )?;
        }
    }
    upsert_flow_lanes(client, economy)?;
    let status = status(client)?;
    let expected_inventory = i64::try_from(
        economy.markets.len()
            * economy
                .goods
                .iter()
                .filter(|good| good.runtime_trade_allowed)
                .count(),
    )?;
    anyhow::ensure!(
        status.inventory_rows == expected_inventory,
        "實際市場庫存沒有覆蓋 54 個市場的 70 種可用商品"
    );
    anyhow::ensure!(status.flow_lanes == 478, "市場間 478 條來源流向未完整接通");
    Ok(MarketSeedReport {
        inventory_rows: status.inventory_rows,
        supply_profiles: status.supply_profiles,
        demand_profiles: status.demand_profiles,
        flow_lanes: status.flow_lanes,
        portable_goods: status.portable_goods,
        consumable_goods: status.consumable_goods,
    })
}

pub fn status(client: &mut impl GenericClient) -> anyhow::Result<MarketDbStatus> {
    ensure_schema(client)?;
    let upstream = super::world_supply::status(client)?;
    let row = client.query_one(
        "SELECT
          (SELECT COUNT(*)::bigint FROM world_market_inventory),
          (SELECT COALESCE(SUM(quantity),0)::bigint FROM world_market_inventory),
          (SELECT COUNT(*)::bigint FROM world_market_inventory WHERE local_supply_deals>0),
          (SELECT COUNT(*)::bigint FROM world_market_inventory WHERE local_demand_deals>0),
          (SELECT COUNT(*)::bigint FROM world_market_flow_lanes WHERE active),
          (SELECT COUNT(*)::bigint FROM world_market_inventory WHERE portable),
          (SELECT COUNT(*)::bigint FROM world_market_inventory WHERE consumable),
          (SELECT COUNT(*)::bigint FROM world_market_supply_ledger),
          (SELECT COUNT(*)::bigint FROM world_consumption_ledger)",
        &[],
    )?;
    Ok(MarketDbStatus {
        inventory_rows: row.get(0),
        inventory_quantity: row.get(1),
        supply_profiles: row.get(2),
        demand_profiles: row.get(3),
        flow_lanes: row.get(4),
        portable_goods: row.get(5),
        consumable_goods: row.get(6),
        supply_events: row
            .get::<_, i64>(7)
            .checked_add(upstream.supply_events)
            .ok_or_else(|| anyhow::anyhow!("市場供應事件總數溢位"))?,
        consumption_events: row.get(8),
    })
}

/// 驗證商品不會因成交、運送、消耗或重啟而憑空增減。
///
/// 每種商品皆須滿足：初始市場量 + 有帳本的供應 = 現有市場量 + 在途量 +
/// 人物持有量 + 已消耗量。
pub fn conservation_status(
    client: &mut impl GenericClient,
) -> anyhow::Result<MarketConservationStatus> {
    ensure_schema(client)?;
    super::world_trade::ensure_schema(client)?;
    super::world_logistics::ensure_schema(client)?;
    super::world_supply::ensure_schema(client)?;
    let mut balances = BTreeMap::<String, GoodBalance>::new();
    for row in client.query(
        "SELECT item_id,SUM(initial_quantity)::bigint,SUM(quantity)::bigint
         FROM world_market_inventory GROUP BY item_id",
        &[],
    )? {
        let item_id: String = row.get(0);
        let balance = balances.entry(item_id).or_default();
        balance.initial = row.get(1);
        balance.market = row.get(2);
    }
    for row in client.query(
        "SELECT item_id,SUM(quantity)::bigint FROM world_market_supply_ledger GROUP BY item_id",
        &[],
    )? {
        balances.entry(row.get(0)).or_default().supplied = row.get(1);
    }
    for row in client.query(
        "SELECT item_id,SUM(quantity)::bigint
         FROM world_upstream_supply_ledger GROUP BY item_id",
        &[],
    )? {
        let balance = balances.entry(row.get(0)).or_default();
        balance.supplied = balance
            .supplied
            .checked_add(row.get(1))
            .ok_or_else(|| anyhow::anyhow!("上游供應總量溢位"))?;
    }
    for row in client.query(
        "SELECT item_id,SUM(quantity)::bigint FROM world_market_shipments
         WHERE status IN ('in_transit','stranded','failed') GROUP BY item_id",
        &[],
    )? {
        balances.entry(row.get(0)).or_default().in_transit = row.get(1);
    }
    for row in client.query(
        "SELECT item_id,SUM(quantity)::bigint FROM world_consumption_ledger GROUP BY item_id",
        &[],
    )? {
        balances.entry(row.get(0)).or_default().consumed = row.get(1);
    }
    for row in client.query(
        "SELECT inventory FROM entities WHERE kind IN ('npc','player')",
        &[],
    )? {
        let raw: String = row.get(0);
        let entries: Vec<ActorInventoryEntry> = if raw.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&raw)
                .map_err(|error| anyhow::anyhow!("人物背包不是有效 JSON：{error}"))?
        };
        for entry in entries.into_iter().filter(|entry| {
            entry.qty > 0 && crate::world::author_economy::is_runtime_good_item(&entry.item_id)
        }) {
            let balance = balances.entry(entry.item_id).or_default();
            balance.held = balance
                .held
                .checked_add(i64::from(entry.qty))
                .ok_or_else(|| anyhow::anyhow!("人物持有商品總量溢位"))?;
        }
    }
    let mut result = MarketConservationStatus {
        initial_quantity: 0,
        supplied_quantity: 0,
        current_market_quantity: 0,
        in_transit_quantity: 0,
        entity_held_quantity: 0,
        consumed_quantity: 0,
        quantity_gap: 0,
        mismatched_goods: 0,
        invalid_supply_rows: client
            .query_one(
                "SELECT
                   (SELECT COUNT(*)::bigint FROM world_market_supply_ledger
                    WHERE market_quantity_before+quantity<>market_quantity_after)
                   +
                   (SELECT COUNT(*)::bigint FROM world_upstream_supply_ledger
                    WHERE NOT (
                      (delivery_kind='local' AND shipment_id IS NULL
                       AND market_quantity_before+quantity=market_quantity_after)
                      OR
                      (delivery_kind='shipment' AND shipment_id IS NOT NULL
                       AND market_quantity_before=market_quantity_after)
                    ))",
                &[],
            )?
            .get(0),
        invalid_consumption_rows: client
            .query_one(
                "SELECT COUNT(*)::bigint FROM world_consumption_ledger
                 WHERE inventory_quantity_before<>inventory_quantity_after+quantity",
                &[],
            )?
            .get(0),
        blocked_nonruntime_trades: client
            .query_one(
                "SELECT COUNT(*)::bigint FROM world_trade_ledger t
                 JOIN world_market_adoptions a ON a.runtime_id=$1
                 WHERE t.occurred_at>=a.activated_at AND t.item_id NOT LIKE 'wt-good-%'",
                &[&crate::world::runtime_world::WHITE_TOWER_RUNTIME_ID],
            )?
            .get(0),
    };
    for balance in balances.values() {
        result.initial_quantity += balance.initial;
        result.supplied_quantity += balance.supplied;
        result.current_market_quantity += balance.market;
        result.in_transit_quantity += balance.in_transit;
        result.entity_held_quantity += balance.held;
        result.consumed_quantity += balance.consumed;
        let gap = balance.initial + balance.supplied
            - balance.market
            - balance.in_transit
            - balance.held
            - balance.consumed;
        result.quantity_gap += gap;
        result.mismatched_goods += i64::from(gap != 0);
    }
    Ok(result)
}

/// 市場工作者替同地人物提出一筆實際庫存報價；舊 `mat_*` 背包不參與。
pub fn offer_for_operator(
    pool: &DbPool,
    operator_id: &str,
    buyer_id: &str,
    now_unix: i64,
) -> anyhow::Result<Option<MarketOffer>> {
    let mut conn = pool.get()?;
    let row = conn.query_opt(
        "SELECT w.source_market_id,w.place_id,i.source_good_id,i.item_id,g.display_name,
                g.source_unit,i.quantity,i.target_quantity,i.price_mg
         FROM assignments a
         JOIN world_work_sites w ON w.venue_id=a.venue_id
         JOIN entities operator ON operator.id=a.entity_id AND operator.place_id=w.place_id
         JOIN entities buyer ON buyer.id=$2 AND buyer.place_id=w.place_id
         JOIN world_market_inventory i ON i.runtime_id=w.runtime_id
              AND i.source_market_id=w.source_market_id
         JOIN world_goods g ON g.runtime_id=i.runtime_id AND g.source_good_id=i.source_good_id
         WHERE a.entity_id=$1 AND w.site_kind='market'
           AND operator.kind='npc' AND operator.vit>0
           AND buyer.kind IN ('npc','player') AND buyer.vit>0
           AND i.portable AND i.quantity>0
           AND NOT EXISTS (
             SELECT 1 FROM world_market_shipments s
             WHERE s.carrier_id=operator.id AND s.status IN ('in_transit','stranded'))
         ORDER BY abs(hashtextextended($1||':'||$2||':'||i.source_good_id::text||':'||
                   ($3::bigint/300)::text,0)),i.source_good_id
         LIMIT 1",
        &[&operator_id, &buyer_id, &now_unix],
    )?;
    Ok(row.map(|row| MarketOffer {
        source_market_id: row.get(0),
        place_id: row.get(1),
        source_good_id: row.get(2),
        item_id: row.get(3),
        display_name: row.get(4),
        source_unit: row.get(5),
        quantity: row.get(6),
        target_quantity: row.get(7),
        ask_mg: row.get(8),
    }))
}

/// 找出買方此刻同地、確實能從實際庫存報價的市場經手人。
///
/// 玩家層會把大量陌生人摺成「人群」；這份批次查詢讓仍可互動的經手人保留
/// 一張人物卡，而不必逐人試報價，也不會因公開互動角色而洩漏私名。
pub fn available_operator_ids_for_buyer(
    pool: &DbPool,
    buyer_id: &str,
) -> anyhow::Result<std::collections::HashSet<String>> {
    let mut conn = pool.get()?;
    let rows = conn.query(
        "SELECT DISTINCT operator.id
         FROM assignments a
         JOIN world_work_sites w ON w.venue_id=a.venue_id
         JOIN entities operator ON operator.id=a.entity_id AND operator.place_id=w.place_id
         JOIN entities buyer ON buyer.id=$1 AND buyer.place_id=w.place_id
         JOIN world_market_inventory i ON i.runtime_id=w.runtime_id
              AND i.source_market_id=w.source_market_id
         WHERE w.site_kind='market' AND operator.kind='npc' AND operator.vit>0
           AND buyer.kind IN ('npc','player') AND buyer.vit>0
           AND i.portable AND i.quantity>0
           AND NOT EXISTS (
             SELECT 1 FROM world_market_shipments s
             WHERE s.carrier_id=operator.id AND s.status IN ('in_transit','stranded'))",
        &[&buyer_id],
    )?;
    Ok(rows.into_iter().map(|row| row.get(0)).collect())
}

#[must_use]
pub fn price_for_stock(reference_price_mg: i32, quantity: i32, target_quantity: i32) -> i32 {
    let reference = reference_price_mg.max(1);
    let numerator = i64::from(reference) * i64::from(target_quantity.max(0) + 1);
    let denominator = i64::from(quantity.max(0) + 1);
    let rounded = (numerator + denominator / 2) / denominator;
    let floor = i64::from((reference + 1) / 2);
    let ceiling = i64::from(reference).saturating_mul(4);
    i32::try_from(rounded.clamp(floor, ceiling).max(1)).unwrap_or(i32::MAX)
}

fn upsert_item_definitions(
    client: &mut impl GenericClient,
    economy: &WhiteTowerAuthorEconomy,
) -> anyhow::Result<()> {
    let statement = client.prepare(
        "INSERT INTO items
         (id,name,slot,item_type,weight,stackable,denomination,description,
          vit_bonus,dex_bonus,atk_bonus)
         VALUES ($1,$2,'','commodity',$3,1,0,$4,0,0,0)
         ON CONFLICT(id) DO UPDATE SET name=EXCLUDED.name,item_type=EXCLUDED.item_type,
           weight=EXCLUDED.weight,stackable=EXCLUDED.stackable,
           description=EXCLUDED.description",
    )?;
    for good in economy
        .goods
        .iter()
        .filter(|good| good.runtime_trade_allowed)
    {
        let item_id = runtime_good_item_id(good.source_good_id);
        let weight = if is_portable(good) { 1.0_f64 } else { 25.0_f64 };
        let description = format!(
            "白塔半島市場流通的{}；每份沿用既有市場單位結算。",
            good.display_name
        );
        client.execute(
            &statement,
            &[&item_id, &good.display_name, &weight, &description],
        )?;
    }
    Ok(())
}

fn flow_profiles(
    economy: &WhiteTowerAuthorEconomy,
) -> anyhow::Result<BTreeMap<(u32, u32), FlowProfile>> {
    let mut profiles = BTreeMap::<(u32, u32), FlowProfile>::new();
    for deal in &economy.trade_evidence {
        if !deal.runtime_trade_allowed {
            continue;
        }
        if deal.seller.source_type == "burg" && deal.buyer.source_type == "market" {
            let profile = profiles
                .entry((deal.buyer.source_id, deal.source_good_id))
                .or_default();
            profile.supply_deals = profile
                .supply_deals
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("供應證據數溢位"))?;
            profile.supply_units += deal.units;
        } else if deal.seller.source_type == "market" && deal.buyer.source_type == "burg" {
            let profile = profiles
                .entry((deal.seller.source_id, deal.source_good_id))
                .or_default();
            profile.demand_deals = profile
                .demand_deals
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("需求證據數溢位"))?;
            profile.demand_units += deal.units;
        }
    }
    Ok(profiles)
}

fn upsert_flow_lanes(
    client: &mut impl GenericClient,
    economy: &WhiteTowerAuthorEconomy,
) -> anyhow::Result<()> {
    let world = builtin_white_tower_runtime_world()?;
    let graph = world.graph()?;
    let statement = client.prepare(
        "INSERT INTO world_market_flow_lanes
         (runtime_id,source_deal_id,from_market_id,to_market_id,source_good_id,
          evidence_units,reference_price_mg,planned_path,travel_cost,source_ref,active)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,TRUE)
         ON CONFLICT(runtime_id,source_deal_id) DO UPDATE SET
          from_market_id=EXCLUDED.from_market_id,to_market_id=EXCLUDED.to_market_id,
          source_good_id=EXCLUDED.source_good_id,evidence_units=EXCLUDED.evidence_units,
          reference_price_mg=EXCLUDED.reference_price_mg,
          planned_path=EXCLUDED.planned_path,travel_cost=EXCLUDED.travel_cost,
          source_ref=EXCLUDED.source_ref,active=TRUE",
    )?;
    for deal in economy.trade_evidence.iter().filter(|deal| {
        deal.runtime_trade_allowed
            && deal.seller.source_type == "market"
            && deal.buyer.source_type == "market"
            && deal.seller.source_id != deal.buyer.source_id
    }) {
        let path = graph
            .find_path(&deal.seller.place_id, &deal.buyer.place_id)
            .ok_or_else(|| anyhow::anyhow!("市場流向沒有可通行路徑：{}", deal.source_deal_id))?;
        let travel_cost = path.windows(2).try_fold(0_i32, |total, pair| {
            let route = graph
                .route_to(&pair[0], &pair[1])
                .ok_or_else(|| anyhow::anyhow!("市場流向路徑引用不存在路線"))?;
            total
                .checked_add(route.travel_cost)
                .ok_or_else(|| anyhow::anyhow!("市場流向成本溢位"))
        })?;
        anyhow::ensure!(travel_cost > 0, "市場流向成本必須為正");
        let source_deal_id = i32::try_from(deal.source_deal_id)?;
        let from_market_id = i32::try_from(deal.seller.source_id)?;
        let to_market_id = i32::try_from(deal.buyer.source_id)?;
        let source_good_id = i32::try_from(deal.source_good_id)?;
        let reference_price_mg = rounded_price(deal.price)?;
        let planned_path = serde_json::to_string(&path)?;
        let source_ref = format!("azgaar:deal:{}:market-lane", deal.source_deal_id);
        client.execute(
            &statement,
            &[
                &economy.runtime_id,
                &source_deal_id,
                &from_market_id,
                &to_market_id,
                &source_good_id,
                &deal.units,
                &reference_price_mg,
                &planned_path,
                &travel_cost,
                &source_ref,
            ],
        )?;
    }
    Ok(())
}

fn rounded_quantity(value: f64) -> anyhow::Result<i32> {
    anyhow::ensure!(value.is_finite() && value >= 0.0, "市場庫存證據無效");
    Ok(value.round().clamp(0.0, f64::from(i32::MAX)) as i32)
}

fn rounded_price(value: f64) -> anyhow::Result<i32> {
    anyhow::ensure!(value.is_finite() && value > 0.0, "市場價格證據無效");
    Ok(value.round().clamp(1.0, f64::from(i32::MAX / 4)) as i32)
}

fn target_quantity(initial_quantity: i32, profile: FlowProfile) -> i32 {
    let stock_anchor = (f64::from(initial_quantity) * 0.75).round() as i32;
    let demand_anchor = if profile.demand_deals > 0 {
        (profile.demand_units / f64::from(profile.demand_deals))
            .ceil()
            .clamp(1.0, 25.0) as i32
    } else {
        0
    };
    stock_anchor.max(demand_anchor).max(i32::from(
        profile.supply_deals > 0 || profile.demand_deals > 0,
    ))
}

fn is_portable(good: &AuthorGood) -> bool {
    !good.tags.iter().any(|tag| tag == "currency")
        && !matches!(
            good.source_unit.as_str(),
            "head" | "ship" | "cannon" | "wagon" | "wain" | "pallet" | "pile"
        )
}

fn is_consumable(good: &AuthorGood) -> bool {
    good.tags.iter().any(|tag| tag == "food") || matches!(good.source_good_id, 69..=71)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortage_raises_price_and_surplus_lowers_it_with_bounds() {
        assert_eq!(price_for_stock(10, 10, 10), 10);
        assert!(price_for_stock(10, 0, 10) > 10);
        assert!(price_for_stock(10, 30, 10) < 10);
        assert_eq!(price_for_stock(10, 0, 1000), 40);
        assert_eq!(price_for_stock(10, 1000, 0), 5);
    }

    #[test]
    fn target_uses_existing_stock_and_observed_demand_without_forcing_every_good() {
        assert_eq!(target_quantity(100, FlowProfile::default()), 75);
        assert_eq!(target_quantity(0, FlowProfile::default()), 0);
        assert_eq!(
            target_quantity(
                0,
                FlowProfile {
                    demand_deals: 2,
                    demand_units: 5.0,
                    ..FlowProfile::default()
                }
            ),
            3
        );
    }

    #[test]
    fn personal_offers_exclude_bulk_transport_units_and_currency() -> anyhow::Result<()> {
        let economy = crate::world::author_economy::builtin_white_tower_author_economy()?;
        let ships = economy
            .goods
            .iter()
            .find(|good| good.source_good_id == 53)
            .expect("ships retained");
        let soap = economy
            .goods
            .iter()
            .find(|good| good.source_good_id == 70)
            .expect("soap retained");
        let coins = economy
            .goods
            .iter()
            .find(|good| good.source_good_id == 62)
            .expect("coins retained as source evidence");
        assert!(!is_portable(ships));
        assert!(!is_portable(coins));
        assert!(is_portable(soap));
        assert!(is_consumable(soap));
        Ok(())
    }
}

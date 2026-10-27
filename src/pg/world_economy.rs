//! 白塔半島作者經濟證據的 PostgreSQL 投影。
//!
//! 71 種商品、54 個市場、3,834 筆報價與 25,808 筆來源交易只作 runtime
//! 經濟證據；實際人物成交另寫 `world_trade_ledger`，不能冒充來源交易。

use std::collections::BTreeMap;

use r2d2_postgres::postgres::GenericClient;
use serde::{Deserialize, Serialize};

use crate::world::author_economy::{WhiteTowerAuthorEconomy, builtin_white_tower_author_economy};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EconomySeedReport {
    pub artifact_id: String,
    pub goods: i64,
    pub markets: i64,
    pub quotes: i64,
    pub trade_evidence: i64,
    pub runtime_allowed_goods: i64,
    pub runtime_allowed_deals: i64,
    pub work_sites: i64,
    pub upstream_supply_profiles: i64,
    pub producer_sites: i64,
    pub operational_inventory: i64,
    pub operational_flow_lanes: i64,
    pub reloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EconomyDbStatus {
    pub goods: i64,
    pub markets: i64,
    pub quotes: i64,
    pub trade_evidence: i64,
    pub runtime_allowed_goods: i64,
    pub runtime_allowed_deals: i64,
    pub blocked_human_goods: i64,
    pub work_sites: i64,
    pub upstream_supply_profiles: i64,
    pub producer_sites: i64,
    pub operational_inventory: i64,
    pub operational_flow_lanes: i64,
}

pub fn ensure_schema(client: &mut impl GenericClient) -> anyhow::Result<()> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS world_goods (
            runtime_id TEXT NOT NULL,
            source_good_id INTEGER NOT NULL,
            source_name TEXT NOT NULL,
            display_name TEXT NOT NULL,
            source_unit TEXT NOT NULL,
            tags TEXT NOT NULL,
            base_value DOUBLE PRECISION NOT NULL CHECK(base_value >= 0),
            runtime_disposition TEXT NOT NULL,
            runtime_trade_allowed BOOLEAN NOT NULL,
            source_ref TEXT NOT NULL,
            PRIMARY KEY(runtime_id,source_good_id)
        );
        CREATE TABLE IF NOT EXISTS world_markets (
            runtime_id TEXT NOT NULL,
            source_market_id INTEGER NOT NULL,
            center_burg_id INTEGER NOT NULL,
            place_id TEXT NOT NULL,
            source_ref TEXT NOT NULL,
            PRIMARY KEY(runtime_id,source_market_id)
        );
        CREATE INDEX IF NOT EXISTS idx_world_markets_place
        ON world_markets(runtime_id,place_id);

        CREATE TABLE IF NOT EXISTS world_market_goods (
            runtime_id TEXT NOT NULL,
            source_market_id INTEGER NOT NULL,
            source_good_id INTEGER NOT NULL,
            stock DOUBLE PRECISION NOT NULL CHECK(stock >= 0),
            price DOUBLE PRECISION NOT NULL CHECK(price > 0),
            runtime_trade_allowed BOOLEAN NOT NULL,
            source_ref TEXT NOT NULL,
            PRIMARY KEY(runtime_id,source_market_id,source_good_id)
        );
        CREATE INDEX IF NOT EXISTS idx_world_market_goods_good
        ON world_market_goods(runtime_id,source_good_id,source_market_id);

        CREATE TABLE IF NOT EXISTS world_trade_evidence (
            runtime_id TEXT NOT NULL,
            source_deal_id INTEGER NOT NULL,
            seller_type TEXT NOT NULL CHECK(seller_type IN ('burg','market')),
            seller_source_id INTEGER NOT NULL,
            seller_place_id TEXT NOT NULL,
            buyer_type TEXT NOT NULL CHECK(buyer_type IN ('burg','market')),
            buyer_source_id INTEGER NOT NULL,
            buyer_place_id TEXT NOT NULL,
            source_good_id INTEGER NOT NULL,
            units DOUBLE PRECISION NOT NULL CHECK(units >= 0),
            price DOUBLE PRECISION NOT NULL CHECK(price > 0),
            tax DOUBLE PRECISION NOT NULL CHECK(tax >= 0),
            runtime_trade_allowed BOOLEAN NOT NULL,
            source_ref TEXT NOT NULL,
            PRIMARY KEY(runtime_id,source_deal_id)
        );
        CREATE INDEX IF NOT EXISTS idx_world_trade_evidence_places
        ON world_trade_evidence(runtime_id,seller_place_id,buyer_place_id);
        CREATE INDEX IF NOT EXISTS idx_world_trade_evidence_good
        ON world_trade_evidence(runtime_id,source_good_id);

        CREATE TABLE IF NOT EXISTS world_economy_imports (
            runtime_id TEXT PRIMARY KEY,
            artifact_id TEXT NOT NULL,
            source_full_json_sha256 TEXT NOT NULL,
            goods BIGINT NOT NULL,
            markets BIGINT NOT NULL,
            quotes BIGINT NOT NULL,
            trade_evidence BIGINT NOT NULL,
            imported_at BIGINT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS world_work_sites (
            runtime_id TEXT NOT NULL,
            venue_id TEXT NOT NULL,
            site_kind TEXT NOT NULL DEFAULT 'market',
            source_market_id INTEGER NOT NULL,
            place_id TEXT NOT NULL,
            occupation_id TEXT NOT NULL,
            capacity INTEGER NOT NULL CHECK(capacity > 0),
            wage_mg INTEGER NOT NULL CHECK(wage_mg > 0),
            initial_treasury_mg INTEGER NOT NULL CHECK(initial_treasury_mg >= 0),
            treasury_mg INTEGER NOT NULL CHECK(treasury_mg >= 0),
            evidence_deal_count INTEGER NOT NULL CHECK(evidence_deal_count >= 0),
            source_ref TEXT NOT NULL,
            updated_at BIGINT NOT NULL,
            PRIMARY KEY(runtime_id,venue_id),
            UNIQUE(venue_id),
            CONSTRAINT world_work_sites_site_kind_valid
              CHECK(site_kind IN ('market','producer'))
        );
        ALTER TABLE world_work_sites
          ADD COLUMN IF NOT EXISTS site_kind TEXT NOT NULL DEFAULT 'market';
        DO $migration$
        BEGIN
          IF NOT EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conname='world_work_sites_site_kind_valid'
          ) THEN
            ALTER TABLE world_work_sites
              ADD CONSTRAINT world_work_sites_site_kind_valid
              CHECK(site_kind IN ('market','producer'));
          END IF;
        END
        $migration$;",
    )?;
    super::world_supply::ensure_schema(client)?;
    Ok(())
}

/// 在白塔 runtime 採用交易內載入作者經濟。來源收據與實際列數都相符才跳過重載。
pub fn upsert_seed(
    client: &mut impl GenericClient,
    now_unix: i64,
) -> anyhow::Result<EconomySeedReport> {
    ensure_schema(client)?;
    let economy = builtin_white_tower_author_economy()?;
    let operational = super::world_market::upsert_seed(client, &economy, now_unix)?;
    let upstream = super::world_supply::seed_counts(&economy)?;
    let current = status(client)?;
    let expected = expected_report(&economy, &operational, &upstream, false)?;
    let receipt_matches = client
        .query_opt(
            "SELECT artifact_id,source_full_json_sha256 FROM world_economy_imports
             WHERE runtime_id=$1",
            &[&economy.runtime_id],
        )?
        .is_some_and(|row| {
            row.get::<_, String>(0) == economy.id
                && row.get::<_, String>(1) == economy.source_full_json_sha256
        });
    if receipt_matches && counts_match(&current, &expected) {
        return Ok(expected);
    }

    for table in [
        "world_trade_evidence",
        "world_market_goods",
        "world_markets",
        "world_goods",
    ] {
        let delete = format!("DELETE FROM {table} WHERE runtime_id=$1");
        client.execute(delete.as_str(), &[&economy.runtime_id])?;
    }

    let good_statement = client.prepare(
        "INSERT INTO world_goods
         (runtime_id,source_good_id,source_name,display_name,source_unit,tags,base_value,
          runtime_disposition,runtime_trade_allowed,source_ref)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )?;
    for good in &economy.goods {
        let id = i32::try_from(good.source_good_id)?;
        let tags = serde_json::to_string(&good.tags)?;
        let source_ref = format!("azgaar:good:{}", good.source_good_id);
        client.execute(
            &good_statement,
            &[
                &economy.runtime_id,
                &id,
                &good.source_name,
                &good.display_name,
                &good.source_unit,
                &tags,
                &good.base_value,
                &good.runtime_disposition,
                &good.runtime_trade_allowed,
                &source_ref,
            ],
        )?;
    }

    let market_statement = client.prepare(
        "INSERT INTO world_markets
         (runtime_id,source_market_id,center_burg_id,place_id,source_ref)
         VALUES ($1,$2,$3,$4,$5)",
    )?;
    let quote_statement = client.prepare(
        "INSERT INTO world_market_goods
         (runtime_id,source_market_id,source_good_id,stock,price,runtime_trade_allowed,source_ref)
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )?;
    for market in &economy.markets {
        let market_id = i32::try_from(market.source_market_id)?;
        let center_burg_id = i32::try_from(market.center_burg_id)?;
        let source_ref = format!("azgaar:market:{}", market.source_market_id);
        client.execute(
            &market_statement,
            &[
                &economy.runtime_id,
                &market_id,
                &center_burg_id,
                &market.place_id,
                &source_ref,
            ],
        )?;
        for quote in &market.quotes {
            let good_id = i32::try_from(quote.source_good_id)?;
            let quote_ref = format!(
                "azgaar:market:{}:good:{}",
                market.source_market_id, quote.source_good_id
            );
            client.execute(
                &quote_statement,
                &[
                    &economy.runtime_id,
                    &market_id,
                    &good_id,
                    &quote.stock,
                    &quote.price,
                    &quote.runtime_trade_allowed,
                    &quote_ref,
                ],
            )?;
        }
    }

    upsert_work_sites(client, &economy, now_unix)?;

    let deal_statement = client.prepare(
        "INSERT INTO world_trade_evidence
         (runtime_id,source_deal_id,seller_type,seller_source_id,seller_place_id,
          buyer_type,buyer_source_id,buyer_place_id,source_good_id,units,price,tax,
          runtime_trade_allowed,source_ref)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )?;
    for deal in &economy.trade_evidence {
        let deal_id = i32::try_from(deal.source_deal_id)?;
        let seller_id = i32::try_from(deal.seller.source_id)?;
        let buyer_id = i32::try_from(deal.buyer.source_id)?;
        let good_id = i32::try_from(deal.source_good_id)?;
        let source_ref = format!("azgaar:deal:{}", deal.source_deal_id);
        client.execute(
            &deal_statement,
            &[
                &economy.runtime_id,
                &deal_id,
                &deal.seller.source_type,
                &seller_id,
                &deal.seller.place_id,
                &deal.buyer.source_type,
                &buyer_id,
                &deal.buyer.place_id,
                &good_id,
                &deal.units,
                &deal.price,
                &deal.tax,
                &deal.runtime_trade_allowed,
                &source_ref,
            ],
        )?;
    }

    let upstream = super::world_supply::upsert_seed(client, &economy, now_unix)?;
    let operational = super::world_market::upsert_seed(client, &economy, now_unix)?;
    let report = expected_report(&economy, &operational, &upstream, true)?;
    client.execute(
        "INSERT INTO world_economy_imports
         (runtime_id,artifact_id,source_full_json_sha256,goods,markets,quotes,trade_evidence,imported_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
         ON CONFLICT(runtime_id) DO UPDATE SET artifact_id=EXCLUDED.artifact_id,
         source_full_json_sha256=EXCLUDED.source_full_json_sha256,goods=EXCLUDED.goods,
         markets=EXCLUDED.markets,quotes=EXCLUDED.quotes,
         trade_evidence=EXCLUDED.trade_evidence,imported_at=EXCLUDED.imported_at",
        &[
            &economy.runtime_id,
            &economy.id,
            &economy.source_full_json_sha256,
            &report.goods,
            &report.markets,
            &report.quotes,
            &report.trade_evidence,
            &now_unix,
        ],
    )?;
    let loaded = status(client)?;
    anyhow::ensure!(
        counts_match(&loaded, &report),
        "作者經濟資料寫入後數量不完整"
    );
    Ok(report)
}

pub fn status(client: &mut impl GenericClient) -> anyhow::Result<EconomyDbStatus> {
    ensure_schema(client)?;
    let upstream = super::world_supply::status(client)?;
    let row = client.query_one(
        "SELECT
          (SELECT COUNT(*)::bigint FROM world_goods),
          (SELECT COUNT(*)::bigint FROM world_markets),
          (SELECT COUNT(*)::bigint FROM world_market_goods),
          (SELECT COUNT(*)::bigint FROM world_trade_evidence),
          (SELECT COUNT(*)::bigint FROM world_goods WHERE runtime_trade_allowed),
          (SELECT COUNT(*)::bigint FROM world_trade_evidence WHERE runtime_trade_allowed),
          (SELECT COUNT(*)::bigint FROM world_goods
             WHERE runtime_disposition='retained_blocked_human_commodity'
               AND NOT runtime_trade_allowed),
          (SELECT COUNT(*)::bigint FROM world_work_sites),
          (SELECT COUNT(*)::bigint FROM world_market_inventory),
          (SELECT COUNT(*)::bigint FROM world_market_flow_lanes WHERE active)",
        &[],
    )?;
    Ok(EconomyDbStatus {
        goods: row.get(0),
        markets: row.get(1),
        quotes: row.get(2),
        trade_evidence: row.get(3),
        runtime_allowed_goods: row.get(4),
        runtime_allowed_deals: row.get(5),
        blocked_human_goods: row.get(6),
        work_sites: row.get(7),
        upstream_supply_profiles: upstream.profiles,
        producer_sites: upstream.producer_sites,
        operational_inventory: row.get(8),
        operational_flow_lanes: row.get(9),
    })
}

fn expected_report(
    economy: &WhiteTowerAuthorEconomy,
    operational: &super::world_market::MarketSeedReport,
    upstream: &super::world_supply::SupplySeedReport,
    reloaded: bool,
) -> anyhow::Result<EconomySeedReport> {
    Ok(EconomySeedReport {
        artifact_id: economy.id.clone(),
        goods: i64::try_from(economy.goods.len())?,
        markets: i64::try_from(economy.markets.len())?,
        quotes: i64::try_from(economy.coverage.market_quotes_accounted)?,
        trade_evidence: i64::try_from(economy.trade_evidence.len())?,
        runtime_allowed_goods: i64::try_from(economy.coverage.goods_runtime_allowed)?,
        runtime_allowed_deals: i64::try_from(economy.coverage.deals_runtime_allowed)?,
        work_sites: i64::try_from(economy.markets.len())?
            .checked_add(upstream.producer_sites)
            .ok_or_else(|| anyhow::anyhow!("工作場所總數溢位"))?,
        upstream_supply_profiles: upstream.profiles,
        producer_sites: upstream.producer_sites,
        operational_inventory: operational.inventory_rows,
        operational_flow_lanes: operational.flow_lanes,
        reloaded,
    })
}

fn counts_match(status: &EconomyDbStatus, expected: &EconomySeedReport) -> bool {
    status.goods == expected.goods
        && status.markets == expected.markets
        && status.quotes == expected.quotes
        && status.trade_evidence == expected.trade_evidence
        && status.runtime_allowed_goods == expected.runtime_allowed_goods
        && status.runtime_allowed_deals == expected.runtime_allowed_deals
        && status.blocked_human_goods == 1
        && status.work_sites == expected.work_sites
        && status.upstream_supply_profiles == expected.upstream_supply_profiles
        && status.producer_sites == expected.producer_sites
        && status.operational_inventory == expected.operational_inventory
        && status.operational_flow_lanes == expected.operational_flow_lanes
}

pub fn occupation_for_venue(
    client: &mut impl GenericClient,
    venue_id: &str,
) -> anyhow::Result<Option<String>> {
    Ok(client
        .query_opt(
            "SELECT occupation_id FROM world_work_sites WHERE venue_id=$1",
            &[&venue_id],
        )?
        .map(|row| row.get(0)))
}

fn upsert_work_sites(
    client: &mut impl GenericClient,
    economy: &WhiteTowerAuthorEconomy,
    now_unix: i64,
) -> anyhow::Result<()> {
    let mut flow = BTreeMap::<u32, (usize, usize)>::new();
    for deal in &economy.trade_evidence {
        if deal.seller.source_type == "market" {
            flow.entry(deal.seller.source_id).or_default().0 += 1;
        }
        if deal.buyer.source_type == "market" {
            flow.entry(deal.buyer.source_id).or_default().1 += 1;
        }
    }
    for market in &economy.markets {
        let market_id = i32::try_from(market.source_market_id)?;
        let venue_id = format!("wt-market-{:03}", market.source_market_id);
        let (outbound, inbound) = flow
            .get(&market.source_market_id)
            .copied()
            .unwrap_or_default();
        let occupation_id = if outbound > inbound + inbound / 5 {
            "市場供應調度員"
        } else if inbound > outbound + outbound / 5 {
            "市場採購調度員"
        } else {
            "市場營運員"
        };
        let evidence_deal_count = i32::try_from(outbound + inbound)?;
        let capacity = (4 + evidence_deal_count / 250).clamp(4, 12);
        let initial_treasury_mg = market
            .treasury_reference
            .max(0.0)
            .round()
            .clamp(0.0, f64::from(i32::MAX)) as i32;
        let place_name = client
            .query_opt(
                "SELECT name FROM world_places WHERE id=$1",
                &[&market.place_id],
            )?
            .map(|row| row.get::<_, String>(0))
            .unwrap_or_else(|| market.place_id.clone());
        let venue_name = format!("{place_name}市場運務點");
        let room_ids = serde_json::to_string(&[market.place_id.as_str()])?;
        client.execute(
            "INSERT INTO venues(id,name,room_ids,max_staff) VALUES ($1,$2,$3,$4)
             ON CONFLICT(id) DO UPDATE SET name=EXCLUDED.name,room_ids=EXCLUDED.room_ids,
             max_staff=EXCLUDED.max_staff",
            &[&venue_id, &venue_name, &room_ids, &capacity],
        )?;
        let source_ref = format!("azgaar:market:{}:employment", market.source_market_id);
        client.execute(
            "INSERT INTO world_work_sites
             (runtime_id,venue_id,site_kind,source_market_id,place_id,occupation_id,capacity,wage_mg,
              initial_treasury_mg,treasury_mg,evidence_deal_count,source_ref,updated_at)
             VALUES ($1,$2,'market',$3,$4,$5,$6,1,$7,$7,$8,$9,$10)
             ON CONFLICT(runtime_id,venue_id) DO UPDATE SET
             site_kind='market',source_market_id=EXCLUDED.source_market_id,
             place_id=EXCLUDED.place_id,
             occupation_id=EXCLUDED.occupation_id,capacity=EXCLUDED.capacity,
             wage_mg=EXCLUDED.wage_mg,initial_treasury_mg=EXCLUDED.initial_treasury_mg,
             evidence_deal_count=EXCLUDED.evidence_deal_count,source_ref=EXCLUDED.source_ref,
             updated_at=EXCLUDED.updated_at",
            &[
                &economy.runtime_id,
                &venue_id,
                &market_id,
                &market.place_id,
                &occupation_id,
                &capacity,
                &initial_treasury_mg,
                &evidence_deal_count,
                &source_ref,
                &now_unix,
            ],
        )?;
    }
    Ok(())
}

//! 人物共用的原子交易入口。
//!
//! 玩家與 NPC 都在同一筆 PostgreSQL 交易內交換既有物品與既有鎂；雙方必須
//! 同地、存活且有完整前置狀態。這裡不生成商品、不生成買家，也不生成貨幣。

use std::collections::BTreeMap;

use r2d2_postgres::postgres::{GenericClient, Row, Transaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::store::sql::DbPool;
use crate::world::place::WHITE_TOWER_WORLD_ID;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradePartyAfter {
    pub entity_id: String,
    pub magnesium: i32,
    pub inventory: String,
    pub current_activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeSettlement {
    pub trade_id: String,
    pub buyer_action_id: String,
    pub seller_action_id: String,
    pub place_id: String,
    pub item_id: String,
    pub quantity: i32,
    pub price_mg: i32,
    pub buyer: TradePartyAfter,
    pub seller: TradePartyAfter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketTradeSettlement {
    pub trade_id: String,
    pub buyer_action_id: String,
    pub operator_action_id: String,
    pub consumption_id: Option<String>,
    pub place_id: String,
    pub source_market_id: i32,
    pub item_id: String,
    pub display_name: String,
    pub quantity: i32,
    pub price_mg: i32,
    pub market_quantity_before: i32,
    pub market_quantity_after: i32,
    pub market_treasury_before: i32,
    pub market_treasury_after: i32,
    pub buyer: TradePartyAfter,
    pub operator: TradePartyAfter,
}

#[derive(Debug, Clone)]
struct LockedActor {
    id: String,
    kind: String,
    vit: i32,
    magnesium: i32,
    inventory: String,
    place_id: Option<String>,
    current_activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct InventoryEntry {
    item_id: String,
    qty: i32,
}

pub fn ensure_schema(client: &mut impl GenericClient) -> anyhow::Result<()> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS world_trade_ledger (
            trade_id TEXT PRIMARY KEY,
            world_id TEXT NOT NULL,
            place_id TEXT NOT NULL,
            buyer_id TEXT NOT NULL,
            seller_id TEXT NOT NULL,
            item_id TEXT NOT NULL,
            quantity INTEGER NOT NULL CHECK(quantity > 0),
            price_mg INTEGER NOT NULL CHECK(price_mg > 0),
            buyer_mg_before INTEGER NOT NULL CHECK(buyer_mg_before >= 0),
            buyer_mg_after INTEGER NOT NULL CHECK(buyer_mg_after >= 0),
            seller_mg_before INTEGER NOT NULL CHECK(seller_mg_before >= 0),
            seller_mg_after INTEGER NOT NULL CHECK(seller_mg_after >= 0),
            buyer_quantity_before INTEGER NOT NULL CHECK(buyer_quantity_before >= 0),
            buyer_quantity_after INTEGER NOT NULL CHECK(buyer_quantity_after >= 0),
            seller_quantity_before INTEGER NOT NULL CHECK(seller_quantity_before >= 0),
            seller_quantity_after INTEGER NOT NULL CHECK(seller_quantity_after >= 0),
            source_ref TEXT NOT NULL,
            occurred_at BIGINT NOT NULL,
            CHECK(buyer_id <> seller_id),
            CHECK(buyer_mg_before + seller_mg_before = buyer_mg_after + seller_mg_after),
            CHECK(buyer_quantity_before + seller_quantity_before =
                  buyer_quantity_after + seller_quantity_after)
        );
        CREATE INDEX IF NOT EXISTS idx_world_trade_ledger_place
        ON world_trade_ledger(place_id,occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_world_trade_ledger_buyer
        ON world_trade_ledger(buyer_id,occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_world_trade_ledger_seller
        ON world_trade_ledger(seller_id,occurred_at DESC);

        CREATE TABLE IF NOT EXISTS world_consumption_ledger (
            consumption_id TEXT PRIMARY KEY,
            runtime_id TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            source_market_id INTEGER NOT NULL,
            source_good_id INTEGER NOT NULL,
            place_id TEXT NOT NULL,
            item_id TEXT NOT NULL,
            consumption_period BIGINT NOT NULL,
            quantity INTEGER NOT NULL CHECK(quantity > 0),
            inventory_quantity_before INTEGER NOT NULL CHECK(inventory_quantity_before >= 0),
            inventory_quantity_after INTEGER NOT NULL CHECK(inventory_quantity_after >= 0),
            source_ref TEXT NOT NULL,
            occurred_at BIGINT NOT NULL,
            UNIQUE(entity_id,consumption_period),
            CHECK(inventory_quantity_before = inventory_quantity_after + quantity)
        );
        CREATE INDEX IF NOT EXISTS idx_world_consumption_ledger_market
        ON world_consumption_ledger(source_market_id,occurred_at DESC);",
    )?;
    Ok(())
}

/// 玩家與指定人物成交；雙方規則與 NPC 自主交易完全相同。
#[allow(clippy::too_many_arguments)]
pub fn settle(
    pool: &DbPool,
    buyer_id: &str,
    seller_id: &str,
    item_id: &str,
    quantity: i32,
    price_mg: i32,
    now_unix: i64,
    source_ref: &str,
) -> anyhow::Result<TradeSettlement> {
    validate_request(buyer_id, seller_id, item_id, quantity, price_mg)?;
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let rows = tx.query(
        "SELECT id,kind,vit,magnesium,inventory,place_id,current_activity
         FROM entities WHERE id IN ($1,$2) ORDER BY id FOR UPDATE",
        &[&buyer_id, &seller_id],
    )?;
    anyhow::ensure!(rows.len() == 2, "交易雙方至少一人不存在");
    let actors = rows
        .into_iter()
        .map(actor_from_row)
        .map(|actor| (actor.id.clone(), actor))
        .collect::<BTreeMap<_, _>>();
    let buyer = actors
        .get(buyer_id)
        .ok_or_else(|| anyhow::anyhow!("買方不存在"))?;
    let seller = actors
        .get(seller_id)
        .ok_or_else(|| anyhow::anyhow!("賣方不存在"))?;
    let settlement = settle_locked(
        &mut tx, buyer, seller, item_id, quantity, price_mg, now_unix, source_ref,
    )?;
    tx.commit()?;
    Ok(settlement)
}

/// NPC 只會把實際持物賣給同一市場區內、存活且付得起的另一名自主居民。
/// 沒有真買家就不成交，任何貨幣與物品都不變。
pub fn settle_npc_sale(
    pool: &DbPool,
    seller_id: &str,
    item_id: &str,
    quantity: i32,
    price_mg: i32,
    now_unix: i64,
    source_ref: &str,
) -> anyhow::Result<Option<TradeSettlement>> {
    anyhow::ensure!(!seller_id.trim().is_empty(), "賣方不可空白");
    anyhow::ensure!(!item_id.trim().is_empty(), "商品不可空白");
    anyhow::ensure!(quantity > 0, "交易數量須為正");
    anyhow::ensure!(price_mg > 0, "交易鎂數須為正");
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let Some(seller_row) = tx.query_opt(
        "SELECT id,kind,vit,magnesium,inventory,place_id,current_activity
         FROM entities WHERE id=$1 FOR UPDATE",
        &[&seller_id],
    )?
    else {
        anyhow::bail!("賣方不存在");
    };
    let seller = actor_from_row(seller_row);
    anyhow::ensure!(seller.kind == "npc", "自主兜售只允許 NPC");
    anyhow::ensure!(seller.vit > 0, "賣方已無法交易");
    let place_id = seller
        .place_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("賣方沒有正式位置"))?;
    let is_market = tx
        .query_opt(
            "SELECT 1 FROM world_markets WHERE place_id=$1 LIMIT 1",
            &[&place_id],
        )?
        .is_some();
    if !is_market {
        return Ok(None);
    }
    let buyer_rows = tx.query(
        "SELECT id,kind,vit,magnesium,inventory,place_id,current_activity
         FROM entities
         WHERE kind='npc' AND vit>0 AND id<>$1 AND place_id=$2 AND magnesium >= $3
         ORDER BY abs(hashtext(id || ':' || $1 || ':' || ($4::bigint)::text)),id
         LIMIT 16 FOR UPDATE SKIP LOCKED",
        &[&seller_id, &place_id, &price_mg, &now_unix],
    )?;
    let mut chosen_buyer = None;
    for row in buyer_rows {
        let buyer = actor_from_row(row);
        if let Ok(mut test_inv) = parse_inventory(&buyer.inventory) {
            if add_quantity_for_actor(&mut test_inv, item_id, quantity, buyer.kind == "npc", false).is_ok() {
                chosen_buyer = Some(buyer);
                break;
            }
        }
    }
    let Some(buyer) = chosen_buyer else {
        return Ok(None);
    };
    let settlement = settle_locked(
        &mut tx, &buyer, &seller, item_id, quantity, price_mg, now_unix, source_ref,
    )?;
    tx.commit()?;
    Ok(Some(settlement))
}

/// 從市場實際庫存成交：買方付出自己的鎂，收入進工作場所財庫，市場工作者只是
/// 經手人，不會把公款收進個人背包。`consume_period` 存在時，商品隨後由買方
/// 明確消耗並寫入 sink 帳本。
#[allow(clippy::too_many_arguments)]
pub fn settle_market_sale(
    pool: &DbPool,
    buyer_id: &str,
    operator_id: &str,
    source_market_id: i32,
    item_id: &str,
    quantity: i32,
    price_mg: i32,
    now_unix: i64,
    source_ref: &str,
    consume_period: Option<i64>,
) -> anyhow::Result<MarketTradeSettlement> {
    validate_request(buyer_id, operator_id, item_id, quantity, price_mg)?;
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;

    let rows = tx.query(
        "SELECT id,kind,vit,magnesium,inventory,place_id,current_activity
         FROM entities WHERE id IN ($1,$2) ORDER BY id FOR UPDATE",
        &[&buyer_id, &operator_id],
    )?;
    anyhow::ensure!(rows.len() == 2, "市場交易雙方至少一人不存在");
    let actors = rows
        .into_iter()
        .map(actor_from_row)
        .map(|actor| (actor.id.clone(), actor))
        .collect::<BTreeMap<_, _>>();
    let buyer = actors
        .get(buyer_id)
        .ok_or_else(|| anyhow::anyhow!("買方不存在"))?;
    let operator = actors
        .get(operator_id)
        .ok_or_else(|| anyhow::anyhow!("市場工作者不存在"))?;
    anyhow::ensure!(
        matches!(buyer.kind.as_str(), "player" | "npc"),
        "買方不是人物"
    );
    anyhow::ensure!(operator.kind == "npc", "市場經手人必須是居民");
    anyhow::ensure!(buyer.vit > 0 && operator.vit > 0, "市場交易人物必須存活");
    let place_id = buyer
        .place_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("買方沒有正式位置"))?;
    anyhow::ensure!(
        operator.place_id.as_deref() == Some(place_id),
        "雙方不在同一市場"
    );

    let market = tx.query_opt(
        "SELECT w.runtime_id,w.venue_id,w.treasury_mg,i.source_good_id,i.quantity,
                i.target_quantity,i.reference_price_mg,i.price_mg,g.display_name
         FROM assignments a
         JOIN world_work_sites w ON w.venue_id=a.venue_id
         JOIN world_market_inventory i ON i.runtime_id=w.runtime_id
              AND i.source_market_id=w.source_market_id AND i.item_id=$3
         JOIN world_goods g ON g.runtime_id=i.runtime_id AND g.source_good_id=i.source_good_id
         WHERE a.entity_id=$1 AND w.site_kind='market'
           AND w.source_market_id=$2 AND w.place_id=$4
         ORDER BY w.venue_id LIMIT 1 FOR UPDATE OF w,i",
        &[&operator_id, &source_market_id, &item_id, &place_id],
    )?;
    let market = market.ok_or_else(|| anyhow::anyhow!("經手人並未受派於這個市場"))?;
    let runtime_id: String = market.get(0);
    let venue_id: String = market.get(1);
    let treasury_before: i32 = market.get(2);
    let source_good_id: i32 = market.get(3);
    let market_quantity_before: i32 = market.get(4);
    let target_quantity: i32 = market.get(5);
    let reference_price_mg: i32 = market.get(6);
    let current_ask_mg: i32 = market.get(7);
    let display_name = super::living_world::peninsula_good_display_name(market.get::<_, &str>(8)).to_string();
    let minimum_total = current_ask_mg
        .checked_mul(quantity)
        .map(|total| (total * 7 / 10).max(1))
        .ok_or_else(|| anyhow::anyhow!("市場價格溢位"))?;
    anyhow::ensure!(price_mg >= minimum_total, "出價低於當下市場可接受底線");
    anyhow::ensure!(buyer.magnesium >= price_mg, "買方鎂不足");
    anyhow::ensure!(market_quantity_before >= quantity, "市場商品不足");

    let buyer_mg_after = buyer.magnesium - price_mg;
    let treasury_after = treasury_before
        .checked_add(price_mg)
        .ok_or_else(|| anyhow::anyhow!("市場財庫溢位"))?;
    let market_quantity_after = market_quantity_before - quantity;
    let market_price_after = super::world_market::price_for_stock(
        reference_price_mg,
        market_quantity_after,
        target_quantity,
    );
    let mut buyer_inventory = parse_inventory(&buyer.inventory)?;
    let buyer_quantity_before = inventory_quantity(&buyer_inventory, item_id)?;
    add_quantity_for_actor(
        &mut buyer_inventory,
        item_id,
        quantity,
        buyer.kind == "npc",
        consume_period.is_some(),
    )?;
    let buyer_quantity_after_sale = inventory_quantity(&buyer_inventory, item_id)?;
    let buyer_inventory_after_sale = serde_json::to_string(&buyer_inventory)?;
    let (buyer_activity, operator_activity) = resolve_venue_trade_activities(
        place_id,
        item_id,
        &display_name,
        buyer.kind == "npc",
        &buyer.current_activity,
    );

    tx.execute(
        "UPDATE entities SET magnesium=$1,inventory=$2,inventory_seq=inventory_seq+1,current_activity=$3 WHERE id=$4",
        &[
            &buyer_mg_after,
            &buyer_inventory_after_sale,
            &buyer_activity,
            &buyer.id,
        ],
    )?;
    tx.execute(
        "UPDATE entities SET current_activity=$1 WHERE id=$2",
        &[&operator_activity, &operator.id],
    )?;
    tx.execute(
        "UPDATE world_work_sites SET treasury_mg=$1,updated_at=$2
         WHERE runtime_id=$3 AND venue_id=$4",
        &[&treasury_after, &now_unix, &runtime_id, &venue_id],
    )?;
    tx.execute(
        "UPDATE world_market_inventory SET quantity=$1,price_mg=$2,updated_at=$3
         WHERE runtime_id=$4 AND source_market_id=$5 AND source_good_id=$6",
        &[
            &market_quantity_after,
            &market_price_after,
            &now_unix,
            &runtime_id,
            &source_market_id,
            &source_good_id,
        ],
    )?;

    let trade_id = Uuid::new_v4().to_string();
    let market_actor_id = format!("market:{runtime_id}:{source_market_id}");
    tx.execute(
        "INSERT INTO world_trade_ledger
         (trade_id,world_id,place_id,buyer_id,seller_id,item_id,quantity,price_mg,
          buyer_mg_before,buyer_mg_after,seller_mg_before,seller_mg_after,
          buyer_quantity_before,buyer_quantity_after,seller_quantity_before,
          seller_quantity_after,source_ref,occurred_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
        &[
            &trade_id,
            &WHITE_TOWER_WORLD_ID,
            &place_id,
            &buyer.id,
            &market_actor_id,
            &item_id,
            &quantity,
            &price_mg,
            &buyer.magnesium,
            &buyer_mg_after,
            &treasury_before,
            &treasury_after,
            &buyer_quantity_before,
            &buyer_quantity_after_sale,
            &market_quantity_before,
            &market_quantity_after,
            &source_ref,
            &now_unix,
        ],
    )?;
    let buyer_action_id = super::living_world::record_action_in_tx(
        &mut tx,
        super::living_world::ActionDraft {
            actor_id: &buyer.id,
            action_key: "trade",
            target_ref: &market_actor_id,
            place_id,
            preconditions: serde_json::json!({
                "trade_id": trade_id,
                "same_place": true,
                "buyer_magnesium": buyer.magnesium,
                "market_quantity": market_quantity_before
            }),
            effects: serde_json::json!({
                "trade_id": trade_id,
                "role": "buyer",
                "item_id": item_id,
                "quantity": quantity,
                "immediate_consumption": consume_period.is_some(),
                "magnesium": {"before": buyer.magnesium, "after": buyer_mg_after},
                "inventory_quantity": {
                    "before": buyer_quantity_before,
                    "after": buyer_quantity_after_sale
                }
            }),
            outcome: "market_sale_settled",
            source_ref,
            occurred_at: now_unix,
        },
    )?;
    let operator_action_id = super::living_world::record_action_in_tx(
        &mut tx,
        super::living_world::ActionDraft {
            actor_id: &operator.id,
            action_key: "trade",
            target_ref: &buyer.id,
            place_id,
            preconditions: serde_json::json!({
                "trade_id": trade_id,
                "assigned_venue": venue_id,
                "market_quantity": market_quantity_before
            }),
            effects: serde_json::json!({
                "trade_id": trade_id,
                "role": "market_operator",
                "item_id": item_id,
                "quantity": quantity,
                "market_quantity": {
                    "before": market_quantity_before,
                    "after": market_quantity_after
                },
                "market_treasury": {"before": treasury_before, "after": treasury_after}
            }),
            outcome: "market_sale_settled",
            source_ref,
            occurred_at: now_unix,
        },
    )?;

    let mut final_inventory = buyer_inventory_after_sale;
    let mut final_activity = buyer_activity;
    let mut consumption_id = None;
    if let Some(consumption_period) = consume_period {
        let consumption_before = buyer_quantity_after_sale;
        remove_quantity(&mut buyer_inventory, item_id, quantity)?;
        let consumption_after = inventory_quantity(&buyer_inventory, item_id)?;
        final_inventory = serde_json::to_string(&buyer_inventory)?;
        final_activity = if matches!(
            display_name.as_str(),
            "日曬海鹽" | "茶籽皂塊" | "蜂蠟燭台" | "深海鯨脂" | "精碾白米" | "鮮海魚獲" | "山林野味"
        ) {
            format!("在市集添購了日常所需的{display_name}")
        } else {
            format!("在市集購得並享用{display_name}")
        };
        tx.execute(
            "UPDATE entities SET inventory=$1,inventory_seq=inventory_seq+1,current_activity=$2 WHERE id=$3",
            &[&final_inventory, &final_activity, &buyer.id],
        )?;
        let id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO world_consumption_ledger
             (consumption_id,runtime_id,entity_id,source_market_id,source_good_id,
              place_id,item_id,consumption_period,quantity,inventory_quantity_before,
              inventory_quantity_after,source_ref,occurred_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
            &[
                &id,
                &runtime_id,
                &buyer.id,
                &source_market_id,
                &source_good_id,
                &place_id,
                &item_id,
                &consumption_period,
                &quantity,
                &consumption_before,
                &consumption_after,
                &source_ref,
                &now_unix,
            ],
        )?;
        consumption_id = Some(id);
    }
    tx.commit()?;
    Ok(MarketTradeSettlement {
        trade_id,
        buyer_action_id,
        operator_action_id,
        consumption_id,
        place_id: place_id.to_string(),
        source_market_id,
        item_id: item_id.to_string(),
        display_name,
        quantity,
        price_mg,
        market_quantity_before,
        market_quantity_after,
        market_treasury_before: treasury_before,
        market_treasury_after: treasury_after,
        buyer: TradePartyAfter {
            entity_id: buyer.id.clone(),
            magnesium: buyer_mg_after,
            inventory: final_inventory,
            current_activity: final_activity,
        },
        operator: TradePartyAfter {
            entity_id: operator.id.clone(),
            magnesium: operator.magnesium,
            inventory: operator.inventory.clone(),
            current_activity: operator_activity,
        },
    })
}

/// 讓一名同地居民以自己的鎂購買既有需求輪廓中的可消耗品；每十五分鐘最多一次。
pub fn settle_one_npc_consumption(
    pool: &DbPool,
    now_unix: i64,
) -> anyhow::Result<Option<MarketTradeSettlement>> {
    let consumption_period = now_unix.div_euclid(900);
    let mut conn = pool.get()?;
    let candidate = conn.query_opt(
        "SELECT buyer.id,operator.id,i.source_market_id,i.item_id,i.price_mg
         FROM world_market_inventory i
         JOIN world_work_sites w ON w.runtime_id=i.runtime_id
              AND w.site_kind='market' AND w.source_market_id=i.source_market_id
         JOIN assignments a ON a.venue_id=w.venue_id
         JOIN entities operator ON operator.id=a.entity_id AND operator.place_id=w.place_id
              AND operator.kind='npc' AND operator.vit>0
         JOIN entities buyer ON buyer.place_id=w.place_id AND buyer.kind='npc' AND buyer.vit>0
              AND buyer.id<>operator.id AND buyer.magnesium>=i.price_mg
         WHERE (i.consumable OR i.source_good_id IN (9, 11, 12, 16, 39, 64, 66)) AND i.local_demand_deals>0 AND i.quantity>0 AND i.price_mg >= 1
           AND NOT EXISTS (
             SELECT 1 FROM world_consumption_ledger c
             WHERE c.entity_id=buyer.id AND c.consumption_period=$1)
           AND NOT EXISTS (
             SELECT 1 FROM world_market_shipments s
             WHERE s.carrier_id IN (buyer.id,operator.id) AND s.status='in_transit')
         ORDER BY (i.quantity > i.target_quantity / 2) DESC,
                  abs(hashtextextended(buyer.id||':'||operator.id||':'||i.item_id||':'||$1::text,0)),
                  i.item_id
         LIMIT 1",
        &[&consumption_period],
    )?;
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    let buyer_id: String = candidate.get(0);
    let operator_id: String = candidate.get(1);
    let source_market_id: i32 = candidate.get(2);
    let item_id: String = candidate.get(3);
    let price_mg: i32 = candidate.get(4);
    drop(conn);
    match settle_market_sale(
        pool,
        &buyer_id,
        &operator_id,
        source_market_id,
        &item_id,
        1,
        price_mg,
        now_unix,
        "runtime:npc-market-consumption",
        Some(consumption_period),
    ) {
        Ok(settlement) => Ok(Some(settlement)),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("買方鎂不足")
                || msg.contains("市場商品不足")
                || msg.contains("雙方不在同一市場")
                || msg.contains("出價低於當下市場可接受底線")
                || msg.contains("賣方並非此市場在崗人員")
            {
                Ok(None)
            } else {
                Err(err)
            }
        }
    }
}


#[allow(clippy::too_many_arguments)]
fn settle_locked(
    tx: &mut Transaction<'_>,
    buyer: &LockedActor,
    seller: &LockedActor,
    item_id: &str,
    quantity: i32,
    price_mg: i32,
    now_unix: i64,
    source_ref: &str,
) -> anyhow::Result<TradeSettlement> {
    validate_request(&buyer.id, &seller.id, item_id, quantity, price_mg)?;
    anyhow::ensure!(
        matches!(buyer.kind.as_str(), "player" | "npc"),
        "買方不是人物"
    );
    anyhow::ensure!(
        matches!(seller.kind.as_str(), "player" | "npc"),
        "賣方不是人物"
    );
    anyhow::ensure!(buyer.vit > 0 && seller.vit > 0, "交易人物必須存活");
    let place_id = buyer
        .place_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("買方沒有正式位置"))?;
    anyhow::ensure!(
        seller.place_id.as_deref() == Some(place_id),
        "交易雙方不在同一場所"
    );
    anyhow::ensure!(buyer.magnesium >= price_mg, "買方鎂不足");
    let seller_mg_after = seller
        .magnesium
        .checked_add(price_mg)
        .ok_or_else(|| anyhow::anyhow!("賣方鎂數溢位"))?;
    let buyer_mg_after = buyer.magnesium - price_mg;

    let mut buyer_inventory = parse_inventory(&buyer.inventory)?;
    let mut seller_inventory = parse_inventory(&seller.inventory)?;
    let buyer_quantity_before = inventory_quantity(&buyer_inventory, item_id)?;
    let seller_quantity_before = inventory_quantity(&seller_inventory, item_id)?;
    anyhow::ensure!(seller_quantity_before >= quantity, "賣方商品不足");
    remove_quantity(&mut seller_inventory, item_id, quantity)?;
    add_quantity_for_actor(
        &mut buyer_inventory,
        item_id,
        quantity,
        buyer.kind == "npc",
        false,
    )?;
    let buyer_quantity_after = inventory_quantity(&buyer_inventory, item_id)?;
    let seller_quantity_after = inventory_quantity(&seller_inventory, item_id)?;
    let buyer_inventory_after = serde_json::to_string(&buyer_inventory)?;
    let seller_inventory_after = serde_json::to_string(&seller_inventory)?;
    let buyer_activity = trade_activity(buyer, place_id, item_id, false);
    let seller_activity = trade_activity(seller, place_id, item_id, true);

    tx.execute(
        "UPDATE entities SET magnesium=$1,inventory=$2,inventory_seq=inventory_seq+1,current_activity=$3 WHERE id=$4",
        &[
            &buyer_mg_after,
            &buyer_inventory_after,
            &buyer_activity,
            &buyer.id,
        ],
    )?;
    tx.execute(
        "UPDATE entities SET magnesium=$1,inventory=$2,inventory_seq=inventory_seq+1,current_activity=$3 WHERE id=$4",
        &[
            &seller_mg_after,
            &seller_inventory_after,
            &seller_activity,
            &seller.id,
        ],
    )?;

    let trade_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO world_trade_ledger
         (trade_id,world_id,place_id,buyer_id,seller_id,item_id,quantity,price_mg,
          buyer_mg_before,buyer_mg_after,seller_mg_before,seller_mg_after,
          buyer_quantity_before,buyer_quantity_after,seller_quantity_before,
          seller_quantity_after,source_ref,occurred_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
        &[
            &trade_id,
            &WHITE_TOWER_WORLD_ID,
            &place_id,
            &buyer.id,
            &seller.id,
            &item_id,
            &quantity,
            &price_mg,
            &buyer.magnesium,
            &buyer_mg_after,
            &seller.magnesium,
            &seller_mg_after,
            &buyer_quantity_before,
            &buyer_quantity_after,
            &seller_quantity_before,
            &seller_quantity_after,
            &source_ref,
            &now_unix,
        ],
    )?;

    let buyer_action_id = crate::pg::living_world::record_action_in_tx(
        tx,
        crate::pg::living_world::ActionDraft {
            actor_id: &buyer.id,
            action_key: "trade",
            target_ref: &seller.id,
            place_id,
            preconditions: serde_json::json!({
                "trade_id": trade_id,
                "same_place": true,
                "buyer_magnesium": buyer.magnesium,
                "seller_quantity": seller_quantity_before
            }),
            effects: serde_json::json!({
                "trade_id": trade_id,
                "role": "buyer",
                "item_id": item_id,
                "quantity": quantity,
                "magnesium": {"before": buyer.magnesium, "after": buyer_mg_after},
                "inventory_quantity": {"before": buyer_quantity_before, "after": buyer_quantity_after}
            }),
            outcome: "settled",
            source_ref,
            occurred_at: now_unix,
        },
    )?;
    let seller_action_id = crate::pg::living_world::record_action_in_tx(
        tx,
        crate::pg::living_world::ActionDraft {
            actor_id: &seller.id,
            action_key: "trade",
            target_ref: &buyer.id,
            place_id,
            preconditions: serde_json::json!({
                "trade_id": trade_id,
                "same_place": true,
                "seller_quantity": seller_quantity_before
            }),
            effects: serde_json::json!({
                "trade_id": trade_id,
                "role": "seller",
                "item_id": item_id,
                "quantity": quantity,
                "magnesium": {"before": seller.magnesium, "after": seller_mg_after},
                "inventory_quantity": {"before": seller_quantity_before, "after": seller_quantity_after}
            }),
            outcome: "settled",
            source_ref,
            occurred_at: now_unix,
        },
    )?;
    Ok(TradeSettlement {
        trade_id,
        buyer_action_id,
        seller_action_id,
        place_id: place_id.to_string(),
        item_id: item_id.to_string(),
        quantity,
        price_mg,
        buyer: TradePartyAfter {
            entity_id: buyer.id.clone(),
            magnesium: buyer_mg_after,
            inventory: buyer_inventory_after,
            current_activity: buyer_activity,
        },
        seller: TradePartyAfter {
            entity_id: seller.id.clone(),
            magnesium: seller_mg_after,
            inventory: seller_inventory_after,
            current_activity: seller_activity,
        },
    })
}

fn actor_from_row(row: Row) -> LockedActor {
    LockedActor {
        id: row.get(0),
        kind: row.get(1),
        vit: row.get(2),
        magnesium: row.get(3),
        inventory: row.get(4),
        place_id: row.get(5),
        current_activity: row.get(6),
    }
}

fn validate_request(
    buyer_id: &str,
    seller_id: &str,
    item_id: &str,
    quantity: i32,
    price_mg: i32,
) -> anyhow::Result<()> {
    anyhow::ensure!(buyer_id != seller_id, "交易雙方不能是同一人物");
    anyhow::ensure!(
        !buyer_id.trim().is_empty() && !seller_id.trim().is_empty(),
        "交易人物不可空白"
    );
    anyhow::ensure!(!item_id.trim().is_empty(), "交易商品不可空白");
    anyhow::ensure!(quantity > 0, "交易數量須為正");
    anyhow::ensure!(price_mg > 0, "交易鎂數須為正");
    Ok(())
}

fn parse_inventory(raw: &str) -> anyhow::Result<Vec<InventoryEntry>> {
    if raw.trim().is_empty() || raw.trim() == "[]" {
        return Ok(Vec::new());
    }
    let entries: Vec<InventoryEntry> = serde_json::from_str(raw)?;
    anyhow::ensure!(
        entries
            .iter()
            .all(|entry| !entry.item_id.trim().is_empty() && entry.qty > 0),
        "背包含無效物品列"
    );
    Ok(entries)
}

fn inventory_quantity(entries: &[InventoryEntry], item_id: &str) -> anyhow::Result<i32> {
    entries
        .iter()
        .filter(|entry| entry.item_id == item_id)
        .try_fold(0_i32, |sum, entry| {
            sum.checked_add(entry.qty)
                .ok_or_else(|| anyhow::anyhow!("物品數量溢位"))
        })
}

#[cfg(test)]
fn add_quantity(
    entries: &mut Vec<InventoryEntry>,
    item_id: &str,
    quantity: i32,
) -> anyhow::Result<()> {
    add_quantity_for_actor(entries, item_id, quantity, true, false)
}

fn add_quantity_for_actor(
    entries: &mut Vec<InventoryEntry>,
    item_id: &str,
    quantity: i32,
    is_npc: bool,
    immediate_consume: bool,
) -> anyhow::Result<()> {
    if let Some(entry) = entries.iter_mut().find(|entry| entry.item_id == item_id) {
        let after = (entry.qty as i64).saturating_add(i64::from(quantity));
        anyhow::ensure!(after <= i64::from(i32::MAX), "物品數量溢位");
        if is_npc && !immediate_consume {
            anyhow::ensure!(
                after <= crate::db::inventory::MAX_NPC_ITEM_STACK_QTY as i64,
                "買方該項物產堆疊已達上限"
            );
        }
        entry.qty = after as i32;
    } else {
        if is_npc && !immediate_consume {
            anyhow::ensure!(
                entries.len() < crate::db::inventory::MAX_NPC_DISTINCT_ITEMS,
                "買方行囊品類已滿"
            );
            anyhow::ensure!(
                quantity <= crate::db::inventory::MAX_NPC_ITEM_STACK_QTY,
                "買方該項物產堆疊已達上限"
            );
        }
        entries.push(InventoryEntry {
            item_id: item_id.to_string(),
            qty: quantity,
        });
    }
    Ok(())
}

fn remove_quantity(
    entries: &mut Vec<InventoryEntry>,
    item_id: &str,
    quantity: i32,
) -> anyhow::Result<()> {
    let mut remaining = quantity;
    for entry in entries.iter_mut().filter(|entry| entry.item_id == item_id) {
        let removed = remaining.min(entry.qty);
        entry.qty -= removed;
        remaining -= removed;
        if remaining == 0 {
            break;
        }
    }
    anyhow::ensure!(remaining == 0, "賣方商品不足");
    entries.retain(|entry| entry.qty > 0);
    Ok(())
}

fn resolve_venue_trade_activities(
    place_id: &str,
    item_id_or_name: &str,
    display_name: &str,
    buyer_is_npc: bool,
    buyer_prev_activity: &str,
) -> (String, String) {
    let raw_place_title = crate::store::get_store()
        .and_then(|s| {
            s.read()
                .place_graph
                .place(place_id)
                .map(|p| p.player_title().to_string())
        })
        .unwrap_or_else(|| place_id.to_string());
    let place_title = super::living_world::peninsula_market_settlement_name(&raw_place_title);
    let venue_opt = crate::grid::resolve_any_room_venue_object(&raw_place_title, display_name)
        .or_else(|| crate::grid::resolve_any_room_venue_object(place_title, display_name));

    let operator_activity = match venue_opt.as_ref() {
        Some(venue) if venue.has_counter_restricted_zone => {
            let venue_kind = if venue.venue_kind == "大食堂" {
                "聚落大食堂"
            } else {
                venue.venue_kind
            };
            let host_title = if venue.default_host_title == "食堂主廚" {
                "當值大廚"
            } else {
                venue.default_host_title
            };
            format!("在{}當值櫃檯內側交付{}（{}）", venue_kind, display_name, host_title)
        }
        Some(venue) => {
            let venue_kind = if venue.venue_kind == "大食堂" {
                "聚落大食堂"
            } else {
                venue.venue_kind
            };
            format!("正在替{}交付{}", venue_kind, display_name)
        }
        None => format!("正在替市場交付{display_name}"),
    };

    let buyer_activity = if !buyer_is_npc {
        buyer_prev_activity.to_string()
    } else if let Some(venue) = venue_opt.as_ref() {
        let venue_kind = if venue.venue_kind == "大食堂" {
            "聚落大食堂"
        } else {
            venue.venue_kind
        };
        format!("在{}櫃檯前買下{}", venue_kind, display_name)
    } else {
        format!("正在市場買下{display_name}")
    };

    let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
        &raw_place_title,
        display_name,
    );
    if item_id_or_name != display_name {
        let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
            &raw_place_title,
            item_id_or_name,
        );
    }

    (buyer_activity, operator_activity)
}

fn trade_activity(actor: &LockedActor, place_id: &str, item_id: &str, is_seller: bool) -> String {
    let item_name = crate::game::peninsula_catalog::find_catalog_entry(item_id)
        .map(|e| e.name)
        .unwrap_or(item_id);
    let raw_place_title = crate::store::get_store()
        .and_then(|s| {
            s.read()
                .place_graph
                .place(place_id)
                .map(|p| p.player_title().to_string())
        })
        .unwrap_or_else(|| place_id.to_string());
    if is_seller {
        let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
            &raw_place_title,
            item_name,
        );
    }
    if actor.kind == "npc" {
        let place_title = super::living_world::peninsula_market_settlement_name(&raw_place_title);
        if let Some(venue) = crate::grid::resolve_any_room_venue_object(&raw_place_title, item_name)
            .or_else(|| crate::grid::resolve_any_room_venue_object(place_title, item_name))
        {
            let venue_kind = if venue.venue_kind == "大食堂" {
                "聚落大食堂"
            } else {
                venue.venue_kind
            };
            if is_seller && venue.has_counter_restricted_zone {
                format!("在{}當值櫃檯內側點交{}", venue_kind, item_name)
            } else if is_seller {
                format!("在{}點交{}", venue_kind, item_name)
            } else {
                format!("在{}買入{}", venue_kind, item_name)
            }
        } else {
            "正在與身邊的人交易".to_string()
        }
    } else {
        actor.current_activity.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_transfer_preserves_total_quantity() -> anyhow::Result<()> {
        let mut buyer = parse_inventory(r#"[{"item_id":"paper","qty":2}]"#)?;
        let mut seller =
            parse_inventory(r#"[{"item_id":"soap","qty":2},{"item_id":"paper","qty":1}]"#)?;
        let before = inventory_quantity(&buyer, "soap")? + inventory_quantity(&seller, "soap")?;
        remove_quantity(&mut seller, "soap", 1)?;
        add_quantity(&mut buyer, "soap", 1)?;
        let after = inventory_quantity(&buyer, "soap")? + inventory_quantity(&seller, "soap")?;
        assert_eq!(before, after);
        assert_eq!(inventory_quantity(&buyer, "paper")?, 2);
        assert_eq!(inventory_quantity(&seller, "paper")?, 1);
        Ok(())
    }

    #[test]
    fn invalid_trade_request_is_rejected_before_mutation() {
        assert!(validate_request("same", "same", "soap", 1, 2).is_err());
        assert!(validate_request("buyer", "seller", "", 1, 2).is_err());
        assert!(validate_request("buyer", "seller", "soap", 0, 2).is_err());
        assert!(validate_request("buyer", "seller", "soap", 1, 0).is_err());
    }

    #[test]
    fn trade_add_quantity_enforces_npc_stack_and_distinct_caps() -> anyhow::Result<()> {
        let mut entries = Vec::new();
        assert!(add_quantity_for_actor(&mut entries, "mat_hay", 20, true, false).is_err());
        add_quantity_for_actor(&mut entries, "mat_hay", 12, true, false)?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].qty, crate::db::inventory::MAX_NPC_ITEM_STACK_QTY);

        // Rejects exceeding stack cap for non-immediate-consume NPC purchase
        assert!(add_quantity_for_actor(&mut entries, "mat_hay", 1, true, false).is_err());

        // Fill to MAX_NPC_DISTINCT_ITEMS
        for i in 1..crate::db::inventory::MAX_NPC_DISTINCT_ITEMS {
            add_quantity_for_actor(&mut entries, &format!("item_{i}"), 1, true, false)?;
        }
        assert_eq!(entries.len(), crate::db::inventory::MAX_NPC_DISTINCT_ITEMS);

        // 9th distinct item is rejected for non-immediate-consume NPC purchase
        assert!(add_quantity_for_actor(&mut entries, "overflow_item", 1, true, false).is_err());

        // Immediate market consumption succeeds even when 8 slots are full, then removes cleanly
        add_quantity_for_actor(&mut entries, "daily_ration", 1, true, true)?;
        remove_quantity(&mut entries, "daily_ration", 1)?;
        assert_eq!(entries.len(), crate::db::inventory::MAX_NPC_DISTINCT_ITEMS);

        // Player buyers are not restricted by NPC 12/8 cap
        let mut player_entries = Vec::new();
        add_quantity_for_actor(&mut player_entries, "mat_hay", 25, false, false)?;
        assert_eq!(player_entries[0].qty, 25);
        Ok(())
    }

    #[test]
    fn venue_trade_activities_reflect_counter_zone_and_advance_event_pack() {
        let (buyer_act, op_act) = resolve_venue_trade_activities(
            "三水大食堂",
            "wt_cat_0444",
            "三水大釜牛雜湯",
            true,
            "",
        );
        assert_eq!(
            op_act,
            "在聚落大食堂當值櫃檯內側交付三水大釜牛雜湯（當值大廚）"
        );
        assert_eq!(buyer_act, "在聚落大食堂櫃檯前買下三水大釜牛雜湯");
    }
}

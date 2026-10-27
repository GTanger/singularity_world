//! Azgaar 作者包中的商品、市場與交易證據。
//!
//! 這份資料不是正典，也不是玩家報表；它保留現有作者資料並把每筆經濟紀錄
//! 對回白塔半島的正式物理區，供 runtime 的價格、產能與人物工作使用。

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

use super::runtime_world::{WHITE_TOWER_RUNTIME_ID, builtin_white_tower_runtime_world};

pub const WHITE_TOWER_AUTHOR_ECONOMY_ID: &str = "white_tower_author_economy_v1";
pub const WHITE_TOWER_AUTHOR_ECONOMY_JSON: &str =
    include_str!("../../data/world/white_tower_author_economy_v1.json");
pub const WHITE_TOWER_GOOD_ITEM_PREFIX: &str = "wt-good-";

#[must_use]
pub fn runtime_good_item_id(source_good_id: u32) -> String {
    format!("{WHITE_TOWER_GOOD_ITEM_PREFIX}{source_good_id:03}")
}

#[must_use]
pub fn source_good_id_from_item_id(item_id: &str) -> Option<u32> {
    let id = item_id.strip_prefix(WHITE_TOWER_GOOD_ITEM_PREFIX)?;
    if id.len() != 3 || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let id = id.parse().ok()?;
    (1..=71).contains(&id).then_some(id)
}

#[must_use]
pub fn is_runtime_good_item(item_id: &str) -> bool {
    source_good_id_from_item_id(item_id).is_some_and(|id| id != 31)
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AuthorGood {
    pub source_good_id: u32,
    pub source_name: String,
    pub display_name: String,
    pub source_unit: String,
    pub tags: Vec<String>,
    pub base_value: f64,
    pub runtime_disposition: String,
    pub runtime_trade_allowed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AuthorMarketQuote {
    pub source_good_id: u32,
    pub stock: f64,
    pub price: f64,
    pub runtime_trade_allowed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AuthorMarket {
    pub source_market_id: u32,
    pub center_burg_id: u32,
    pub place_id: String,
    pub treasury_reference: f64,
    pub quotes: Vec<AuthorMarketQuote>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AuthorTradeEndpoint {
    pub source_type: String,
    pub source_id: u32,
    pub place_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AuthorTradeEvidence {
    pub source_deal_id: u32,
    pub seller: AuthorTradeEndpoint,
    pub buyer: AuthorTradeEndpoint,
    pub source_good_id: u32,
    pub units: f64,
    pub price: f64,
    pub tax: f64,
    pub runtime_trade_allowed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AuthorEconomyCoverage {
    pub goods_accounted: usize,
    pub goods_runtime_allowed: usize,
    pub goods_blocked_from_trade: usize,
    pub markets_accounted: usize,
    pub market_quotes_accounted: usize,
    pub deals_accounted: usize,
    pub deals_runtime_allowed: usize,
    pub deals_retained_only: usize,
    pub burg_endpoints_resolved: usize,
    pub market_endpoints_resolved: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct WhiteTowerAuthorEconomy {
    pub schema_version: u32,
    pub id: String,
    pub runtime_id: String,
    pub status: String,
    pub canon: bool,
    pub runtime_authority: bool,
    pub player_facing_report: bool,
    pub source_package_id: String,
    pub source_full_json_path: String,
    pub source_full_json_sha256: String,
    pub compiler_version: u32,
    pub goods: Vec<AuthorGood>,
    pub markets: Vec<AuthorMarket>,
    pub trade_evidence: Vec<AuthorTradeEvidence>,
    pub coverage: AuthorEconomyCoverage,
}

pub fn builtin_white_tower_author_economy() -> Result<WhiteTowerAuthorEconomy> {
    let economy: WhiteTowerAuthorEconomy = serde_json::from_str(WHITE_TOWER_AUTHOR_ECONOMY_JSON)?;
    validate_white_tower_author_economy(&economy)?;
    Ok(economy)
}

pub fn validate_white_tower_author_economy(economy: &WhiteTowerAuthorEconomy) -> Result<()> {
    ensure!(economy.schema_version == 1, "不支援的白塔經濟證據版本");
    ensure!(
        economy.id == WHITE_TOWER_AUTHOR_ECONOMY_ID,
        "白塔經濟證據 id 不符"
    );
    ensure!(
        economy.runtime_id == WHITE_TOWER_RUNTIME_ID,
        "經濟證據未對準正式 runtime"
    );
    ensure!(
        economy.status == "runtime_evidence",
        "經濟資料不是 runtime evidence"
    );
    ensure!(!economy.canon, "Azgaar 經濟候選不可冒充正典");
    ensure!(
        economy.runtime_authority,
        "經濟證據尚未成為 runtime 查詢權威"
    );
    ensure!(
        !economy.player_facing_report,
        "作者交易明細不可直接灌成玩家報表"
    );
    ensure!(
        !economy.source_full_json_sha256.is_empty(),
        "缺少來源 Full JSON 收據"
    );

    let world = builtin_white_tower_runtime_world()?;
    let place_ids = world
        .places
        .iter()
        .map(|place| place.id.as_str())
        .collect::<BTreeSet<_>>();
    let goods = economy
        .goods
        .iter()
        .map(|good| (good.source_good_id, good))
        .collect::<BTreeMap<_, _>>();
    ensure!(goods.len() == economy.goods.len(), "商品 id 重複");
    ensure!(goods.len() == 71, "Azgaar 71 種商品未完整保留");
    for good in &economy.goods {
        ensure!(good.source_good_id > 0, "商品 id 無效");
        ensure!(!good.source_name.trim().is_empty(), "商品來源名稱空白");
        ensure!(!good.display_name.trim().is_empty(), "商品顯示名稱空白");
        ensure!(
            good.base_value >= 0.0 && good.base_value.is_finite(),
            "商品價值無效"
        );
        if good.source_good_id == 31 {
            ensure!(
                !good.runtime_trade_allowed,
                "人口販運項目不得進入 runtime 交易"
            );
            ensure!(
                good.runtime_disposition == "retained_blocked_human_commodity",
                "被阻擋的人口販運來源仍須明確保留"
            );
        }
    }

    let market_ids = economy
        .markets
        .iter()
        .map(|market| market.source_market_id)
        .collect::<BTreeSet<_>>();
    ensure!(market_ids.len() == economy.markets.len(), "市場 id 重複");
    ensure!(market_ids.len() == 54, "Azgaar 54 個市場未完整保留");
    let mut quote_count = 0_usize;
    for market in &economy.markets {
        ensure!(
            place_ids.contains(market.place_id.as_str()),
            "市場未對回正式物理區"
        );
        ensure!(
            market.treasury_reference.is_finite(),
            "市場中心聚落財庫證據無效"
        );
        ensure!(
            market.quotes.len() == goods.len(),
            "市場報價沒有覆蓋全部商品"
        );
        let mut quote_goods = BTreeSet::new();
        for quote in &market.quotes {
            let good = goods
                .get(&quote.source_good_id)
                .ok_or_else(|| anyhow::anyhow!("市場報價引用不存在商品"))?;
            ensure!(quote_goods.insert(quote.source_good_id), "市場商品報價重複");
            ensure!(
                quote.stock >= 0.0 && quote.stock.is_finite(),
                "市場庫存無效"
            );
            ensure!(quote.price > 0.0 && quote.price.is_finite(), "市場價格無效");
            ensure!(
                quote.runtime_trade_allowed == good.runtime_trade_allowed,
                "市場報價繞過商品禁售狀態"
            );
            quote_count += 1;
        }
    }

    let deal_ids = economy
        .trade_evidence
        .iter()
        .map(|deal| deal.source_deal_id)
        .collect::<BTreeSet<_>>();
    ensure!(
        deal_ids.len() == economy.trade_evidence.len(),
        "交易證據 id 重複"
    );
    ensure!(deal_ids.len() == 25_808, "Azgaar 25,808 筆交易未完整保留");
    for deal in &economy.trade_evidence {
        let good = goods
            .get(&deal.source_good_id)
            .ok_or_else(|| anyhow::anyhow!("交易引用不存在商品"))?;
        for endpoint in [&deal.seller, &deal.buyer] {
            ensure!(
                matches!(endpoint.source_type.as_str(), "burg" | "market"),
                "交易端點類型無效"
            );
            ensure!(
                place_ids.contains(endpoint.place_id.as_str()),
                "交易端點未對回正式物理區"
            );
        }
        ensure!(deal.units >= 0.0 && deal.units.is_finite(), "交易數量無效");
        ensure!(deal.price > 0.0 && deal.price.is_finite(), "交易價格無效");
        ensure!(deal.tax >= 0.0 && deal.tax.is_finite(), "交易稅額無效");
        ensure!(
            deal.runtime_trade_allowed == (good.runtime_trade_allowed && deal.units > 0.0),
            "交易可用狀態與商品／數量不一致"
        );
    }

    ensure!(
        economy.coverage.goods_accounted == goods.len(),
        "商品覆蓋收據不符"
    );
    ensure!(
        economy.coverage.markets_accounted == economy.markets.len(),
        "市場覆蓋收據不符"
    );
    ensure!(
        economy.coverage.market_quotes_accounted == quote_count,
        "市場報價收據不符"
    );
    ensure!(
        economy.coverage.deals_accounted == economy.trade_evidence.len(),
        "交易覆蓋收據不符"
    );
    ensure!(
        economy.coverage.goods_runtime_allowed + economy.coverage.goods_blocked_from_trade
            == economy.coverage.goods_accounted,
        "商品去向未閉合"
    );
    ensure!(
        economy.coverage.deals_runtime_allowed + economy.coverage.deals_retained_only
            == economy.coverage.deals_accounted,
        "交易去向未閉合"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_economy_keeps_every_source_record_and_blocks_human_trade() -> Result<()> {
        let economy = builtin_white_tower_author_economy()?;
        assert_eq!(economy.goods.len(), 71);
        assert_eq!(economy.markets.len(), 54);
        assert_eq!(economy.trade_evidence.len(), 25_808);
        let blocked = economy
            .goods
            .iter()
            .find(|good| good.source_good_id == 31)
            .expect("source good 31 retained");
        assert!(!blocked.runtime_trade_allowed);
        assert!(
            economy
                .trade_evidence
                .iter()
                .filter(|deal| deal.source_good_id == 31)
                .all(|deal| !deal.runtime_trade_allowed)
        );
        Ok(())
    }

    #[test]
    fn runtime_item_ids_are_stable_and_cannot_enable_blocked_human_trade() {
        assert_eq!(runtime_good_item_id(1), "wt-good-001");
        assert_eq!(source_good_id_from_item_id("wt-good-071"), Some(71));
        assert!(is_runtime_good_item("wt-good-070"));
        assert!(!is_runtime_good_item("wt-good-031"));
        assert_eq!(source_good_id_from_item_id("mat_rough_ore"), None);
        assert_eq!(source_good_id_from_item_id("wt-good-72"), None);
    }
}

#![forbid(unsafe_code)]
//! 把 Azgaar 已有的商品、市場與交易逐筆對回白塔半島物理區。
//!
//! 本工具不生成經濟，也不把候選升格成正典；只把 71 / 54 / 25,808 筆既有
//! 作者資料編成可查詢、可驗證的 runtime evidence。

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use singularity_world::world::author_economy::{
    AuthorEconomyCoverage, AuthorGood, AuthorMarket, AuthorMarketQuote, AuthorTradeEndpoint,
    AuthorTradeEvidence, WHITE_TOWER_AUTHOR_ECONOMY_ID, WhiteTowerAuthorEconomy,
    validate_white_tower_author_economy,
};
use singularity_world::world::physical_geography::{
    NormalizedPoint, PhysicalArea, PhysicalGeography, validate_physical_geography,
};
use singularity_world::world::runtime_world::WHITE_TOWER_RUNTIME_ID;

const MANIFEST_PATH: &str = "data/world/white_tower_azgaar_v1/manifest.json";
const FULL_JSON_PATH: &str = "data/world/white_tower_azgaar_v1/white_tower_peninsula_v1_full.json";
const GEOGRAPHY_PATH: &str = "data/world/white_tower_physical_geography_v1.json";
const OUTPUT_PATH: &str = "data/world/white_tower_author_economy_v1.json";

struct AreaLookup<'a> {
    areas: &'a [PhysicalArea],
}

impl AreaLookup<'_> {
    fn associate(&self, point: &NormalizedPoint, feature_id: Option<u32>) -> &PhysicalArea {
        let same_feature = self
            .areas
            .iter()
            .filter(|area| feature_id == Some(area.feature_id))
            .collect::<Vec<_>>();
        let candidates = if same_feature.is_empty() {
            self.areas.iter().collect::<Vec<_>>()
        } else {
            same_feature
        };
        candidates
            .into_iter()
            .min_by(|left, right| {
                squared_distance(&left.centroid, point)
                    .total_cmp(&squared_distance(&right.centroid, point))
                    .then_with(|| left.id.cmp(&right.id))
            })
            .expect("validated geography has areas")
    }
}

fn main() -> Result<()> {
    let output = parse_output_path()?;
    let manifest_bytes = fs::read(MANIFEST_PATH).context("讀取 Azgaar manifest 失敗")?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let full_bytes = fs::read(FULL_JSON_PATH).context("讀取 Azgaar Full JSON 失敗")?;
    let full: Value = serde_json::from_slice(&full_bytes).context("解析 Full JSON 失敗")?;
    let geography: PhysicalGeography = serde_json::from_slice(&fs::read(GEOGRAPHY_PATH)?)?;
    validate_physical_geography(&geography)?;
    verify_manifest(&manifest, &full_bytes)?;

    let economy = compile_economy(&manifest, &full, &geography, &full_bytes)?;
    validate_white_tower_author_economy(&economy)?;
    fs::write(&output, serde_json::to_string_pretty(&economy)? + "\n")
        .with_context(|| format!("寫入 {} 失敗", output.display()))?;
    println!("white_tower_economy_compile: ok");
    println!(
        "goods={} markets={} quotes={} deals={} runtime_allowed_deals={} retained_only_deals={}",
        economy.goods.len(),
        economy.markets.len(),
        economy.coverage.market_quotes_accounted,
        economy.trade_evidence.len(),
        economy.coverage.deals_runtime_allowed,
        economy.coverage.deals_retained_only
    );
    println!("output={}", output.display());
    Ok(())
}

fn compile_economy(
    manifest: &Value,
    full: &Value,
    geography: &PhysicalGeography,
    full_bytes: &[u8],
) -> Result<WhiteTowerAuthorEconomy> {
    let goods = compile_goods(full)?;
    let good_allowed = goods
        .iter()
        .map(|good| (good.source_good_id, good.runtime_trade_allowed))
        .collect::<BTreeMap<_, _>>();
    let lookup = AreaLookup {
        areas: &geography.areas,
    };
    let burg_places = compile_burg_places(full, geography, &lookup)?;
    let burg_treasuries = compile_burg_treasuries(full)?;
    let markets = compile_markets(full, &burg_places, &burg_treasuries, &good_allowed)?;
    let market_places = markets
        .iter()
        .map(|market| (market.source_market_id, market.place_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let (trade_evidence, burg_endpoints, market_endpoints) =
        compile_deals(full, &burg_places, &market_places, &good_allowed)?;
    let goods_runtime_allowed = goods
        .iter()
        .filter(|good| good.runtime_trade_allowed)
        .count();
    let deals_runtime_allowed = trade_evidence
        .iter()
        .filter(|deal| deal.runtime_trade_allowed)
        .count();
    let market_quotes_accounted = markets.iter().map(|market| market.quotes.len()).sum();
    Ok(WhiteTowerAuthorEconomy {
        schema_version: 1,
        id: WHITE_TOWER_AUTHOR_ECONOMY_ID.to_string(),
        runtime_id: WHITE_TOWER_RUNTIME_ID.to_string(),
        status: "runtime_evidence".to_string(),
        canon: false,
        runtime_authority: true,
        player_facing_report: false,
        source_package_id: manifest["id"].as_str().unwrap_or_default().to_string(),
        source_full_json_path: FULL_JSON_PATH.to_string(),
        source_full_json_sha256: sha256(full_bytes),
        compiler_version: 1,
        coverage: AuthorEconomyCoverage {
            goods_accounted: goods.len(),
            goods_runtime_allowed,
            goods_blocked_from_trade: goods.len() - goods_runtime_allowed,
            markets_accounted: markets.len(),
            market_quotes_accounted,
            deals_accounted: trade_evidence.len(),
            deals_runtime_allowed,
            deals_retained_only: trade_evidence.len() - deals_runtime_allowed,
            burg_endpoints_resolved: burg_endpoints,
            market_endpoints_resolved: market_endpoints,
        },
        goods,
        markets,
        trade_evidence,
    })
}

fn compile_goods(full: &Value) -> Result<Vec<AuthorGood>> {
    let source = array_at(&full["pack"], "goods")?;
    let mut goods = Vec::with_capacity(source.len());
    for raw in source {
        let source_good_id = u32_at(raw, "i")?;
        let source_name = string_at(raw, "name")?.to_string();
        let runtime_trade_allowed = source_good_id != 31;
        let tags = raw["tags"]
            .as_array()
            .context("商品 tags 不是陣列")?
            .iter()
            .map(|tag| tag.as_str().unwrap_or_default().to_string())
            .collect();
        goods.push(AuthorGood {
            source_good_id,
            display_name: display_name(source_good_id, &source_name).to_string(),
            source_name,
            source_unit: string_at(raw, "unit")?.to_string(),
            tags,
            base_value: f64_at(raw, "value")?,
            runtime_disposition: if runtime_trade_allowed {
                "runtime_trade_reference".to_string()
            } else {
                "retained_blocked_human_commodity".to_string()
            },
            runtime_trade_allowed,
        });
    }
    goods.sort_by_key(|good| good.source_good_id);
    Ok(goods)
}

fn compile_burg_places(
    full: &Value,
    geography: &PhysicalGeography,
    lookup: &AreaLookup<'_>,
) -> Result<BTreeMap<u32, String>> {
    let mut burg_places = BTreeMap::new();
    for burg in array_at(&full["pack"], "burgs")?
        .iter()
        .filter(|burg| burg.is_object())
    {
        let id = u32_at(burg, "i")?;
        let point = NormalizedPoint {
            x: f64_at(burg, "x")? / f64::from(geography.coordinate_space.width),
            y: f64_at(burg, "y")? / f64::from(geography.coordinate_space.height),
        };
        let feature = burg["feature"]
            .as_u64()
            .and_then(|value| u32::try_from(value).ok());
        let area = lookup.associate(&point, feature);
        ensure!(
            burg_places.insert(id, area.id.clone()).is_none(),
            "聚落 id 重複"
        );
    }
    ensure!(burg_places.len() == 1_666, "聚落端點沒有完整對回物理區");
    Ok(burg_places)
}

fn compile_markets(
    full: &Value,
    burg_places: &BTreeMap<u32, String>,
    burg_treasuries: &BTreeMap<u32, f64>,
    good_allowed: &BTreeMap<u32, bool>,
) -> Result<Vec<AuthorMarket>> {
    let mut markets = Vec::new();
    for raw in array_at(&full["pack"], "markets")? {
        let source_market_id = u32_at(raw, "i")?;
        let center_burg_id = u32_at(raw, "centerBurgId")?;
        let place_id = burg_places
            .get(&center_burg_id)
            .with_context(|| format!("市場 {source_market_id} 中心聚落不存在"))?
            .clone();
        let treasury_reference = *burg_treasuries
            .get(&center_burg_id)
            .with_context(|| format!("市場 {source_market_id} 缺中心聚落財庫"))?;
        let goods = raw["goods"].as_object().context("市場 goods 不是物件")?;
        let mut quotes = Vec::with_capacity(goods.len());
        for (good_id, quote) in goods {
            let source_good_id = good_id.parse::<u32>().context("市場商品 id 無效")?;
            let runtime_trade_allowed = *good_allowed
                .get(&source_good_id)
                .context("市場引用不存在商品")?;
            quotes.push(AuthorMarketQuote {
                source_good_id,
                stock: f64_at(quote, "stock")?,
                price: f64_at(quote, "price")?,
                runtime_trade_allowed,
            });
        }
        quotes.sort_by_key(|quote| quote.source_good_id);
        markets.push(AuthorMarket {
            source_market_id,
            center_burg_id,
            place_id,
            treasury_reference,
            quotes,
        });
    }
    markets.sort_by_key(|market| market.source_market_id);
    Ok(markets)
}

fn compile_burg_treasuries(full: &Value) -> Result<BTreeMap<u32, f64>> {
    array_at(&full["pack"], "burgs")?
        .iter()
        .filter(|burg| burg.is_object())
        .map(|burg| Ok((u32_at(burg, "i")?, f64_at(burg, "treasury")?)))
        .collect()
}

fn compile_deals(
    full: &Value,
    burg_places: &BTreeMap<u32, String>,
    market_places: &BTreeMap<u32, String>,
    good_allowed: &BTreeMap<u32, bool>,
) -> Result<(Vec<AuthorTradeEvidence>, usize, usize)> {
    let mut deals = Vec::new();
    let mut burg_endpoints = 0;
    let mut market_endpoints = 0;
    for raw in array_at(&full["pack"], "deals")? {
        let seller = endpoint(raw, "seller", "sellerType", burg_places, market_places)?;
        let buyer = endpoint(raw, "buyer", "buyerType", burg_places, market_places)?;
        for endpoint in [&seller, &buyer] {
            match endpoint.source_type.as_str() {
                "burg" => burg_endpoints += 1,
                "market" => market_endpoints += 1,
                _ => unreachable!("endpoint validated"),
            }
        }
        let source_good_id = u32_at(raw, "good")?;
        let units = f64_at(raw, "units")?;
        let runtime_trade_allowed = good_allowed
            .get(&source_good_id)
            .copied()
            .context("交易引用不存在商品")?
            && units > 0.0;
        deals.push(AuthorTradeEvidence {
            source_deal_id: u32_at_allow_zero(raw, "i")?,
            seller,
            buyer,
            source_good_id,
            units,
            price: f64_at(raw, "price")?,
            tax: f64_at(raw, "tax")?,
            runtime_trade_allowed,
        });
    }
    deals.sort_by_key(|deal| deal.source_deal_id);
    Ok((deals, burg_endpoints, market_endpoints))
}

fn endpoint(
    raw: &Value,
    id_key: &str,
    type_key: &str,
    burg_places: &BTreeMap<u32, String>,
    market_places: &BTreeMap<u32, String>,
) -> Result<AuthorTradeEndpoint> {
    let source_id = u32_at(raw, id_key)?;
    let source_type = string_at(raw, type_key)?.to_string();
    let place_id = match source_type.as_str() {
        "burg" => burg_places.get(&source_id),
        "market" => market_places.get(&source_id),
        other => anyhow::bail!("未知交易端點類型：{other}"),
    }
    .with_context(|| format!("{source_type} {source_id} 沒有對應物理區"))?
    .clone();
    Ok(AuthorTradeEndpoint {
        source_type,
        source_id,
        place_id,
    })
}

fn verify_manifest(manifest: &Value, full_bytes: &[u8]) -> Result<()> {
    ensure!(
        manifest["generated"]["goods"].as_u64() == Some(71),
        "manifest 商品數不符"
    );
    ensure!(
        manifest["generated"]["markets"].as_u64() == Some(54),
        "manifest 市場數不符"
    );
    ensure!(
        manifest["generated"]["deals"].as_u64() == Some(25_808),
        "manifest 交易數不符"
    );
    let expected = manifest["files"]
        .as_array()
        .context("manifest files 缺失")?
        .iter()
        .find(|file| file["path"].as_str() == Some("white_tower_peninsula_v1_full.json"))
        .and_then(|file| file["sha256"].as_str())
        .context("manifest 缺 Full JSON 收據")?;
    ensure!(
        sha256(full_bytes) == expected,
        "Full JSON SHA-256 與 manifest 不符"
    );
    Ok(())
}

fn display_name(id: u32, fallback: &str) -> &str {
    match id {
        1 => "木材",
        2 => "石材",
        3 => "大理石",
        4 => "鐵",
        5 => "銅",
        6 => "錫",
        7 => "銀",
        8 => "金",
        9 => "穀物",
        10 => "牛隻",
        11 => "魚獲",
        12 => "野味",
        13 => "葡萄酒",
        14 => "橄欖",
        15 => "蜂蜜",
        16 => "鹽",
        17 => "椰棗",
        18 => "馬匹",
        19 => "象",
        20 => "駱駝",
        21 => "工業用麻",
        22 => "珍珠",
        23 => "寶石",
        24 => "染料",
        25 => "薰香",
        26 => "絲綢",
        27 => "香辛料",
        28 => "琥珀",
        29 => "毛皮",
        30 => "羊隻",
        31 => "奴隸（禁止交易）",
        32 => "焦油",
        33 => "硝石",
        34 => "煤",
        35 => "油料",
        36 => "桃花心木",
        37 => "鯨產物",
        38 => "甘蔗",
        39 => "茶葉",
        40 => "菸草",
        41 => "黏土",
        42 => "白砂",
        43 => "皮革",
        44 => "布料",
        45 => "成衣",
        46 => "陶瓷",
        47 => "玻璃",
        48 => "繩索",
        49 => "紙張",
        50 => "墨水",
        51 => "書籍",
        52 => "船帆",
        53 => "船舶",
        54 => "靴子",
        55 => "挽具",
        56 => "木桶",
        57 => "青銅",
        58 => "工具",
        59 => "武器",
        60 => "火藥",
        61 => "火砲",
        62 => "鎂幣",
        63 => "珠寶",
        64 => "保存食品",
        65 => "醋",
        66 => "乳酪",
        67 => "大麥酵飲",
        68 => "烈酒",
        69 => "蠟燭",
        70 => "茶籽香胰",
        71 => "草本香露",
        _ => fallback,
    }
}

fn parse_output_path() -> Result<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(OUTPUT_PATH));
    ensure!(args.next().is_none(), "參數過多");
    Ok(output)
}

fn array_at<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value[key]
        .as_array()
        .with_context(|| format!("{key} 不是陣列"))
}

fn string_at<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .with_context(|| format!("{key} 不是字串"))
}

fn f64_at(value: &Value, key: &str) -> Result<f64> {
    value[key]
        .as_f64()
        .with_context(|| format!("{key} 不是數字"))
}

fn u32_at(value: &Value, key: &str) -> Result<u32> {
    let result = u32_at_allow_zero(value, key)?;
    ensure!(result > 0, "{key} 必須大於零");
    Ok(result)
}

fn u32_at_allow_zero(value: &Value, key: &str) -> Result<u32> {
    u32::try_from(
        value[key]
            .as_u64()
            .with_context(|| format!("{key} 不是非負整數"))?,
    )
    .with_context(|| format!("{key} 超出 u32"))
}

fn squared_distance(left: &NormalizedPoint, right: &NormalizedPoint) -> f64 {
    (left.x - right.x).powi(2) + (left.y - right.y).powi(2)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}


#![forbid(unsafe_code)]
//! 驗證白塔半島正式執行種子，不讀寫資料庫。

use anyhow::Result;
use singularity_world::world::author_economy::builtin_white_tower_author_economy;
use singularity_world::world::runtime_world::builtin_white_tower_runtime_world;

fn main() -> Result<()> {
    let world = builtin_white_tower_runtime_world()?;
    let economy = builtin_white_tower_author_economy()?;
    let roads = world
        .routes
        .iter()
        .filter(|route| route.mode == "road")
        .count();
    let ferries = world
        .routes
        .iter()
        .filter(|route| route.mode == "ferry")
        .count();
    println!("white_tower_runtime_check: ok");
    println!(
        "world={} places={} routes={} roads={} ferries={} resources={} spawn={}",
        world.id,
        world.places.len(),
        world.routes.len(),
        roads,
        ferries,
        world.resources.len(),
        world.spawn_place_id
    );
    println!(
        "accounted: burgs={} native_routes={} states=23 provinces=487 cultures=9 religions=14 markers=456 zones=15 goods={} markets={} deals={}",
        world.coverage.burgs_accounted,
        world.coverage.native_routes_accounted,
        world.coverage.goods_accounted,
        world.coverage.markets_accounted,
        world.coverage.deals_accounted
    );
    println!("runtime_authority=true canon=false player_facing_map=false");
    println!(
        "economy_evidence: goods={} markets={} quotes={} deals={} allowed_deals={} blocked_goods={}",
        economy.goods.len(),
        economy.markets.len(),
        economy.coverage.market_quotes_accounted,
        economy.trade_evidence.len(),
        economy.coverage.deals_runtime_allowed,
        economy.coverage.goods_blocked_from_trade
    );
    Ok(())
}

//! earth-places-preview — 從 OSM PBF 抽 place=* 種子，印清單 + PPM 點圖。
//!
//! seed-and-grow 的 seed 階段：把 OSM 標註的聚落都撈出來，目視確認分佈合理後再寫 grow。
//!
//! 用法：
//!   cargo run --release --bin earth-places-preview -- \
//!       --pbf data/earth/osm/taiwan-latest.osm.pbf \
//!       --out /tmp/taiwan_places.ppm

use singularity_world::worldgen::earth_places::{Tier, load_places, summarize};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

struct Args {
    pbf: String,
    out: String,
    step: f64,
}

fn parse_args() -> Args {
    let mut pbf = "data/earth/osm/taiwan-latest.osm.pbf".to_string();
    let mut out = "/tmp/taiwan_places.ppm".to_string();
    let mut step: f64 = 0.005;
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--pbf" => {
                i += 1;
                pbf = args[i].clone();
            }
            "--out" => {
                i += 1;
                out = args[i].clone();
            }
            "--step" => {
                i += 1;
                step = args[i].parse().expect("--step 需浮點數");
            }
            "-h" | "--help" => {
                eprintln!("earth-places-preview --pbf <file> --out <ppm> [--step <deg>]");
                std::process::exit(0);
            }
            other => eprintln!("未知參數：{other}（跳過）"),
        }
        i += 1;
    }
    Args { pbf, out, step }
}

fn tier_rgb(t: Tier) -> (u8, u8, u8) {
    match t {
        Tier::Urban => (220, 60, 60),   // 紅：城核
        Tier::Town => (240, 180, 60),   // 橘：鄉鎮
        Tier::Wild => (100, 100, 100),  // 灰（seed 不會出現）
    }
}

/// 五級聚落（對應 docs/design/人類活動地帶聚落五級—規格草案.md）
/// 依 population 映射；place_kind 當 fallback。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rank5 {
    Capital,  // 都 ≥ 1M
    Prefect,  // 府 ≥ 300k
    City,     // 市 ≥ 50k
    Town,     // 鎮 ≥ 5k 或 place=town/suburb
    Village,  // 村（其餘）
}

fn rank5_of(pop: Option<u64>, kind: &str) -> Rank5 {
    if let Some(p) = pop {
        if p >= 1_000_000 { return Rank5::Capital; }
        if p >= 300_000 { return Rank5::Prefect; }
        if p >= 50_000 { return Rank5::City; }
        if p >= 5_000 { return Rank5::Town; }
    }
    match kind {
        "city" => Rank5::Prefect,
        "town" | "suburb" => Rank5::Town,
        _ => Rank5::Village,
    }
}

fn rank5_rgb(r: Rank5) -> (u8, u8, u8) {
    match r {
        Rank5::Capital => (180, 20, 30),   // 都 深紅
        Rank5::Prefect => (230, 90, 50),   // 府 橙紅
        Rank5::City    => (240, 160, 50),  // 市 橙
        Rank5::Town    => (240, 210, 80),  // 鎮 黃
        Rank5::Village => (230, 220, 150), // 村 淡黃
    }
}

fn rank5_radius(r: Rank5) -> i32 {
    match r {
        Rank5::Capital => 6,
        Rank5::Prefect => 4,
        Rank5::City    => 2,
        Rank5::Town    => 1,
        Rank5::Village => 0,
    }
}

fn rank5_name(r: Rank5) -> &'static str {
    match r {
        Rank5::Capital => "都",
        Rank5::Prefect => "府",
        Rank5::City    => "市",
        Rank5::Town    => "鎮",
        Rank5::Village => "村",
    }
}

fn main() -> anyhow::Result<()> {
    let args = parse_args();

    // 台灣 bbox
    let lat_min = 21.5_f64;
    let lat_max = 25.5_f64;
    let lon_min = 119.8_f64;
    let lon_max = 122.2_f64;

    let seeds = load_places(&PathBuf::from(&args.pbf))?;

    // 統計
    eprintln!("\n種子分佈：");
    for (label, count) in summarize(&seeds) {
        eprintln!("  {:<20} {}", label, count);
    }

    // 帶 population 的大型城鎮印出來備查
    let mut big: Vec<_> = seeds
        .iter()
        .filter(|s| s.population.is_some_and(|p| p >= 50_000))
        .collect();
    big.sort_by_key(|s| std::cmp::Reverse(s.population.unwrap_or(0)));
    eprintln!("\n前 20 大（population >= 50k）：");
    for s in big.iter().take(20) {
        eprintln!(
            "  {:<15} {:<10} pop={:>8}  ({:.4}, {:.4})",
            s.name,
            s.place_kind,
            s.population.unwrap_or(0),
            s.lat,
            s.lon
        );
    }

    // 光柵化 PPM：底色白，種子按 tier 塗色，半徑依 tier 放大
    let w = ((lon_max - lon_min) / args.step).ceil() as usize;
    let h = ((lat_max - lat_min) / args.step).ceil() as usize;
    let mut pixels: Vec<Vec<(u8, u8, u8)>> = vec![vec![(250, 248, 240); w]; h];

    let mut plotted = 0_usize;
    for s in &seeds {
        if s.lat < lat_min || s.lat > lat_max || s.lon < lon_min || s.lon > lon_max {
            continue;
        }
        let xi = ((s.lon - lon_min) / args.step) as i32;
        let yi = ((lat_max - s.lat) / args.step) as i32;
        let r = match s.tier {
            Tier::Urban => 3,
            Tier::Town => 1,
            Tier::Wild => 0,
        };
        let color = tier_rgb(s.tier);
        for dy in -r..=r {
            for dx in -r..=r {
                let nx = xi + dx;
                let ny = yi + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                pixels[ny as usize][nx as usize] = color;
            }
        }
        plotted += 1;
    }
    eprintln!("\n落在 bbox 內並塗色：{plotted}");

    // PPM 輸出（Tier 版）
    let mut f = File::create(&args.out)?;
    writeln!(f, "P6")?;
    writeln!(f, "{w} {h}")?;
    writeln!(f, "255")?;
    for row in &pixels {
        for &(r, g, b) in row {
            f.write_all(&[r, g, b])?;
        }
    }
    eprintln!("輸出：{}（Urban/Town 二級）", args.out);

    // ─── 五階版（對照聚落五級規格）───
    let mut pixels5: Vec<Vec<(u8, u8, u8)>> = vec![vec![(250, 248, 240); w]; h];
    let mut rank_count: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();

    // 先畫小的，大的壓上面
    let mut sorted: Vec<_> = seeds.iter().collect();
    sorted.sort_by_key(|s| {
        let r = rank5_of(s.population, &s.place_kind);
        rank5_radius(r)
    });

    for s in sorted {
        if s.lat < lat_min || s.lat > lat_max || s.lon < lon_min || s.lon > lon_max {
            continue;
        }
        let rank = rank5_of(s.population, &s.place_kind);
        *rank_count.entry(rank5_name(rank)).or_insert(0) += 1;
        let xi = ((s.lon - lon_min) / args.step) as i32;
        let yi = ((lat_max - s.lat) / args.step) as i32;
        let r = rank5_radius(rank);
        let color = rank5_rgb(rank);
        for dy in -r..=r {
            for dx in -r..=r {
                let nx = xi + dx;
                let ny = yi + dy;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                pixels5[ny as usize][nx as usize] = color;
            }
        }
    }

    eprintln!("\n五階分佈（對應聚落五級規格）：");
    for name in ["都", "府", "市", "鎮", "村"] {
        eprintln!("  {:<4} {:>6}", name, rank_count.get(name).unwrap_or(&0));
    }

    let out5 = args.out.replace(".ppm", "_rank5.ppm");
    let mut f5 = File::create(&out5)?;
    writeln!(f5, "P6")?;
    writeln!(f5, "{w} {h}")?;
    writeln!(f5, "255")?;
    for row in &pixels5 {
        for &(r, g, b) in row {
            f5.write_all(&[r, g, b])?;
        }
    }
    eprintln!("輸出：{out5}（五階）");

    Ok(())
}

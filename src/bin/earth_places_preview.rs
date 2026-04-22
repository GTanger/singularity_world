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

    // PPM 輸出
    let mut f = File::create(&args.out)?;
    writeln!(f, "P6")?;
    writeln!(f, "{w} {h}")?;
    writeln!(f, "255")?;
    for row in &pixels {
        for &(r, g, b) in row {
            f.write_all(&[r, g, b])?;
        }
    }
    eprintln!("輸出：{}", args.out);
    Ok(())
}

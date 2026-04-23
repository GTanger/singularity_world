//! worldgen-preview — 離線 WorldGen 視覺化工具。
//!
//! 用途：不接主遊戲、不落 PG，純跑 `worldgen::biome_at` 把一片地圖輸出到 stdout（ASCII）
//! 和可選 CSV。用來離線調參數、目視驗證地形邏輯。
//!
//! 用法：
//!   cargo run --release --bin worldgen-preview -- --seed 42 --size 80
//!   cargo run --release --bin worldgen-preview -- --seed 42 --size 120 --csv /tmp/map.csv

use singularity_world::grid::Terrain;
use singularity_world::worldgen;
use singularity_world::worldgen::FlowField;
use std::env;
use std::fs::File;
use std::io::Write;

fn terrain_glyph(t: Terrain) -> char {
    match t {
        Terrain::WaterDeep => '≈',
        Terrain::Water => '~',
        Terrain::Swamp => ',',
        Terrain::Tundra => '.',
        Terrain::Desert => '·',
        Terrain::Plain => '_',
        Terrain::Grassland => '"',
        Terrain::ForestLight => 'ţ',
        Terrain::Forest => 't',
        Terrain::ForestHeavy => 'T',
        Terrain::Jungle => '#',
        Terrain::Hills => 'n',
        Terrain::Mountain => '^',
        _ => '?',
    }
}

fn terrain_name(t: Terrain) -> &'static str {
    match t {
        Terrain::WaterDeep => "WaterDeep",
        Terrain::Water => "Water",
        Terrain::Swamp => "Swamp",
        Terrain::Tundra => "Tundra",
        Terrain::Desert => "Desert",
        Terrain::Plain => "Plain",
        Terrain::Grassland => "Grassland",
        Terrain::ForestLight => "ForestLight",
        Terrain::Forest => "Forest",
        Terrain::ForestHeavy => "ForestHeavy",
        Terrain::Jungle => "Jungle",
        Terrain::Hills => "Hills",
        Terrain::Mountain => "Mountain",
        _ => "Unknown",
    }
}

struct Args {
    seed: u32,
    size: i32,
    csv: Option<String>,
    ppm: Option<String>,
    /// 每格放大像素數（預設 4）
    scale: u32,
}

fn terrain_rgb(t: Terrain) -> (u8, u8, u8) {
    match t {
        Terrain::WaterDeep => (20, 40, 90),
        Terrain::Water => (60, 110, 180),
        Terrain::Swamp => (80, 100, 60),
        Terrain::Tundra => (220, 220, 230),
        Terrain::Desert => (220, 200, 140),
        Terrain::Plain => (200, 190, 130),
        Terrain::Grassland => (140, 180, 80),
        Terrain::ForestLight => (100, 160, 80),
        Terrain::Forest => (60, 130, 60),
        Terrain::ForestHeavy => (30, 90, 40),
        Terrain::Jungle => (0, 100, 50),
        Terrain::Hills => (160, 140, 90),
        Terrain::Mountain => (130, 110, 90),
        _ => (255, 0, 255),
    }
}

fn parse_args() -> Args {
    let mut seed: u32 = 42;
    let mut size: i32 = 80;
    let mut csv: Option<String> = None;
    let mut ppm: Option<String> = None;
    let mut scale: u32 = 4;
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                i += 1;
                seed = args[i].parse().expect("--seed 需整數");
            }
            "--size" => {
                i += 1;
                size = args[i].parse().expect("--size 需整數");
            }
            "--csv" => {
                i += 1;
                csv = Some(args[i].clone());
            }
            "--ppm" => {
                i += 1;
                ppm = Some(args[i].clone());
            }
            "--scale" => {
                i += 1;
                scale = args[i].parse().expect("--scale 需整數");
            }
            "-h" | "--help" => {
                eprintln!(
                    "worldgen-preview --seed <u32> --size <N> [--csv <path>] [--ppm <path>] [--scale <N>]"
                );
                std::process::exit(0);
            }
            other => eprintln!("未知參數：{other}（跳過）"),
        }
        i += 1;
    }
    Args { seed, size, csv, ppm, scale }
}

fn write_ppm(path: &str, pixels: &[Vec<(u8, u8, u8)>], scale: u32) -> std::io::Result<()> {
    let h = pixels.len() as u32;
    let w = pixels.first().map(|r| r.len()).unwrap_or(0) as u32;
    let out_w = w * scale;
    let out_h = h * scale;
    let mut f = File::create(path)?;
    // P6 binary PPM
    writeln!(f, "P6")?;
    writeln!(f, "{out_w} {out_h}")?;
    writeln!(f, "255")?;
    for j in 0..h {
        for _ in 0..scale {
            for i in 0..w {
                let (r, g, b) = pixels[j as usize][i as usize];
                for _ in 0..scale {
                    f.write_all(&[r, g, b])?;
                }
            }
        }
    }
    Ok(())
}

fn main() {
    let args = parse_args();
    let half = args.size / 2;

    // 地形分佈統計
    let mut histo: std::collections::BTreeMap<&'static str, u32> =
        std::collections::BTreeMap::new();

    // ASCII 地圖：y 由上往下、x 由左往右。y 軸代表緯度（y=0 赤道）。
    println!(
        "# WorldGen preview — seed={} size={}x{} range=[({},{})..({},{})]",
        args.seed,
        args.size,
        args.size,
        -half,
        -half,
        half - 1,
        half - 1
    );

    let mut csv_out = args.csv.as_ref().map(|p| {
        let mut f = File::create(p).expect("開 csv 失敗");
        writeln!(f, "x,y,terrain,elevation,moisture,temperature,flow").unwrap();
        f
    });

    // §3 水系：全圖預計算
    let field = FlowField::new(
        args.seed,
        -half,
        -half,
        args.size as usize,
        args.size as usize,
    );

    // PPM pixel buffer (每列 = y 軸)，用於可視化輸出
    let mut pixels: Vec<Vec<(u8, u8, u8)>> =
        Vec::with_capacity(args.size as usize);

    for y in -half..half {
        let mut line = String::with_capacity(args.size as usize);
        let mut row: Vec<(u8, u8, u8)> = Vec::with_capacity(args.size as usize);
        for x in -half..half {
            let t = field.biome_at(x, y);
            line.push(terrain_glyph(t));
            row.push(terrain_rgb(t));
            *histo.entry(terrain_name(t)).or_insert(0) += 1;

            if let Some(f) = csv_out.as_mut() {
                let e = worldgen::elevation_at(args.seed, x, y);
                let m = worldgen::moisture_at(args.seed, x, y);
                let temp = worldgen::temperature_at(args.seed, x, y, e);
                let flow = field.flow_at(x, y);
                writeln!(
                    f,
                    "{},{},{},{:.3},{:.3},{:.3},{}",
                    x,
                    y,
                    terrain_name(t),
                    e,
                    m,
                    temp,
                    flow
                )
                .unwrap();
            }
        }
        println!("{line}");
        pixels.push(row);
    }

    if let Some(p) = &args.ppm {
        match write_ppm(p, &pixels, args.scale) {
            Ok(()) => eprintln!("PPM 已輸出：{p}（{}x 放大）", args.scale),
            Err(e) => eprintln!("PPM 輸出失敗：{e}"),
        }
    }

    println!();
    println!("# terrain 分佈（{} 格）：", args.size * args.size);
    let total = (args.size * args.size) as f64;
    for (name, count) in &histo {
        let pct = *count as f64 / total * 100.0;
        println!("  {:<12} {:>5}  {:>5.1}%", name, count, pct);
    }

    if let Some(p) = &args.csv {
        eprintln!("CSV 已輸出：{p}");
    }
}

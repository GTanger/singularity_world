//! earth-import-preview — SRTM 高程 + OSM landuse 合併分類，輸出遊戲 Terrain PPM。
//!
//! 整條管線的離線驗證：兩種資料源融合後，每格被判定成什麼地形類型。
//! 尚未寫 PG，只產圖供目視確認分類策略是否合理。
//!
//! 用法：
//!   cargo run --release --bin earth-import-preview -- \
//!       --srtm data/earth/srtm --pbf data/earth/osm/taiwan-latest.osm.pbf \
//!       --out /tmp/taiwan_terrain.ppm --step 0.005

use singularity_world::hex::Terrain;
use singularity_world::worldgen::earth::{
    LanduseIndex, SrtmGrid, classify_terrain, load_landuse,
};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn terrain_rgb(t: Terrain) -> (u8, u8, u8) {
    match t {
        Terrain::WaterDeep => (20, 40, 90),
        Terrain::Water => (60, 110, 180),
        Terrain::Swamp => (80, 100, 60),
        Terrain::Tundra => (220, 220, 230),
        Terrain::Desert => (220, 200, 140),
        Terrain::Plain => (200, 80, 80), // 人類活動區=紅（Urban → Plain）
        Terrain::Grassland => (220, 200, 100), // 農田/草原=黃
        Terrain::ForestLight => (100, 160, 80),
        Terrain::Forest => (60, 130, 60),
        Terrain::ForestHeavy => (30, 90, 40),
        Terrain::Jungle => (0, 100, 50),
        Terrain::Hills => (160, 140, 90),
        Terrain::Mountain => (130, 110, 90),
        _ => (255, 0, 255),
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
    srtm: String,
    pbf: String,
    out: String,
    step: f64,
}

fn parse_args() -> Args {
    let mut srtm = "data/earth/srtm".to_string();
    let mut pbf = "data/earth/osm/taiwan-latest.osm.pbf".to_string();
    let mut out = "/tmp/taiwan_terrain.ppm".to_string();
    let mut step: f64 = 0.005;
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--srtm" => {
                i += 1;
                srtm = args[i].clone();
            }
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
                eprintln!(
                    "earth-import-preview --srtm <dir> --pbf <file> --out <ppm> [--step <deg>]"
                );
                std::process::exit(0);
            }
            other => eprintln!("未知參數：{other}（跳過）"),
        }
        i += 1;
    }
    Args { srtm, pbf, out, step }
}

fn main() {
    let args = parse_args();

    // 台灣 bbox（SRTM tile 涵蓋 N21-25、E120-122）
    let lat_min = 21.5_f64;
    let lat_max = 25.5_f64;
    let lon_min = 119.8_f64;
    let lon_max = 122.2_f64;

    // 載入 SRTM
    eprintln!("載入 SRTM…");
    let srtm = SrtmGrid::load(
        &PathBuf::from(&args.srtm),
        lat_min.floor() as i32,
        lat_max.ceil() as i32 - 1,
        lon_min.floor() as i32,
        lon_max.ceil() as i32 - 1,
    );
    eprintln!("  SRTM: {}×{}", srtm.height(), srtm.width());

    // 載入 OSM landuse
    let polygons = load_landuse(&PathBuf::from(&args.pbf)).expect("OSM 載入失敗");
    eprintln!("建 landuse 空間索引…");
    let index = LanduseIndex::build(polygons, lon_min, lat_min, lon_max, lat_max, 0.05);

    // 柵格化
    let w = ((lon_max - lon_min) / args.step).ceil() as usize;
    let h = ((lat_max - lat_min) / args.step).ceil() as usize;
    eprintln!("柵格化 {w}×{h}（step={}°）…", args.step);

    let mut pixels: Vec<Vec<(u8, u8, u8)>> = vec![vec![(0, 0, 0); w]; h];
    let mut histo: HashMap<&'static str, u32> = HashMap::new();

    for (j, row) in pixels.iter_mut().enumerate() {
        if j % 100 == 0 {
            eprintln!("  row {j}/{h}");
        }
        let lat = lat_max - (j as f64 + 0.5) * args.step;
        for (i, pixel) in row.iter_mut().enumerate() {
            let lon = lon_min + (i as f64 + 0.5) * args.step;
            let elev = srtm.elev_at(lat, lon);
            let landuse = index.query(lon, lat);
            let terrain = classify_terrain(elev, landuse);
            *pixel = terrain_rgb(terrain);
            *histo.entry(terrain_name(terrain)).or_insert(0) += 1;
        }
    }

    // 輸出 PPM
    let mut f = File::create(&args.out).expect("開 PPM 失敗");
    writeln!(f, "P6").unwrap();
    writeln!(f, "{w} {h}").unwrap();
    writeln!(f, "255").unwrap();
    for row in &pixels {
        for &(r, g, b) in row {
            f.write_all(&[r, g, b]).unwrap();
        }
    }
    eprintln!("輸出 PPM：{}", args.out);

    // 統計
    let total = (w * h) as f64;
    eprintln!("\nTerrain 分佈：");
    let mut entries: Vec<_> = histo.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    for (name, count) in entries {
        eprintln!(
            "  {:<12} {:>7}  {:>5.1}%",
            name,
            count,
            count as f64 / total * 100.0
        );
    }
}

//! earth-landuse-preview — 讀台灣 OSM PBF，抽取 landuse 多邊形，輸出 PPM 預覽。
//!
//! 兩趟讀取：
//!   Pass 1: 收集帶 landuse/natural 標籤的 Way 及其 node ID
//!   Pass 2: 收集所需 node 座標
//!   組裝 geo::Polygon → 柵格化 → PPM
//!
//! 用法：
//!   cargo run --release --bin earth-landuse-preview -- \
//!       --pbf data/earth/osm/taiwan-latest.osm.pbf --out /tmp/taiwan_landuse.ppm

use geo::Contains;
use geo::{Coord, Point, Polygon};
use osmpbf::{Element, ElementReader};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::Write;

/// 粗分類：OSM landuse/natural → 遊戲地形大類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LandClass {
    Urban,
    Farmland,
    Forest,
    Water,
    Grassland,
    Bare,
}

impl LandClass {
    fn rgb(self) -> (u8, u8, u8) {
        match self {
            LandClass::Urban => (200, 80, 80),
            LandClass::Farmland => (220, 200, 100),
            LandClass::Forest => (40, 120, 50),
            LandClass::Water => (60, 110, 180),
            LandClass::Grassland => (150, 190, 80),
            LandClass::Bare => (180, 170, 150),
        }
    }
}

fn classify_tags(tags: &[(String, String)]) -> Option<LandClass> {
    for (k, v) in tags {
        match k.as_str() {
            "landuse" => {
                return Some(match v.as_str() {
                    "residential" | "commercial" | "industrial" | "retail"
                    | "construction" | "railway" => LandClass::Urban,
                    "farmland" | "orchard" | "vineyard" | "allotments"
                    | "plant_nursery" | "aquaculture" => LandClass::Farmland,
                    "forest" => LandClass::Forest,
                    "reservoir" | "basin" => LandClass::Water,
                    "grass" | "meadow" | "recreation_ground" | "village_green"
                    | "cemetery" => LandClass::Grassland,
                    "quarry" | "landfill" | "brownfield" => LandClass::Bare,
                    _ => LandClass::Grassland, // fallback
                });
            }
            "natural" => {
                return Some(match v.as_str() {
                    "wood" => LandClass::Forest,
                    "water" => LandClass::Water,
                    "grassland" | "scrub" | "heath" => LandClass::Grassland,
                    "bare_rock" | "scree" | "sand" | "beach" => LandClass::Bare,
                    "wetland" => LandClass::Water,
                    _ => continue,
                });
            }
            _ => continue,
        }
    }
    None
}

struct Args {
    pbf: String,
    out: String,
    /// 每 pixel 代表的經緯度步長（度）。0.001° ≈ 111m
    step: f64,
}

fn parse_args() -> Args {
    let mut pbf = "data/earth/osm/taiwan-latest.osm.pbf".to_string();
    let mut out = "/tmp/taiwan_landuse.ppm".to_string();
    let mut step: f64 = 0.002; // ~222m/pixel
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
                eprintln!(
                    "earth-landuse-preview --pbf <path> --out <ppm> [--step <degrees>]"
                );
                std::process::exit(0);
            }
            other => eprintln!("未知參數：{other}（跳過）"),
        }
        i += 1;
    }
    Args { pbf, out, step }
}

struct LandWay {
    node_ids: Vec<i64>,
    class: LandClass,
}

fn main() {
    let args = parse_args();

    // ── Pass 1：收集 landuse Way ──
    eprintln!("Pass 1：掃描 Way（landuse/natural 標籤）…");
    let reader = ElementReader::from_path(&args.pbf).expect("開 PBF 失敗");

    let ways: Vec<LandWay> = reader
        .par_map_reduce(
            |element| {
                let mut batch = Vec::new();
                if let Element::Way(way) = element {
                    let tags: Vec<(String, String)> = way
                        .tags()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect();
                    if let Some(class) = classify_tags(&tags) {
                        let node_ids: Vec<i64> = way.refs().collect();
                        if node_ids.len() >= 3 {
                            batch.push(LandWay { node_ids, class });
                        }
                    }
                }
                batch
            },
            Vec::new,
            |mut a, b| {
                a.extend(b);
                a
            },
        )
        .expect("Pass 1 失敗");

    eprintln!("  找到 {} 個 landuse 多邊形", ways.len());

    // 收集需要的 node ID
    let needed: HashSet<i64> = ways.iter().flat_map(|w| w.node_ids.iter().copied()).collect();
    eprintln!("  需要 {} 個 node 座標", needed.len());

    // ── Pass 2：收集 Node 座標 ──
    eprintln!("Pass 2：收集 node 座標…");
    let reader2 = ElementReader::from_path(&args.pbf).expect("開 PBF 失敗");

    let coords: HashMap<i64, (f64, f64)> = reader2
        .par_map_reduce(
            |element| {
                let mut batch = HashMap::new();
                if let Element::DenseNode(node) = element {
                    let id = node.id();
                    if needed.contains(&id) {
                        batch.insert(id, (node.lon(), node.lat()));
                    }
                } else if let Element::Node(node) = element {
                    let id = node.id();
                    if needed.contains(&id) {
                        batch.insert(id, (node.lon(), node.lat()));
                    }
                }
                batch
            },
            HashMap::new,
            |mut a, b| {
                a.extend(b);
                a
            },
        )
        .expect("Pass 2 失敗");

    eprintln!("  收到 {} 個 node 座標", coords.len());

    // ── 組裝 Polygon ──
    eprintln!("組裝多邊形…");
    let mut polygons: Vec<(Polygon<f64>, LandClass)> = Vec::with_capacity(ways.len());
    let mut skipped = 0_usize;
    for w in &ways {
        let ring: Vec<Coord<f64>> = w
            .node_ids
            .iter()
            .filter_map(|id| coords.get(id).map(|&(lon, lat)| Coord { x: lon, y: lat }))
            .collect();
        if ring.len() < 3 {
            skipped += 1;
            continue;
        }
        let poly = Polygon::new(ring.into(), vec![]);
        polygons.push((poly, w.class));
    }
    if skipped > 0 {
        eprintln!("  跳過 {skipped} 個缺座標的 way");
    }
    eprintln!("  有效多邊形：{}", polygons.len());

    // ── 柵格化：台灣 bbox ──
    let lat_min = 21.5_f64;
    let lat_max = 25.5_f64;
    let lon_min = 119.8_f64;
    let lon_max = 122.2_f64;
    let step = args.step;
    let w = ((lon_max - lon_min) / step).ceil() as usize;
    let h = ((lat_max - lat_min) / step).ceil() as usize;
    eprintln!("柵格化 {w}×{h}（step={step}°）…");

    // 背景色（無分類）= 灰
    let bg: (u8, u8, u8) = (100, 100, 100);
    let mut pixels = vec![vec![bg; w]; h];

    // 為每個 pixel 中心點找第一個包含它的多邊形
    // 暴力 O(pixels × polygons)，台灣尺度可接受
    for (j, row) in pixels.iter_mut().enumerate() {
        if j % 100 == 0 {
            eprintln!("  row {j}/{h}");
        }
        let lat = lat_max - (j as f64 + 0.5) * step; // 圖上到下 = 北到南
        for (i, pixel) in row.iter_mut().enumerate() {
            let lon = lon_min + (i as f64 + 0.5) * step;
            let pt = Point::new(lon, lat);
            for (poly, class) in &polygons {
                if poly.contains(&pt) {
                    *pixel = class.rgb();
                    break;
                }
            }
        }
    }

    // ── 輸出 PPM ──
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
    let mut counts: HashMap<(u8, u8, u8), usize> = HashMap::new();
    for row in &pixels {
        for &rgb in row {
            *counts.entry(rgb).or_insert(0) += 1;
        }
    }
    let total = (w * h) as f64;
    eprintln!("\n分佈：");
    let labels = [
        (LandClass::Urban.rgb(), "Urban"),
        (LandClass::Farmland.rgb(), "Farmland"),
        (LandClass::Forest.rgb(), "Forest"),
        (LandClass::Water.rgb(), "Water"),
        (LandClass::Grassland.rgb(), "Grassland"),
        (LandClass::Bare.rgb(), "Bare"),
        (bg, "Unclassified"),
    ];
    for (rgb, name) in labels {
        let c = counts.get(&rgb).copied().unwrap_or(0);
        eprintln!("  {:<14} {:>7}  {:>5.1}%", name, c, c as f64 / total * 100.0);
    }
}

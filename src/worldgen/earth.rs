//! Earth import — SRTM 高程 + OSM landuse 的資料載入與分類。
//!
//! 三個子職：
//! - `SrtmGrid`：拼接 .hgt tile，按 (lat, lon) 查高程
//! - `load_landuse`：掃 OSM PBF 抽 landuse/natural 多邊形
//! - `classify`：(elev, landclass) → Terrain + Granularity

use crate::grid::Terrain;
use byteorder::{BigEndian, ReadBytesExt};
use geo::{Contains, Coord, Point, Polygon};
use osmpbf::{Element, ElementReader};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────
// §1 SRTM 高程
// ─────────────────────────────────────────────────────────────

/// SRTM GL1 每 tile 樣本數
const TILE_N: usize = 3601;
/// 無資料哨兵
const NODATA: i16 = -32768;

/// 拼接多個 .hgt tile 的高程網格；按 (lat, lon) 座標查詢。
pub struct SrtmGrid {
    /// 西南角（含）
    pub lat_min: i32,
    pub lon_min: i32,
    /// 東北角（含）— 緯度方向
    pub lat_max: i32,
    pub lon_max: i32,
    /// grid[row][col]，row=0 為 lat_max+1° 北緣
    grid: Vec<Vec<i16>>,
    /// 整幅高/寬（pixel）
    height: usize,
    width: usize,
}

impl SrtmGrid {
    /// 載入 bbox 內所有 tile。缺的 tile 以 NODATA 填。
    pub fn load(dir: &Path, lat_min: i32, lat_max: i32, lon_min: i32, lon_max: i32) -> Self {
        let lat_span = (lat_max - lat_min + 1) as usize;
        let lon_span = (lon_max - lon_min + 1) as usize;
        let height = lat_span * TILE_N;
        let width = lon_span * TILE_N;
        let mut grid = vec![vec![NODATA; width]; height];

        for lat in lat_min..=lat_max {
            for lon in lon_min..=lon_max {
                let name = format!("N{lat:02}E{lon:03}.hgt");
                let p: PathBuf = [dir, Path::new(&name)].iter().collect();
                if !p.exists() {
                    eprintln!("SRTM 缺 tile：{}（NODATA 填充）", p.display());
                    continue;
                }
                match read_tile(&p) {
                    Ok(tile) => {
                        let row_off = ((lat_max - lat) as usize) * TILE_N;
                        let col_off = ((lon - lon_min) as usize) * TILE_N;
                        for (dy, src) in tile.iter().enumerate() {
                            grid[row_off + dy][col_off..col_off + TILE_N]
                                .copy_from_slice(src);
                        }
                    }
                    Err(e) => eprintln!("SRTM 讀 {name} 失敗：{e}"),
                }
            }
        }

        Self {
            lat_min,
            lat_max,
            lon_min,
            lon_max,
            grid,
            height,
            width,
        }
    }

    /// 查某經緯度的高程（公尺）。超界或 NODATA 回 None。
    pub fn elev_at(&self, lat: f64, lon: f64) -> Option<i16> {
        // row 0 = (lat_max+1)°（最北）；每 TILE_N row 跨 1°
        let north = (self.lat_max + 1) as f64;
        let west = self.lon_min as f64;
        let row = ((north - lat) * TILE_N as f64) as i32;
        let col = ((lon - west) * TILE_N as f64) as i32;
        if row < 0 || col < 0 || row as usize >= self.height || col as usize >= self.width {
            return None;
        }
        let v = self.grid[row as usize][col as usize];
        if v == NODATA { None } else { Some(v) }
    }

    pub fn height(&self) -> usize {
        self.height
    }
    pub fn width(&self) -> usize {
        self.width
    }
}

fn read_tile(path: &Path) -> std::io::Result<Vec<Vec<i16>>> {
    let f = File::open(path)?;
    let mut r = BufReader::new(f);
    let mut buf = Vec::with_capacity(TILE_N * TILE_N * 2);
    r.read_to_end(&mut buf)?;
    let mut cur = Cursor::new(buf);
    let mut grid = vec![vec![0_i16; TILE_N]; TILE_N];
    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
            *cell = cur.read_i16::<BigEndian>()?;
        }
    }
    Ok(grid)
}

// ─────────────────────────────────────────────────────────────
// §2 OSM landuse
// ─────────────────────────────────────────────────────────────

/// OSM landuse/natural 粗分類（遊戲地形大類）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LandClass {
    Urban,
    Farmland,
    Forest,
    Water,
    Grassland,
    Bare,
}

impl LandClass {
    pub fn rgb(self) -> (u8, u8, u8) {
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

pub fn classify_osm_tags(tags: &[(String, String)]) -> Option<LandClass> {
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
                    _ => LandClass::Grassland,
                });
            }
            "natural" => {
                return Some(match v.as_str() {
                    "wood" => LandClass::Forest,
                    "water" | "wetland" => LandClass::Water,
                    "grassland" | "scrub" | "heath" => LandClass::Grassland,
                    "bare_rock" | "scree" | "sand" | "beach" => LandClass::Bare,
                    _ => continue,
                });
            }
            _ => continue,
        }
    }
    None
}

/// 兩趟讀取 PBF，抽出 landuse 多邊形。
pub fn load_landuse(pbf: &Path) -> anyhow::Result<Vec<(Polygon<f64>, LandClass)>> {
    struct LandWay {
        node_ids: Vec<i64>,
        class: LandClass,
    }

    eprintln!("OSM Pass 1：掃 Way…");
    let reader = ElementReader::from_path(pbf)?;
    let ways: Vec<LandWay> = reader.par_map_reduce(
        |element| {
            let mut batch = Vec::new();
            if let Element::Way(way) = element {
                let tags: Vec<(String, String)> = way
                    .tags()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                if let Some(class) = classify_osm_tags(&tags) {
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
    )?;
    eprintln!("  找到 {} 個多邊形", ways.len());

    let needed: HashSet<i64> = ways.iter().flat_map(|w| w.node_ids.iter().copied()).collect();
    eprintln!("  需要 {} 個 node 座標", needed.len());

    eprintln!("OSM Pass 2：收 Node 座標…");
    let reader2 = ElementReader::from_path(pbf)?;
    let coords: HashMap<i64, (f64, f64)> = reader2.par_map_reduce(
        |element| {
            let mut batch = HashMap::new();
            match element {
                Element::DenseNode(n) => {
                    let id = n.id();
                    if needed.contains(&id) {
                        batch.insert(id, (n.lon(), n.lat()));
                    }
                }
                Element::Node(n) => {
                    let id = n.id();
                    if needed.contains(&id) {
                        batch.insert(id, (n.lon(), n.lat()));
                    }
                }
                _ => {}
            }
            batch
        },
        HashMap::new,
        |mut a, b| {
            a.extend(b);
            a
        },
    )?;
    eprintln!("  收到 {} 個座標", coords.len());

    let mut out = Vec::with_capacity(ways.len());
    for w in &ways {
        let ring: Vec<Coord<f64>> = w
            .node_ids
            .iter()
            .filter_map(|id| coords.get(id).map(|&(lon, lat)| Coord { x: lon, y: lat }))
            .collect();
        if ring.len() < 3 {
            continue;
        }
        out.push((Polygon::new(ring.into(), vec![]), w.class));
    }
    eprintln!("  有效多邊形：{}", out.len());
    Ok(out)
}

// ─────────────────────────────────────────────────────────────
// §3 空間索引（格子桶加速 PIP）
// ─────────────────────────────────────────────────────────────

/// 把多邊形依 bbox 塞進格子桶，查詢時只檢查同桶的。
pub struct LanduseIndex {
    polygons: Vec<(Polygon<f64>, LandClass)>,
    /// buckets[bj][bi] = 該格涵蓋的多邊形 index
    buckets: Vec<Vec<Vec<u32>>>,
    /// bbox
    lon_min: f64,
    lat_min: f64,
    bucket_deg: f64,
    nx: usize,
    ny: usize,
}

impl LanduseIndex {
    pub fn build(
        polygons: Vec<(Polygon<f64>, LandClass)>,
        lon_min: f64,
        lat_min: f64,
        lon_max: f64,
        lat_max: f64,
        bucket_deg: f64,
    ) -> Self {
        let nx = ((lon_max - lon_min) / bucket_deg).ceil() as usize + 1;
        let ny = ((lat_max - lat_min) / bucket_deg).ceil() as usize + 1;
        let mut buckets: Vec<Vec<Vec<u32>>> = vec![vec![Vec::new(); nx]; ny];

        for (idx, (poly, _)) in polygons.iter().enumerate() {
            // 粗 bbox
            let mut xmin = f64::INFINITY;
            let mut xmax = f64::NEG_INFINITY;
            let mut ymin = f64::INFINITY;
            let mut ymax = f64::NEG_INFINITY;
            for c in poly.exterior().coords() {
                if c.x < xmin {
                    xmin = c.x;
                }
                if c.x > xmax {
                    xmax = c.x;
                }
                if c.y < ymin {
                    ymin = c.y;
                }
                if c.y > ymax {
                    ymax = c.y;
                }
            }
            let bi0 = ((xmin - lon_min) / bucket_deg).floor().max(0.0) as usize;
            let bi1 = ((xmax - lon_min) / bucket_deg).ceil() as usize;
            let bj0 = ((ymin - lat_min) / bucket_deg).floor().max(0.0) as usize;
            let bj1 = ((ymax - lat_min) / bucket_deg).ceil() as usize;
            let bj_end = bj1.min(ny - 1);
            let bi_end = bi1.min(nx - 1);
            #[allow(clippy::needless_range_loop)]
            for bj in bj0..=bj_end {
                for bi in bi0..=bi_end {
                    buckets[bj][bi].push(idx as u32);
                }
            }
        }

        Self {
            polygons,
            buckets,
            lon_min,
            lat_min,
            bucket_deg,
            nx,
            ny,
        }
    }

    /// 查 (lon, lat) 屬於哪個 LandClass；無則 None。
    pub fn query(&self, lon: f64, lat: f64) -> Option<LandClass> {
        let bi = ((lon - self.lon_min) / self.bucket_deg) as i32;
        let bj = ((lat - self.lat_min) / self.bucket_deg) as i32;
        if bi < 0 || bj < 0 || bi as usize >= self.nx || bj as usize >= self.ny {
            return None;
        }
        let pt = Point::new(lon, lat);
        for &idx in &self.buckets[bj as usize][bi as usize] {
            let (poly, class) = &self.polygons[idx as usize];
            if poly.contains(&pt) {
                return Some(*class);
            }
        }
        None
    }
}

// ─────────────────────────────────────────────────────────────
// §4 合併分類：(elev, landuse) → Terrain
// ─────────────────────────────────────────────────────────────

/// 合併高程 + landuse → 最終遊戲 Terrain。
///
/// 規則：
/// - 海拔 < 0 或 landuse=Water → Water
/// - landuse=Urban 且海拔 < 800 → Plain（代表有道路/建築的人類活動區）
/// - landuse=Farmland → Grassland（可耕地）
/// - landuse=Forest 或 landuse=None 海拔 200-1500 → Forest
/// - 海拔 > 2500 → Mountain
/// - 海拔 > 1000 → Hills
/// - landuse=Grassland → Grassland
/// - landuse=Bare 或海拔 > 2000 且未分類 → Hills
/// - fallback → Plain
pub fn classify_terrain(elev: Option<i16>, landuse: Option<LandClass>) -> Terrain {
    // 水優先
    if matches!(landuse, Some(LandClass::Water)) {
        return Terrain::Water;
    }
    // SRTM NODATA 且無 landuse → 海洋（OSM 只標陸地，SRTM 缺資料就是海）
    let e = match (elev, landuse) {
        (Some(v), _) => v,
        (None, None) => return Terrain::WaterDeep,
        (None, Some(_)) => 0, // 沿岸小島：有 landuse 但 SRTM 缺資料，當平地走 landuse
    };
    if e < 0 {
        return Terrain::WaterDeep;
    }
    // elev=0 + 無 landuse → 海（SRTM 海平面存 0，真陸地標高幾乎不會是正好 0）
    if e == 0 && landuse.is_none() {
        return Terrain::WaterDeep;
    }

    // 高海拔硬門
    if e > 2500 {
        return Terrain::Mountain;
    }
    if e > 1500 {
        return Terrain::Hills;
    }

    // 有 landuse 標註
    match landuse {
        Some(LandClass::Urban) => Terrain::Plain,
        Some(LandClass::Farmland) => Terrain::Grassland,
        Some(LandClass::Forest) => {
            if e > 800 {
                Terrain::ForestHeavy
            } else {
                Terrain::Forest
            }
        }
        Some(LandClass::Grassland) => Terrain::Grassland,
        Some(LandClass::Bare) => Terrain::Hills,
        Some(LandClass::Water) => Terrain::Water, // unreachable but keep exhaustive
        None => {
            // OSM 沒標 → 用海拔猜
            if e > 1000 {
                Terrain::Hills
            } else if e > 300 {
                Terrain::Forest
            } else {
                Terrain::Plain
            }
        }
    }
}

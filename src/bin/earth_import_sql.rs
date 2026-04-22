//! earth-import-sql — SRTM + OSM 融合後輸出 SQL COPY 資料檔。
//!
//! 輸出兩個檔：
//!   - <out_prefix>.tsv：TAB 分隔的 COPY 資料 (x, y, lat, lon, elev, terrain, landuse)
//!   - <out_prefix>.sql：包 COPY ... FROM STDIN 指令，可 psql -f 執行
//!
//! 不直接寫 PG，避免污染遊戲執行狀態。需要灌時手動 psql 執行。

use singularity_world::hex::Terrain;
use singularity_world::worldgen::earth::{
    LandClass, LanduseIndex, SrtmGrid, classify_terrain, load_landuse,
};
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

fn terrain_snake(t: Terrain) -> &'static str {
    match t {
        Terrain::WaterDeep => "water_deep",
        Terrain::Water => "water",
        Terrain::Swamp => "swamp",
        Terrain::Tundra => "tundra",
        Terrain::Desert => "desert",
        Terrain::Plain => "plain",
        Terrain::Grassland => "grassland",
        Terrain::ForestLight => "forest_light",
        Terrain::Forest => "forest",
        Terrain::ForestHeavy => "forest_heavy",
        Terrain::Jungle => "jungle",
        Terrain::Hills => "hills",
        Terrain::Mountain => "mountain",
        _ => "unknown",
    }
}

fn landuse_str(l: Option<LandClass>) -> &'static str {
    match l {
        Some(LandClass::Urban) => "urban",
        Some(LandClass::Farmland) => "farmland",
        Some(LandClass::Forest) => "forest",
        Some(LandClass::Water) => "water",
        Some(LandClass::Grassland) => "grassland",
        Some(LandClass::Bare) => "bare",
        None => "\\N", // PG NULL
    }
}

struct Args {
    srtm: String,
    pbf: String,
    out_prefix: String,
    step: f64,
}

fn parse_args() -> Args {
    let mut srtm = "data/earth/srtm".to_string();
    let mut pbf = "data/earth/osm/taiwan-latest.osm.pbf".to_string();
    let mut out_prefix = "/tmp/taiwan_grid".to_string();
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
            "--out-prefix" => {
                i += 1;
                out_prefix = args[i].clone();
            }
            "--step" => {
                i += 1;
                step = args[i].parse().expect("--step 需浮點數");
            }
            "-h" | "--help" => {
                eprintln!(
                    "earth-import-sql --srtm <dir> --pbf <file> --out-prefix <path> [--step <deg>]"
                );
                std::process::exit(0);
            }
            other => eprintln!("未知參數：{other}（跳過）"),
        }
        i += 1;
    }
    Args { srtm, pbf, out_prefix, step }
}

fn main() -> anyhow::Result<()> {
    let args = parse_args();

    let lat_min = 21.5_f64;
    let lat_max = 25.5_f64;
    let lon_min = 119.8_f64;
    let lon_max = 122.2_f64;

    eprintln!("載入 SRTM…");
    let srtm = SrtmGrid::load(
        &PathBuf::from(&args.srtm),
        lat_min.floor() as i32,
        lat_max.ceil() as i32 - 1,
        lon_min.floor() as i32,
        lon_max.ceil() as i32 - 1,
    );

    let polygons = load_landuse(&PathBuf::from(&args.pbf))?;
    eprintln!("建 landuse 空間索引…");
    let index = LanduseIndex::build(polygons, lon_min, lat_min, lon_max, lat_max, 0.05);

    let w = ((lon_max - lon_min) / args.step).ceil() as i32;
    let h = ((lat_max - lat_min) / args.step).ceil() as i32;
    eprintln!("寫入 {w}×{h} = {} cells…", w * h);

    // TSV
    let tsv_path = format!("{}.tsv", args.out_prefix);
    let sql_path = format!("{}.sql", args.out_prefix);
    let mut tsv = BufWriter::new(File::create(&tsv_path)?);
    let mut land_cells = 0_usize;
    let mut sea_cells = 0_usize;

    for yi in 0..h {
        if yi % 100 == 0 {
            eprintln!("  row {yi}/{h}");
        }
        let lat = lat_max - (yi as f64 + 0.5) * args.step;
        for xi in 0..w {
            let lon = lon_min + (xi as f64 + 0.5) * args.step;
            let elev = srtm.elev_at(lat, lon);
            let landuse = index.query(lon, lat);
            let terrain = classify_terrain(elev, landuse);

            // 過濾深海（太多且無遊戲意義）
            if terrain == Terrain::WaterDeep {
                sea_cells += 1;
                continue;
            }
            land_cells += 1;

            let elev_str = match elev {
                Some(v) if v != 0 => v.to_string(),
                _ => "\\N".to_string(),
            };
            // TSV: x, y, lat, lon, elev, terrain, landuse
            writeln!(
                tsv,
                "{}\t{}\t{:.5}\t{:.5}\t{}\t{}\t{}",
                xi,
                yi,
                lat,
                lon,
                elev_str,
                terrain_snake(terrain),
                landuse_str(landuse)
            )?;
        }
    }
    tsv.flush()?;
    eprintln!("TSV 輸出：{tsv_path}（{land_cells} 陸地 / {sea_cells} 深海濾掉）");

    // SQL 檔：建表 + COPY
    let sql = format!(
        r#"-- Earth import dump（台灣，step={step}°，~{res_m}m/cell）
-- 資料源：SRTM GL1 高程 + OSM Taiwan PBF landuse
-- 生成工具：earth-import-sql（見 src/bin/earth_import_sql.rs）

CREATE TABLE IF NOT EXISTS earth_grid_cells (
    x          INTEGER NOT NULL,
    y          INTEGER NOT NULL,
    lat        DOUBLE PRECISION NOT NULL,
    lon        DOUBLE PRECISION NOT NULL,
    elev_m     INTEGER,              -- NULL = SRTM NODATA / 海平面
    terrain    TEXT NOT NULL,        -- snake_case 遊戲 Terrain
    landuse    TEXT,                 -- OSM 分類：urban/farmland/forest/water/grassland/bare；NULL=未分類
    PRIMARY KEY (x, y)
);

CREATE INDEX IF NOT EXISTS idx_earth_grid_terrain ON earth_grid_cells (terrain);
CREATE INDEX IF NOT EXISTS idx_earth_grid_landuse ON earth_grid_cells (landuse);

-- bbox: lat [{lat_min}, {lat_max}], lon [{lon_min}, {lon_max}]
-- grid: {w}×{h}，{land_cells} 陸地 cells（深海已過濾）

TRUNCATE TABLE earth_grid_cells;
\copy earth_grid_cells (x, y, lat, lon, elev_m, terrain, landuse) FROM '{tsv_path}' WITH (FORMAT text, DELIMITER E'\t', NULL '\\N');
"#,
        step = args.step,
        res_m = (args.step * 111_000.0) as i32,
        lat_min = lat_min,
        lat_max = lat_max,
        lon_min = lon_min,
        lon_max = lon_max,
        w = w,
        h = h,
        land_cells = land_cells,
        tsv_path = tsv_path,
    );
    std::fs::write(&sql_path, sql)?;
    eprintln!("SQL 輸出：{sql_path}");
    eprintln!();
    eprintln!("灌庫指令：");
    eprintln!(
        "  docker exec -i postgres-singularity psql -U postgres -d singularity < {sql_path}"
    );
    Ok(())
}

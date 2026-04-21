//! earth-elevation-preview — 讀 SRTM .hgt tiles 輸出台灣高程預覽圖。
//!
//! 用途：驗證 SRTM 資料讀得對、座標對齊正確。不碰 PG、不接主遊戲。
//!
//! SRTM .hgt 格式：3601×3601 個 big-endian int16，代表一個 1°×1° 方塊
//! 的高程（公尺）。行 0 = 該格北緣；行 3600 = 南緣。每行 3601 個樣本，
//! 索引 0 = 西緣、索引 3600 = 東緣。邊界與鄰格重疊一格。
//!
//! 用法：
//!   cargo run --release --bin earth-elevation-preview -- \
//!       --dir data/earth/srtm --out /tmp/taiwan_elev.ppm --scale 1
//!
//! 輸出 PPM（用 `convert` 轉 PNG 看）。

use byteorder::{BigEndian, ReadBytesExt};
use std::env;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Write};
use std::path::PathBuf;

/// SRTM GL1 每 tile 樣本數（3601×3601）
const TILE_N: usize = 3601;
/// 沒資料的哨兵值
const NODATA: i16 = -32768;

struct Args {
    dir: String,
    out: String,
    /// 降採樣倍數：1=原 30m、5=150m、17≈500m
    downsample: usize,
}

fn parse_args() -> Args {
    let mut dir = "data/earth/srtm".to_string();
    let mut out = "/tmp/taiwan_elev.ppm".to_string();
    let mut downsample: usize = 5;
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                i += 1;
                dir = args[i].clone();
            }
            "--out" => {
                i += 1;
                out = args[i].clone();
            }
            "--downsample" => {
                i += 1;
                downsample = args[i].parse().expect("--downsample 需正整數");
            }
            "-h" | "--help" => {
                eprintln!(
                    "earth-elevation-preview --dir <srtm_dir> --out <ppm_path> [--downsample N]"
                );
                std::process::exit(0);
            }
            other => eprintln!("未知參數：{other}（跳過）"),
        }
        i += 1;
    }
    Args { dir, out, downsample }
}

/// 讀單一 tile。回傳 3601×3601 高程 grid（row=0 為北緣）。
fn read_tile(path: &PathBuf) -> std::io::Result<Vec<Vec<i16>>> {
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

fn main() {
    let args = parse_args();

    // 台灣範圍：lat 21–25N、lon 120–122E
    // 拼接 bbox：西南角 (21°N, 120°E)，東北角 (26°N, 123°E)
    // → 5 lat × 3 lon = 15 tiles 槽位（缺的填 NODATA）
    let lat_min = 21_i32;
    let lat_max = 25_i32; // inclusive
    let lon_min = 120_i32;
    let lon_max = 122_i32;
    let lat_span = (lat_max - lat_min + 1) as usize; // 5
    let lon_span = (lon_max - lon_min + 1) as usize; // 3

    // tile 之間北緣與南緣 row 重疊；簡化處理不去重，直接拼（每格佔 TILE_N 行，
    // 像素會有 lat_span 條水平線小瑕疵，不影響視覺驗證）。
    let big_h = lat_span * TILE_N;
    let big_w = lon_span * TILE_N;
    let mut big: Vec<Vec<i16>> = vec![vec![NODATA; big_w]; big_h];

    for lat in lat_min..=lat_max {
        for lon in lon_min..=lon_max {
            let name = format!("N{lat:02}E{lon:03}.hgt");
            let p: PathBuf = [&args.dir, &name].iter().collect();
            if !p.exists() {
                eprintln!("缺 tile：{}（視為全 NODATA）", p.display());
                continue;
            }
            match read_tile(&p) {
                Ok(grid) => {
                    // 緯度越高放越上面；圖 row=0 在最北
                    // 北界是 lat+1、南界是 lat。tile row=0 是北緣 = (lat+1)°
                    // 對應大圖 row = (lat_max - lat) * TILE_N
                    let row_off = ((lat_max - lat) as usize) * TILE_N;
                    let col_off = ((lon - lon_min) as usize) * TILE_N;
                    for (dy, src_row) in grid.iter().enumerate() {
                        big[row_off + dy][col_off..col_off + TILE_N]
                            .copy_from_slice(src_row);
                    }
                    eprintln!("讀入 {name}");
                }
                Err(e) => eprintln!("讀 {name} 失敗：{e}"),
            }
        }
    }

    // 降採樣 + 高程範圍統計
    let ds = args.downsample.max(1);
    let out_h = big_h / ds;
    let out_w = big_w / ds;
    let mut out_grid: Vec<Vec<i16>> = vec![vec![NODATA; out_w]; out_h];
    let mut min_e: i32 = i32::MAX;
    let mut max_e: i32 = i32::MIN;
    for j in 0..out_h {
        for i in 0..out_w {
            // 取 block 平均（忽略 NODATA）
            let mut sum = 0_i64;
            let mut count = 0_i64;
            for dj in 0..ds {
                for di in 0..ds {
                    let v = big[j * ds + dj][i * ds + di];
                    if v != NODATA {
                        sum += v as i64;
                        count += 1;
                    }
                }
            }
            if count > 0 {
                let avg = (sum / count) as i16;
                out_grid[j][i] = avg;
                min_e = min_e.min(avg as i32);
                max_e = max_e.max(avg as i32);
            }
        }
    }

    eprintln!(
        "拼接完成：{out_w}×{out_h} @ downsample={ds}，elevation range [{min_e}, {max_e}] m"
    );

    // 輸出 PPM P6。海=深藍、平地=綠、山=棕、高山=白
    let mut f = File::create(&args.out).expect("開 PPM 失敗");
    writeln!(f, "P6").unwrap();
    writeln!(f, "{out_w} {out_h}").unwrap();
    writeln!(f, "255").unwrap();
    for row in &out_grid {
        for &e in row {
            let (r, g, b) = if e == NODATA {
                (20, 40, 90) // 海（沒資料區）
            } else if e <= 0 {
                (40, 80, 140)
            } else if e < 100 {
                (180, 200, 130)
            } else if e < 500 {
                (120, 170, 90)
            } else if e < 1500 {
                (140, 130, 80)
            } else if e < 2500 {
                (170, 140, 100)
            } else if e < 3500 {
                (200, 180, 150)
            } else {
                (245, 245, 250)
            };
            f.write_all(&[r, g, b]).unwrap();
        }
    }
    eprintln!("輸出 PPM：{}", args.out);
}

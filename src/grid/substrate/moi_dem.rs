//! 內政部《全臺灣 20 公尺網格數值地形模型（DTM/DEM）》原生解析、雙線性內插與界河四連通加厚柵格化。
//!
//! 支援免外部 C/GDAL 依賴直接讀取內政部公開 `dem_20m.tif`（未壓縮 Little-Endian `i16` Strip GeoTIFF）
//! 與 `dem_20m.tfw`（TWD97 二度分帶公尺仿射定位檔）。

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// 內政部 20m DEM 缺值門檻（原始 GDAL_NODATA 為 `-32767`，任何 `< -500` 均視為無資料外海）。
pub const MOI_DEM_MIN_VALID_M: i16 = -500;

/// TWD97 World File (`.tfw`) 仿射定位參數。
#[derive(Debug, Clone, Copy)]
pub struct TfwGeoTransform {
    pub dx: f64,
    pub dy: f64,
    pub top_left_e: f64,
    pub top_left_n: f64,
}

impl TfwGeoTransform {
    pub fn from_tfw_file(path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        let nums: Vec<f64> = text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<f64>()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            })
            .collect::<io::Result<Vec<_>>>()?;
        if nums.len() < 6 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "TFW file requires 6 lines",
            ));
        }
        Ok(Self {
            dx: nums[0],
            dy: nums[3],
            top_left_e: nums[4],
            top_left_n: nums[5],
        })
    }
}

/// 內政部 20m DEM 記憶體視窗（支援全島或指定 TWD97 子視窗高速載入與雙線性內插）。
#[derive(Debug, Clone)]
pub struct Moi20mDemWindow {
    pub tfw: TfwGeoTransform,
    pub start_col: usize,
    pub start_row: usize,
    pub width: usize,
    pub height: usize,
    samples: Vec<i16>,
}

impl Moi20mDemWindow {
    /// 自 `dem_20m.tif` 與 `dem_20m.tfw` 僅載入涵蓋指定 TWD97 包絡框 `[min_e..=max_e, min_n..=max_n]`
    /// （自動外擴 4 格供雙線性內插邊界使用）之高程子矩陣，毫秒級完成且零多餘記憶體消耗。
    pub fn load_bbox(
        tif_path: &Path,
        tfw_path: &Path,
        min_e: f64,
        max_e: f64,
        min_n: f64,
        max_n: f64,
    ) -> io::Result<Self> {
        let tfw = TfwGeoTransform::from_tfw_file(tfw_path)?;
        let mut f = File::open(tif_path)?;

        let (full_w, full_h, strip_offsets) = parse_uncompressed_i16_tiff_strips(&mut f)?;

        let c0 = (((min_e - tfw.top_left_e) / tfw.dx).floor() as isize - 4).max(0) as usize;
        let c1 = (((max_e - tfw.top_left_e) / tfw.dx).ceil() as isize + 4)
            .clamp(0, (full_w - 1) as isize) as usize;

        let r0 = (((max_n - tfw.top_left_n) / tfw.dy).floor() as isize - 4).max(0) as usize;
        let r1 = (((min_n - tfw.top_left_n) / tfw.dy).ceil() as isize + 4)
            .clamp(0, (full_h - 1) as isize) as usize;

        let win_w = c1.saturating_sub(c0) + 1;
        let win_h = r1.saturating_sub(r0) + 1;
        let mut samples = vec![i16::MIN; win_w * win_h];
        let mut row_bytes = vec![0_u8; win_w * 2];

        for (out_r, src_r) in (r0..=r1).enumerate() {
            let byte_pos = strip_offsets[src_r] + (c0 * 2) as u64;
            f.seek(SeekFrom::Start(byte_pos))?;
            f.read_exact(&mut row_bytes)?;
            let dst_base = out_r * win_w;
            for i in 0..win_w {
                samples[dst_base + i] = i16::from_le_bytes([row_bytes[i * 2], row_bytes[i * 2 + 1]]);
            }
        }

        Ok(Self {
            tfw,
            start_col: c0,
            start_row: r0,
            width: win_w,
            height: win_h,
            samples,
        })
    }

    #[inline]
    fn sample_valid_at_global(&self, global_col: isize, global_row: isize) -> Option<f32> {
        let lc = global_col - (self.start_col as isize);
        let lr = global_row - (self.start_row as isize);
        if lc < 0 || lr < 0 || (lc as usize) >= self.width || (lr as usize) >= self.height {
            return None;
        }
        let raw = self.samples[(lr as usize) * self.width + (lc as usize)];
        if raw < MOI_DEM_MIN_VALID_M {
            None
        } else {
            Some(f32::from(raw))
        }
    }

    /// 在任意 TWD97 平面座標 `(e_m, n_m)` 執行雙線性內插（Bilinear Interpolation）。
    pub fn sample_bilinear_m(&self, e_m: f64, n_m: f64) -> Option<f32> {
        let fx = (e_m - self.tfw.top_left_e) / self.tfw.dx;
        let fy = (n_m - self.tfw.top_left_n) / self.tfw.dy;

        let c0 = fx.floor() as isize;
        let r0 = fy.floor() as isize;
        let tx = (fx - (c0 as f64)) as f32;
        let ty = (fy - (r0 as f64)) as f32;

        let q00 = self.sample_valid_at_global(c0, r0);
        let q10 = self.sample_valid_at_global(c0 + 1, r0);
        let q01 = self.sample_valid_at_global(c0, r0 + 1);
        let q11 = self.sample_valid_at_global(c0 + 1, r0 + 1);

        let weights = [
            ((1.0 - tx) * (1.0 - ty), q00),
            (tx * (1.0 - ty), q10),
            ((1.0 - tx) * ty, q01),
            (tx * ty, q11),
        ];

        let mut w_sum = 0.0_f32;
        let mut v_sum = 0.0_f32;
        for (w, opt_v) in weights {
            if let Some(v) = opt_v {
                w_sum += w;
                v_sum += w * v;
            }
        }
        if w_sum > 0.25 {
            Some(v_sum / w_sum)
        } else {
            None
        }
    }
}

/// 解析未壓縮單通道 Little-Endian `i16` Strip TIFF 的寬、高與每列 StripOffset。
fn parse_uncompressed_i16_tiff_strips(f: &mut File) -> io::Result<(usize, usize, Vec<u64>)> {
    let mut hdr = [0_u8; 8];
    f.read_exact(&mut hdr)?;
    if &hdr[0..2] != b"II" || u16::from_le_bytes([hdr[2], hdr[3]]) != 42 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected little-endian classic TIFF",
        ));
    }
    let ifd_off = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as u64;
    f.seek(SeekFrom::Start(ifd_off))?;

    let mut cnt_buf = [0_u8; 2];
    f.read_exact(&mut cnt_buf)?;
    let entry_count = u16::from_le_bytes(cnt_buf) as usize;

    let mut entries = vec![0_u8; entry_count * 12];
    f.read_exact(&mut entries)?;

    let mut width: Option<usize> = None;
    let mut height: Option<usize> = None;
    let mut compression: u16 = 1;
    let mut rows_per_strip: usize = 1;
    let mut strip_offsets_meta: Option<(u16, usize, u32)> = None;

    for i in 0..entry_count {
        let b = &entries[i * 12..(i + 1) * 12];
        let tag = u16::from_le_bytes([b[0], b[1]]);
        let typ = u16::from_le_bytes([b[2], b[3]]);
        let count = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize;
        let val_or_off = u32::from_le_bytes([b[8], b[9], b[10], b[11]]);

        let scalar_usize = || -> usize {
            if typ == 3 {
                u16::from_le_bytes([b[8], b[9]]) as usize
            } else {
                val_or_off as usize
            }
        };

        match tag {
            256 => width = Some(scalar_usize()),
            257 => height = Some(scalar_usize()),
            259 => compression = u16::from_le_bytes([b[8], b[9]]),
            273 => strip_offsets_meta = Some((typ, count, val_or_off)),
            278 => rows_per_strip = scalar_usize(),
            _ => {}
        }
    }

    if compression != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected uncompressed TIFF (1), got {compression}"),
        ));
    }
    let w = width.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing ImageWidth"))?;
    let h = height.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing ImageLength"))?;
    let (so_typ, so_cnt, so_off) = strip_offsets_meta
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing StripOffsets"))?;

    let mut raw_strips = Vec::with_capacity(so_cnt);
    if so_cnt == 1 {
        raw_strips.push(so_off as u64);
    } else {
        f.seek(SeekFrom::Start(so_off as u64))?;
        if so_typ == 4 {
            let mut buf = vec![0_u8; so_cnt * 4];
            f.read_exact(&mut buf)?;
            for c in buf.chunks_exact(4) {
                raw_strips.push(u32::from_le_bytes([c[0], c[1], c[2], c[3]]) as u64);
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected StripOffsets tag type",
            ));
        }
    }

    let mut row_offsets = Vec::with_capacity(h);
    let row_stride = (w * 2) as u64;
    for r in 0..h {
        let strip_idx = r / rows_per_strip;
        let in_strip = (r % rows_per_strip) as u64;
        row_offsets.push(raw_strips[strip_idx] + in_strip * row_stride);
    }
    Ok((w, h, row_offsets))
}

/// 將連續折線段 `(x0, y0) -> (x1, y1)` 以**四連通無斜縫（4-Connected Supercover）＋ 半徑加厚**柵格化。
pub fn rasterize_4conn_thick_segment<F>(x0: i32, y0: i32, x1: i32, y1: i32, radius: i32, mut plot: F)
where
    F: FnMut(i32, i32),
{
    let mut stamp = |cx: i32, cy: i32| {
        let r = radius.max(0);
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r + r {
                    plot(cx + dx, cy + dy);
                }
            }
        }
    };

    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x0;
    let mut y = y0;

    stamp(x, y);
    while x != x1 || y != y1 {
        let e2 = 2 * err;
        let step_x = e2 > -dy;
        let step_y = e2 < dx;
        if step_x && step_y {
            stamp(x + sx, y);
            stamp(x, y + sy);
        }
        if step_x {
            err -= dy;
            x += sx;
        }
        if step_y {
            err += dx;
            y += sy;
        }
        stamp(x, y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashSet, VecDeque};

    #[test]
    fn diagonal_river_is_strictly_4_connected_barrier() {
        let mut river = HashSet::new();
        rasterize_4conn_thick_segment(0, 0, 20, 20, 0, |x, y| {
            river.insert((x, y));
        });

        let mut visited = HashSet::new();
        let mut q = VecDeque::new();
        q.push_back((0, 15));
        visited.insert((0, 15));
        while let Some((x, y)) = q.pop_front() {
            for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if (0..=20).contains(&nx)
                    && (0..=20).contains(&ny)
                    && !river.contains(&(nx, ny))
                    && visited.insert((nx, ny))
                {
                    q.push_back((nx, ny));
                }
            }
        }
        assert!(
            !visited.contains(&(15, 0)),
            "四連通加厚水系必須徹底阻斷正交穿縫，不得有對角破口"
        );
    }
}

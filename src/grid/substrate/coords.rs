//! 能元台灣全島統一 TWD97 × `k = 2` 座標定位、底板包絡容器與邊緣感官角標推導。
//!
//! 對應正典：
//! - `docs/design/energetic_taiwan_landmass_and_substrate_spec.md` §1.2, §3.2
//! - `docs/design/people_and_relations_plugin_socket_spec.md` §6.5（地標名不動，感官退居格緣方位角標）

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

use super::l0_cell::{L0Cell, SurfaceWaterKind};

/// 西麓斷層湧泉原點 `(gx = 0, gy = 0)` 對應之 TWD97 二度分帶公尺座標（沙鹿竹林／番婆井坡腳折線）。
pub const ORIGIN_TWD97_E: f64 = 205_600.0;
pub const ORIGIN_TWD97_N: f64 = 2_681_000.0;

/// 全島唯一整數線性膨脹倍率 `k = 2`（面積 4 倍，開服鎖死不變）。
pub const EXPANSION_K: f64 = 2.0;
/// 遊戲內單格物理邊長（公尺）。
pub const CELL_SIZE_M: f64 = 20.0;
/// 每 1 遊戲格對應現實底圖公尺跨度（`20.0 / 2.0 = 10.0m`）。
pub const REAL_METERS_PER_CELL: f64 = CELL_SIZE_M / EXPANSION_K;

/// 二進位底板檔案魔術碼與格式版本（Little-Endian `256×256` 稀疏分塊）。
pub const SUBSTRATE_FILE_MAGIC: [u8; 4] = *b"SWL0";
pub const SUBSTRATE_FILE_VERSION: u16 = 1;
pub const CHUNK_SIDE: usize = 256;
pub const CHUNK_CELLS: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const CHUNK_BYTES: usize = CHUNK_CELLS * 6;

/// 現實 TWD97 平面公尺座標 `(E, N)` 轉為全島唯一整數格座標 `(gx, gy)`。
#[inline]
pub fn twd97_to_grid(e_m: f64, n_m: f64) -> (i32, i32) {
    let gx = ((e_m - ORIGIN_TWD97_E) / REAL_METERS_PER_CELL).round() as i32;
    let gy = ((n_m - ORIGIN_TWD97_N) / REAL_METERS_PER_CELL).round() as i32;
    (gx, gy)
}

/// 遊戲整數格座標 `(gx, gy)` 反算對應現實 TWD97 平面公尺中心點 `(E, N)`。
#[inline]
pub fn grid_to_twd97_center(gx: i32, gy: i32) -> (f64, f64) {
    let e_m = ORIGIN_TWD97_E + f64::from(gx) * REAL_METERS_PER_CELL;
    let n_m = ORIGIN_TWD97_N + f64::from(gy) * REAL_METERS_PER_CELL;
    (e_m, n_m)
}

/// WGS84 經緯度 `(lat_deg, lon_deg)` 轉台灣 TWD97 二度分帶（中央經線 121°E，GRS80 橢球）公尺座標 `(E, N)`。
pub fn wgs84_to_twd97(lat_deg: f64, lon_deg: f64) -> (f64, f64) {
    const A: f64 = 6_378_137.0;
    const INV_F: f64 = 298.257_222_101;
    const K0: f64 = 0.9999;
    const FALSE_EASTING: f64 = 250_000.0;

    let f = 1.0 / INV_F;
    let e2 = 2.0 * f - f * f;
    let ep2 = e2 / (1.0 - e2);

    let lat = lat_deg.to_radians();
    let dlon = (lon_deg - 121.0).to_radians();

    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let tan_lat = lat.tan();

    let n_curv = A / (1.0 - e2 * sin_lat * sin_lat).sqrt();
    let t = tan_lat * tan_lat;
    let c = ep2 * cos_lat * cos_lat;
    let big_a = cos_lat * dlon;

    let e4 = e2 * e2;
    let e6 = e4 * e2;
    let m_arc = A
        * ((1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0) * lat
            - (3.0 * e2 / 8.0 + 3.0 * e4 / 32.0 + 45.0 * e6 / 1024.0) * (2.0 * lat).sin()
            + (15.0 * e4 / 256.0 + 45.0 * e6 / 1024.0) * (4.0 * lat).sin()
            - (35.0 * e6 / 3072.0) * (6.0 * lat).sin());

    let big_a2 = big_a * big_a;
    let big_a3 = big_a2 * big_a;
    let big_a4 = big_a2 * big_a2;
    let big_a5 = big_a4 * big_a;
    let big_a6 = big_a4 * big_a2;

    let easting = FALSE_EASTING
        + K0 * n_curv
            * (big_a
                + (1.0 - t + c) * big_a3 / 6.0
                + (5.0 - 18.0 * t + t * t + 72.0 * c - 58.0 * ep2) * big_a5 / 120.0);
    let northing = K0
        * (m_arc
            + n_curv
                * tan_lat
                * (big_a2 / 2.0
                    + (5.0 - t + 9.0 * c + 4.0 * c * c) * big_a4 / 24.0
                    + (61.0 - 58.0 * t + t * t + 600.0 * c - 330.0 * ep2) * big_a6 / 720.0));
    (easting, northing)
}

/// 四鄰即時推導之地形坡度與坡向（不寫入底板存檔）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivedSlope {
    /// 東向高程梯度 `dz/dx`（無因次比值；正值代表東高西低）。
    pub dz_dx: f32,
    /// 北向高程梯度 `dz/dy`（無因次比值；正值代表北高南低）。
    pub dz_dy: f32,
    /// 合成坡度百分比（`0.05` 代表 5% 緩坡）。
    pub grade_ratio: f32,
}

/// 九宮格邊緣微感官方位角標（地標名不動，供荒野讀山看水尋跡）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensoryEdgeCue {
    pub label: &'static str,
    pub arrow: &'static str,
}

impl SensoryEdgeCue {
    pub fn compact_badge(&self) -> String {
        format!("{}{}", self.label, self.arrow)
    }
}

/// 單一區域／陸塊包絡框之常駐 `L0` 底板網格（支援負整數 `gx, gy` 直接定址）。
#[derive(Debug, Clone)]
pub struct SubstrateRegion {
    pub min_gx: i32,
    pub min_gy: i32,
    pub width: usize,
    pub height: usize,
    cells: Vec<L0Cell>,
}

impl SubstrateRegion {
    pub fn new_filled(min_gx: i32, min_gy: i32, width: usize, height: usize, fill: L0Cell) -> Self {
        Self {
            min_gx,
            min_gy,
            width,
            height,
            cells: vec![fill; width.checked_mul(height).expect("region size overflow")],
        }
    }

    #[inline]
    pub fn max_gx(&self) -> i32 {
        self.min_gx + (self.width as i32) - 1
    }

    #[inline]
    pub fn max_gy(&self) -> i32 {
        self.min_gy + (self.height as i32) - 1
    }

    #[inline]
    pub fn contains(&self, gx: i32, gy: i32) -> bool {
        gx >= self.min_gx && gx <= self.max_gx() && gy >= self.min_gy && gy <= self.max_gy()
    }

    #[inline]
    fn index_of(&self, gx: i32, gy: i32) -> Option<usize> {
        if !self.contains(gx, gy) {
            return None;
        }
        let ix = (gx - self.min_gx) as usize;
        let iy = (gy - self.min_gy) as usize;
        Some(iy * self.width + ix)
    }

    #[inline]
    pub fn get(&self, gx: i32, gy: i32) -> Option<L0Cell> {
        self.index_of(gx, gy).map(|idx| self.cells[idx])
    }

    #[inline]
    pub fn set(&mut self, gx: i32, gy: i32, cell: L0Cell) -> bool {
        if let Some(idx) = self.index_of(gx, gy) {
            self.cells[idx] = cell;
            true
        } else {
            false
        }
    }

    pub fn cells(&self) -> &[L0Cell] {
        &self.cells
    }

    /// 常駐記憶體位元組數（每格精確 6 Bytes）。
    pub fn memory_bytes(&self) -> usize {
        self.cells.len() * std::mem::size_of::<L0Cell>()
    }

    /// 由上下左右四鄰格高程差即時推算坡度與坡向（單格跨距 `20m`）。
    pub fn derive_slope(&self, gx: i32, gy: i32) -> Option<DerivedSlope> {
        let center = self.get(gx, gy)?.elevation_m()?;
        let west = self.get(gx - 1, gy).and_then(L0Cell::elevation_m).unwrap_or(center);
        let east = self.get(gx + 1, gy).and_then(L0Cell::elevation_m).unwrap_or(center);
        let south = self.get(gx, gy - 1).and_then(L0Cell::elevation_m).unwrap_or(center);
        let north = self.get(gx, gy + 1).and_then(L0Cell::elevation_m).unwrap_or(center);

        let span_x = if self.get(gx - 1, gy).and_then(L0Cell::elevation_m).is_some()
            && self.get(gx + 1, gy).and_then(L0Cell::elevation_m).is_some()
        {
            40.0
        } else {
            20.0
        };
        let span_y = if self.get(gx, gy - 1).and_then(L0Cell::elevation_m).is_some()
            && self.get(gx, gy + 1).and_then(L0Cell::elevation_m).is_some()
        {
            40.0
        } else {
            20.0
        };

        let dz_dx = (east - west) / span_x;
        let dz_dy = (north - south) / span_y;
        let grade_ratio = (dz_dx * dz_dx + dz_dy * dz_dy).sqrt();
        Some(DerivedSlope {
            dz_dx,
            dz_dy,
            grade_ratio,
        })
    }

    /// 推導荒野九宮格邊緣微感官方位角標（不覆蓋地標名，僅提示附近 1～3 格內最強之水聲／泉氣／海風梯度）。
    pub fn derive_sensory_edge_cue(&self, gx: i32, gy: i32) -> Option<SensoryEdgeCue> {
        let here = self.get(gx, gy)?;
        if here.surface_water() != SurfaceWaterKind::None {
            return None;
        }
        let mut best: Option<(i32, i32, i32, &'static str)> = None;
        for dy in -3..=3_i32 {
            for dx in -3..=3_i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let dist_sq = dx * dx + dy * dy;
                if dist_sq > 9 {
                    continue;
                }
                let Some(nb) = self.get(gx + dx, gy + dy) else {
                    continue;
                };
                let (prio, label) = match nb.surface_water() {
                    SurfaceWaterKind::FaultSpring => (40 - dist_sq, "泉涼"),
                    SurfaceWaterKind::DeepRapidRiver | SurfaceWaterKind::PerennialStream => {
                        (30 - dist_sq, "水聲")
                    }
                    SurfaceWaterKind::IntertidalShallow | SurfaceWaterKind::OpenSea => {
                        (20 - dist_sq, "鹹風")
                    }
                    _ => continue,
                };
                if best.is_none_or(|(bp, _, _, _)| prio > bp) {
                    best = Some((prio, dx, dy, label));
                }
            }
        }
        let (_, dx, dy, label) = best?;
        let arrow = direction_arrow(dx, dy);
        Some(SensoryEdgeCue { label, arrow })
    }

    /// 以 `256×256` 稀疏分塊小端序格式（`SWL0`）寫出底板檔（純外海無資料塊自動跳過不落盤）。
    pub fn save_sparse_chunks(&self, path: &Path) -> io::Result<usize> {
        let chunks_x = self.width.div_ceil(CHUNK_SIDE);
        let chunks_y = self.height.div_ceil(CHUNK_SIDE);

        let mut present_chunks: Vec<(u16, u16, Vec<u8>)> = Vec::new();
        let sea_default = L0Cell::sea_nodata();

        for cy in 0..chunks_y {
            for cx in 0..chunks_x {
                let mut all_sea = true;
                let mut buf = Vec::with_capacity(CHUNK_BYTES);
                for ly in 0..CHUNK_SIDE {
                    let gy = self.min_gy + (cy * CHUNK_SIDE + ly) as i32;
                    for lx in 0..CHUNK_SIDE {
                        let gx = self.min_gx + (cx * CHUNK_SIDE + lx) as i32;
                        let cell = self.get(gx, gy).unwrap_or(sea_default);
                        if cell != sea_default {
                            all_sea = false;
                        }
                        buf.extend_from_slice(&cell.to_le_bytes());
                    }
                }
                if !all_sea {
                    present_chunks.push((cx as u16, cy as u16, buf));
                }
            }
        }

        let mut w = BufWriter::new(File::create(path)?);
        w.write_all(&SUBSTRATE_FILE_MAGIC)?;
        w.write_all(&SUBSTRATE_FILE_VERSION.to_le_bytes())?;
        w.write_all(&self.min_gx.to_le_bytes())?;
        w.write_all(&self.min_gy.to_le_bytes())?;
        w.write_all(&(self.width as u32).to_le_bytes())?;
        w.write_all(&(self.height as u32).to_le_bytes())?;
        w.write_all(&(present_chunks.len() as u32).to_le_bytes())?;

        for (cx, cy, _) in &present_chunks {
            w.write_all(&cx.to_le_bytes())?;
            w.write_all(&cy.to_le_bytes())?;
        }
        for (_, _, data) in &present_chunks {
            w.write_all(data)?;
        }
        w.flush()?;
        Ok(present_chunks.len())
    }

    /// 由 `SWL0` 稀疏分塊底板檔完整還原 `SubstrateRegion`。
    pub fn load_sparse_chunks(path: &Path) -> io::Result<Self> {
        let mut r = BufReader::new(File::open(path)?);
        let mut magic = [0_u8; 4];
        r.read_exact(&mut magic)?;
        if magic != SUBSTRATE_FILE_MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "bad SWL0 magic"));
        }
        let mut u16_buf = [0_u8; 2];
        let mut i32_buf = [0_u8; 4];
        let mut u32_buf = [0_u8; 4];

        r.read_exact(&mut u16_buf)?;
        let version = u16::from_le_bytes(u16_buf);
        if version != SUBSTRATE_FILE_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported SWL0 version {version}"),
            ));
        }
        r.read_exact(&mut i32_buf)?;
        let min_gx = i32::from_le_bytes(i32_buf);
        r.read_exact(&mut i32_buf)?;
        let min_gy = i32::from_le_bytes(i32_buf);
        r.read_exact(&mut u32_buf)?;
        let width = u32::from_le_bytes(u32_buf) as usize;
        r.read_exact(&mut u32_buf)?;
        let height = u32::from_le_bytes(u32_buf) as usize;
        r.read_exact(&mut u32_buf)?;
        let present_count = u32::from_le_bytes(u32_buf) as usize;

        let mut coords = Vec::with_capacity(present_count);
        for _ in 0..present_count {
            r.read_exact(&mut u16_buf)?;
            let cx = u16::from_le_bytes(u16_buf) as usize;
            r.read_exact(&mut u16_buf)?;
            let cy = u16::from_le_bytes(u16_buf) as usize;
            coords.push((cx, cy));
        }

        let mut region = Self::new_filled(min_gx, min_gy, width, height, L0Cell::sea_nodata());
        let mut chunk_buf = vec![0_u8; CHUNK_BYTES];
        for (cx, cy) in coords {
            r.read_exact(&mut chunk_buf)?;
            for ly in 0..CHUNK_SIDE {
                let gy = min_gy + (cy * CHUNK_SIDE + ly) as i32;
                for lx in 0..CHUNK_SIDE {
                    let gx = min_gx + (cx * CHUNK_SIDE + lx) as i32;
                    let off = (ly * CHUNK_SIDE + lx) * 6;
                    let cell = L0Cell::from_le_bytes([
                        chunk_buf[off],
                        chunk_buf[off + 1],
                        chunk_buf[off + 2],
                        chunk_buf[off + 3],
                        chunk_buf[off + 4],
                        chunk_buf[off + 5],
                    ]);
                    region.set(gx, gy, cell);
                }
            }
        }
        Ok(region)
    }
}

fn direction_arrow(dx: i32, dy: i32) -> &'static str {
    match (dx.signum(), dy.signum()) {
        (0, 1) => "↑",
        (1, 1) => "↗",
        (1, 0) => "→",
        (1, -1) => "↘",
        (0, -1) => "↓",
        (-1, -1) => "↙",
        (-1, 0) => "←",
        (-1, 1) => "↖",
        _ => "·",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::l0_cell::{SubstrateMaterial, landmass};

    #[test]
    fn twd97_and_wgs84_origin_alignment() {
        assert_eq!(twd97_to_grid(ORIGIN_TWD97_E, ORIGIN_TWD97_N), (0, 0));
        let (e, n) = wgs84_to_twd97(24.234, 120.563);
        let (gx, gy) = twd97_to_grid(e, n);
        assert!(gx.abs() <= 5 && gy.abs() <= 5, "沙鹿竹林西麓應精準對齊原點附近: ({gx},{gy})");
    }

    #[test]
    fn sparse_chunk_roundtrip_skips_empty_sea_chunks() {
        let mut reg = SubstrateRegion::new_filled(-256, -256, 512, 512, L0Cell::sea_nodata());
        let land = L0Cell::new(
            Some(29.2),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::LateriteGravel,
            SurfaceWaterKind::FaultSpring,
            0.2,
            100,
        );
        let dry_neighbor = L0Cell::new(
            Some(30.0),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::LateriteGravel,
            SurfaceWaterKind::None,
            5.0,
            88,
        );
        assert!(reg.set(0, 0, land));
        assert!(reg.set(-1, -1, dry_neighbor));

        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.swl0");
        let written = reg.save_sparse_chunks(&p).unwrap();
        assert_eq!(written, 2, "4 個 256x256 區塊中僅 (0,0) 與 (-1,-1) 所在 2 塊落盤，純外海 2 塊跳過");

        let loaded = SubstrateRegion::load_sparse_chunks(&p).unwrap();
        assert_eq!(loaded.get(0, 0), Some(land));
        assert_eq!(loaded.get(-1, -1), Some(dry_neighbor));
        assert_eq!(loaded.get(-200, -200), Some(L0Cell::sea_nodata()));

        let cue = loaded.derive_sensory_edge_cue(-1, -1).unwrap();
        assert_eq!(cue.compact_badge(), "泉涼↗");
    }
}

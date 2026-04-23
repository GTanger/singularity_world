//! WorldGen — 主遊戲方格世界的地理生成層。
//!
//! 設計地基：
//! - **純函數、deterministic**：同 (seed, x, y) 永遠產同結果
//! - **不改任何狀態、不碰 PG**：離線可跑、CLI preview 可視化
//! - **介面替換 `terrain_from_seed` 獨立擲骰**，對外簽章穩定
//!
//! Pipeline 分層（本檔目前僅到 §2 地形；§3 水系、§4 聚落留給後續 iteration）：
//!   §1 高度圖：Perlin noise → elevation ∈ [-1, 1]
//!   §2 氣候派生：緯度（y 座標）+ elevation → 溫度 / 濕度 → Terrain
//!   §3 水系連通：低點沿梯度連成河/湖（未實作）
//!   §4 聚落派生：biome × 水系 × 適宜性（未實作）

pub mod earth;
pub mod earth_places;

use crate::grid::Terrain;
use noise::{NoiseFn, Perlin, Seedable};

/// 高度查詢——Perlin 三層疊加（continental / regional / local）。
///
/// 輸入格座標 (x, y)；輸出 elevation ∈ [-1, 1]。-1 為最深海、+1 為最高山。
/// 純函數，同 seed 同座標永遠同結果。
pub fn elevation_at(seed: u32, x: i32, y: i32) -> f64 {
    let base = Perlin::new(seed);
    let regional = Perlin::new(seed.wrapping_add(1));
    let local = Perlin::new(seed.wrapping_add(2));

    // 三層 octave：大陸尺度 / 區域尺度 / 細部
    let fx = x as f64;
    let fy = y as f64;

    let continental = base.get([fx / 96.0, fy / 96.0]) * 1.00;
    let regional_v = regional.get([fx / 32.0, fy / 32.0]) * 0.35;
    let local_v = local.get([fx / 10.0, fy / 10.0]) * 0.15;

    let raw = continental + regional_v + local_v;
    // 鉗制到 [-1, 1]，Perlin 理論值域就在附近
    raw.clamp(-1.0, 1.0)
}

/// 濕度查詢——獨立 Perlin，seed 偏移避免與高度同相位。
///
/// 輸出 ∈ [0, 1]。0 最乾、1 最濕。
pub fn moisture_at(seed: u32, x: i32, y: i32) -> f64 {
    let p = Perlin::new(seed.wrapping_add(100));
    let fx = x as f64;
    let fy = y as f64;
    let v = p.get([fx / 64.0, fy / 64.0]) * 0.75
        + p.get([fx / 16.0, fy / 16.0]) * 0.25;
    // [-1, 1] → [0, 1]
    ((v + 1.0) / 2.0).clamp(0.0, 1.0)
}

/// 溫度派生——緯度 + 海拔懲罰。
///
/// y 假設為緯度軸：y=0 赤道，|y| 越大越冷。世界尺度目前暫以 ±128 為極圈。
/// 輸出 ∈ [0, 1]，0 極寒、1 赤道炙熱。
pub fn temperature_at(_seed: u32, x: i32, y: i32, elevation: f64) -> f64 {
    // 緯度衰減
    let lat_norm = (y.abs() as f64 / 128.0).clamp(0.0, 1.0);
    let lat_temp = 1.0 - lat_norm;
    // 海拔懲罰（高山冷）
    let alt_penalty = if elevation > 0.0 {
        elevation * 0.5
    } else {
        0.0
    };
    let _ = x; // 未使用但保留介面對稱（未來或引入洋流/季風）
    (lat_temp - alt_penalty).clamp(0.0, 1.0)
}

/// 地形派生——elevation × temperature × moisture → Terrain。
///
/// 閾值規則採 Whittaker biome 模型簡化版：
///   海拔決定大類（深海/淺海/陸地/山）
///   陸地內再用溫度+濕度分 biome
pub fn biome_at(seed: u32, x: i32, y: i32) -> Terrain {
    let elev = elevation_at(seed, x, y);

    // §1 海拔大類
    if elev < -0.30 {
        return Terrain::WaterDeep;
    }
    if elev < -0.05 {
        return Terrain::Water;
    }
    if elev > 0.65 {
        return Terrain::Mountain;
    }
    if elev > 0.40 {
        return Terrain::Hills;
    }

    // §2 陸地 biome：溫度 × 濕度
    let temp = temperature_at(seed, x, y, elev);
    let moist = moisture_at(seed, x, y);

    // 寒帶
    if temp < 0.25 {
        return Terrain::Tundra;
    }
    // 乾燥熱帶
    if temp > 0.75 && moist < 0.25 {
        return Terrain::Desert;
    }
    // 濕熱叢林
    if temp > 0.75 && moist > 0.70 {
        return Terrain::Jungle;
    }
    // 溫帶多雨 → 森林梯度
    if moist > 0.65 {
        return Terrain::ForestHeavy;
    }
    if moist > 0.50 {
        return Terrain::Forest;
    }
    if moist > 0.35 {
        return Terrain::ForestLight;
    }
    // 濕地
    if moist > 0.25 && elev < 0.05 {
        return Terrain::Swamp;
    }
    // 溫帶乾草原
    if temp > 0.55 {
        return Terrain::Grassland;
    }
    // fallback
    Terrain::Plain
}

/// 確認 Perlin seedable 介面可用（避免 unused import 警告）。
#[allow(dead_code)]
fn _sanity() {
    let _ = Perlin::new(0).set_seed(1);
}

// ─────────────────────────────────────────────────────────────
// §3 水系連通：FlowField 全圖預計算
// ─────────────────────────────────────────────────────────────

/// FlowField — 對一片矩形區域預計算流量累積，產生河/湖。
///
/// 設計：`biome_at` 是純函數，只看單格；但水系需要全圖視野。
/// 本結構包住「指定 bounds 內的 elevation grid + flow accumulation」，
/// 對外提供 [`FlowField::biome_at`]：先查水系覆寫，再 fallback 純 biome。
///
/// 離線 CLI 直接用；未來接主遊戲時再設計 region-based cache。
pub struct FlowField {
    pub seed: u32,
    /// 區塊左下角 (min_x, min_y)
    pub origin: (i32, i32),
    pub width: usize,
    pub height: usize,
    /// elevation[y][x]（y 由下往上、x 由左往右，index 已轉相對原點）
    elev: Vec<Vec<f64>>,
    /// 流量累積（每格匯集的上游格數）
    flow: Vec<Vec<u32>>,
    /// 內陸低點（封閉盆地，變湖）
    lake_mask: Vec<Vec<bool>>,
}

impl FlowField {
    /// 全圖預計算。範圍 [min_x..min_x+width) × [min_y..min_y+height)
    pub fn new(seed: u32, min_x: i32, min_y: i32, width: usize, height: usize) -> Self {
        // §1 elevation grid
        let mut elev = vec![vec![0.0_f64; width]; height];
        for (j, row) in elev.iter_mut().enumerate() {
            for (i, cell) in row.iter_mut().enumerate() {
                *cell = elevation_at(seed, min_x + i as i32, min_y + j as i32);
            }
        }

        // §2 D8 流向：對每個陸地格找最陡鄰格（8 向）
        // downstream[j][i] = (di, dj) 指向下游，若自己是 sink 則 (0, 0)
        let mut downstream = vec![vec![(0_i32, 0_i32); width]; height];
        let neighbors: [(i32, i32); 8] = [
            (-1, -1), (0, -1), (1, -1),
            (-1, 0),           (1, 0),
            (-1, 1),  (0, 1),  (1, 1),
        ];
        for j in 0..height {
            for i in 0..width {
                let e = elev[j][i];
                // 海（含淺水）直接吸收，不參與陸地流向
                if e < -0.05 {
                    continue;
                }
                let mut best = e;
                let mut best_dir = (0_i32, 0_i32);
                for (dx, dy) in neighbors {
                    let ni = i as i32 + dx;
                    let nj = j as i32 + dy;
                    if ni < 0 || nj < 0 || ni >= width as i32 || nj >= height as i32 {
                        continue;
                    }
                    let ne = elev[nj as usize][ni as usize];
                    if ne < best {
                        best = ne;
                        best_dir = (dx, dy);
                    }
                }
                downstream[j][i] = best_dir;
            }
        }

        // §3 flow accumulation：拓樸排序（高→低）後逐格累加
        // 簡化：多次掃描直到收斂（陸地格數不大）
        let mut flow = vec![vec![1_u32; width]; height];
        // 依 elevation 降序排列 index
        let mut idx: Vec<(usize, usize)> = (0..height)
            .flat_map(|j| (0..width).map(move |i| (i, j)))
            .collect();
        idx.sort_by(|a, b| elev[b.1][b.0].partial_cmp(&elev[a.1][a.0]).unwrap());
        for (i, j) in idx {
            let (di, dj) = downstream[j][i];
            if di == 0 && dj == 0 {
                continue;
            }
            let ni = (i as i32 + di) as usize;
            let nj = (j as i32 + dj) as usize;
            flow[nj][ni] += flow[j][i];
        }

        // §4 lake detect：陸地內的 sink（無下游且非海）= 湖
        let mut lake_mask = vec![vec![false; width]; height];
        for j in 0..height {
            for i in 0..width {
                let e = elev[j][i];
                if !(-0.05..=0.40).contains(&e) {
                    continue; // 海或山不會是湖
                }
                if downstream[j][i] == (0, 0) {
                    // 檢查鄰居是否都高於自己（真 sink）
                    let mut is_sink = true;
                    for (dx, dy) in neighbors {
                        let ni = i as i32 + dx;
                        let nj = j as i32 + dy;
                        if ni < 0 || nj < 0 || ni >= width as i32 || nj >= height as i32 {
                            continue;
                        }
                        if elev[nj as usize][ni as usize] <= e {
                            is_sink = false;
                            break;
                        }
                    }
                    if is_sink {
                        lake_mask[j][i] = true;
                    }
                }
            }
        }

        Self {
            seed,
            origin: (min_x, min_y),
            width,
            height,
            elev,
            flow,
            lake_mask,
        }
    }

    /// 查詢某格最終 biome（含水系覆寫）
    pub fn biome_at(&self, x: i32, y: i32) -> Terrain {
        let i = (x - self.origin.0) as usize;
        let j = (y - self.origin.1) as usize;
        if i >= self.width || j >= self.height {
            return biome_at(self.seed, x, y);
        }

        // 湖覆寫
        if self.lake_mask[j][i] {
            return Terrain::Water;
        }

        // 河覆寫（流量超閾值 + 陸地）
        let e = self.elev[j][i];
        if e >= -0.05 {
            // 閾值：流量 >= 40 認定為河
            // 80x80 陸地總格 ~4000，主河流量破百合理
            if self.flow[j][i] >= 40 {
                return Terrain::Water;
            }
        }

        // fallback：純 biome
        biome_at(self.seed, x, y)
    }

    /// 除錯用：取流量值
    #[allow(dead_code)]
    pub fn flow_at(&self, x: i32, y: i32) -> u32 {
        let i = (x - self.origin.0) as usize;
        let j = (y - self.origin.1) as usize;
        if i >= self.width || j >= self.height {
            return 0;
        }
        self.flow[j][i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_same_seed_same_result() {
        let a = biome_at(42, 10, 10);
        let b = biome_at(42, 10, 10);
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_differ_somewhere() {
        // 不保證每個點都不同，但 100 格取樣內應該有差異
        let mut diff = 0;
        for i in 0..100 {
            if biome_at(1, i, 0) != biome_at(2, i, 0) {
                diff += 1;
            }
        }
        assert!(diff > 0, "不同 seed 產出 100 格完全一致，噪聲沒起作用");
    }

    #[test]
    fn elevation_in_range() {
        for i in -20..20 {
            for j in -20..20 {
                let e = elevation_at(42, i, j);
                assert!((-1.0..=1.0).contains(&e), "elevation 超出 [-1,1]: {e}");
            }
        }
    }
}

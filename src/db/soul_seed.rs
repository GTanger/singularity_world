//! soul_seed → 三軸（能階／時脈／相位）→ 屬性／性格／戰鬥係數／拓撲成本 的純展開邏輯。
//!
//! 此模組無 store/pg 依賴，純數學；與 `db/mod.rs` 共用的外部 API 以 `pub use` 再匯出。

// 三軸區間與映射常數（與人物屬性彙整 §二 一致）
const AMP_MIN: f64 = 0.1;
const AMP_MAX: f64 = 3.0;
const FREQ_MIN: f64 = 0.5;
const FREQ_MAX: f64 = 2.0;
const PHASE_MIN: f64 = -1.0;
const PHASE_MAX: f64 = 1.0;
const BASE_STAT: f64 = 10.0;
const K_AMP: f64 = 0.2;
const K_FREQ: f64 = 0.2;
const K_PHASE: f64 = 0.2;
const MIN_STAT: i32 = 1;
const MAX_STAT: i32 = 30;

/// 簡易 LCG 偽隨機（對齊既有 math/rand）。
pub(super) struct SimpleLcg {
    state: i64,
}

impl SimpleLcg {
    pub(super) fn new(seed: i64) -> Self {
        Self { state: seed }
    }

    fn next_i64(&mut self) -> i64 {
        // 標準庫 PRNG 的 rngSource 使用的是更複雜的演算法，
        // 但對於 soul_seed 展開只要一致性即可，後續可精確對齊。
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    pub(super) fn next_f64(&mut self) -> f64 {
        let v = self.next_i64();
        // 取高 53 位元映射到 [0, 1)
        let bits = (v as u64) >> 11;
        bits as f64 / (1u64 << 53) as f64
    }
}

/// 由 seed 前 3 次偽隨機展開三軸（能階、時脈、相位）。
fn expand_seed_axes(seed: i64) -> (f64, f64, f64) {
    let mut rng = SimpleLcg::new(seed);
    let u1 = rng.next_f64();
    let u2 = rng.next_f64();
    let u3 = rng.next_f64();
    let amp = AMP_MIN + u1 * (AMP_MAX - AMP_MIN);
    let freq = FREQ_MIN + u2 * (FREQ_MAX - FREQ_MIN);
    let phase = PHASE_MIN + u3 * (PHASE_MAX - PHASE_MIN);
    (amp, freq, phase)
}

/// 產生加密安全的 soul_seed（i64）。
pub fn generate_soul_seed() -> i64 {
    let mut buf = [0u8; 8];
    if getrandom_fill(&mut buf).is_err() {
        // fallback: 系統時間
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        return t as i64;
    }
    i64::from_be_bytes(buf)
}

fn getrandom_fill(buf: &mut [u8]) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open("/dev/urandom")?;
    f.read_exact(buf)?;
    Ok(())
}

/// 由 soul_seed 展開為基礎體質／氣脈／靈敏。
pub fn expand_soul_seed_to_base_stats(seed: i64) -> (i32, i32, i32) {
    let (amp, freq, phase) = expand_seed_axes(seed);
    let v = (BASE_STAT * (1.0 + K_AMP * (amp - 1.0))).round() as i32;
    let q = (BASE_STAT * (1.0 + K_FREQ * (freq - 1.0))).round() as i32;
    let d = (BASE_STAT * (1.0 + K_PHASE * phase)).round() as i32;
    (
        v.clamp(MIN_STAT, MAX_STAT),
        q.clamp(MIN_STAT, MAX_STAT),
        d.clamp(MIN_STAT, MAX_STAT),
    )
}

/// 四項資源最大值。
pub struct ResourceMaxes {
    pub hp_max: f64,
    pub inner_max: f64,
    pub spirit_max: f64,
    pub stamina_max: f64,
}

/// 由體質、氣脈、靈敏計算四項資源最大值。
pub fn compute_resource_maxes(vit: i32, qi: i32, dex: i32) -> ResourceMaxes {
    let (v, q, d) = (vit as f64, qi as f64, dex as f64);
    ResourceMaxes {
        hp_max: ((0.7 * v + 0.3 * q) * (0.05 * d + 1.0) * v).ceil(),
        inner_max: ((0.7 * q + 0.3 * v) * (0.05 * d + 1.0) * q).ceil(),
        spirit_max: ((0.6 * d + 0.4 * q) * (0.05 * v + 1.0) * d).ceil(),
        stamina_max: ((0.5 * v + 0.4 * q + 0.3 * d) * ((v + q + d) / 3.0)).ceil(),
    }
}

/// 由三軸產出本源語句。
pub fn generate_origin_sentence(amp: f64, freq: f64, phase: f64) -> String {
    let amp_word = if amp < 1.0 {
        "幽微"
    } else if amp > 2.0 {
        "霸道"
    } else {
        "綿長"
    };
    let freq_word = if freq < 1.0 {
        "渾厚"
    } else if freq > 1.6 {
        "敏銳"
    } else {
        "洞察"
    };
    let phase_word = if phase < -0.3 {
        "混沌"
    } else if phase > 0.3 {
        "秩序"
    } else {
         "順流"
     };
    let phase_desc = if phase_word == "順流" {
        "順流而行的澄明氣韻"
    } else {
        "逆流而上的孤高氣韻"
    };
    format!("你的心神感知{amp_word}且{freq_word}，隱隱透著一股{phase_desc}。")
}

/// 由 soul_seed 展開為本源語句。
pub fn expand_soul_seed_to_origin_sentence(seed: i64) -> String {
    let (amp, freq, phase) = expand_seed_axes(seed);
    generate_origin_sentence(amp, freq, phase)
}

/// 性格維度（皆 [0,1]），供決策／對話權重使用。
#[derive(Debug, Clone)]
pub struct Personality {
    pub boldness: f64,
    pub sensitivity: f64,
    pub orderliness: f64,
}

/// 由 soul_seed 展開為性格維度（先天心性單純形守恆：boldness + sensitivity + orderliness == 1.50，各軸 [0.12, 0.88]）。
pub fn expand_soul_seed_to_personality(seed: i64) -> Personality {
    let (amp, freq, phase) = expand_seed_axes(seed);
    let norm = |v: f64, lo: f64, hi: f64| -> f64 { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) };
    let b_raw = 0.15 + 0.70 * norm(amp, AMP_MIN, AMP_MAX);
    let s_raw = 0.15 + 0.70 * norm(freq, FREQ_MIN, FREQ_MAX);
    let o_raw = 0.15 + 0.70 * norm(phase, PHASE_MIN, PHASE_MAX);

    let mut cur = [b_raw, s_raw, o_raw];
    const TARGET_SUM: f64 = 1.50;
    const MIN_BOUND: f64 = 0.12;
    const MAX_BOUND: f64 = 0.88;

    // Bounded simplex water-filling 投影（12 次迭代）
    for _ in 0..12 {
        let sum: f64 = cur[0] + cur[1] + cur[2];
        let diff = TARGET_SUM - sum;
        if diff.abs() < 1e-12 {
            break;
        }
        let mut free_count = 0usize;
        for &v in &cur {
            if (diff > 0.0 && v < MAX_BOUND - 1e-9) || (diff < 0.0 && v > MIN_BOUND + 1e-9) {
                free_count += 1;
            }
        }
        if free_count == 0 {
            break;
        }
        let delta = diff / (free_count as f64);
        for v in &mut cur {
            if (diff > 0.0 && *v < MAX_BOUND - 1e-9) || (diff < 0.0 && *v > MIN_BOUND + 1e-9) {
                *v = (*v + delta).clamp(MIN_BOUND, MAX_BOUND);
            }
        }
    }
    let rem = TARGET_SUM - (cur[0] + cur[1] + cur[2]);
    if rem.abs() > 1e-12 {
        for v in &mut cur {
            if (rem > 0.0 && *v + rem <= MAX_BOUND) || (rem < 0.0 && *v + rem >= MIN_BOUND) {
                *v += rem;
                break;
            }
        }
    }

    Personality {
        boldness: cur[0],
        sensitivity: cur[1],
        orderliness: cur[2],
    }
}

/// 由 soul_seed 展開為戰鬥三係數（能階α、時脈β、相位γ）。
pub fn expand_soul_seed_to_combat_axes(seed: i64) -> (f64, f64, f64) {
    let (amp, freq, phase) = expand_seed_axes(seed);
    let alpha = 0.5 + (amp - AMP_MIN) / (AMP_MAX - AMP_MIN) * 1.5;
    let beta = 0.5 + (freq - FREQ_MIN) / (FREQ_MAX - FREQ_MIN) * 1.0;
    let gamma = (phase - PHASE_MIN) / (PHASE_MAX - PHASE_MIN);
    (alpha, beta, gamma)
}

/// 拓撲邊權總量常數（對齊既有 `TotalCostNorm`）。
pub const TOTAL_TOPOLOGY_COST_NORM: f64 = 10_000.0;
/// 拓撲邊數（對齊既有 `NumTopologyEdges`）。
pub const NUM_TOPOLOGY_EDGES: usize = 760;

/// 由 soul_seed 產生 760 條拓撲 Cost（對齊 `ExpandSoulSeedToTopologyCosts` 流程）。
#[must_use]
pub fn expand_soul_seed_to_topology_costs(seed: i64) -> Vec<f64> {
    let mut rng = SimpleLcg::new(seed);
    let _ = (rng.next_f64(), rng.next_f64(), rng.next_f64());
    let mut raw = vec![0f64; NUM_TOPOLOGY_EDGES];
    let mut sum = 0f64;
    for r in &mut raw {
        let u = rng.next_f64();
        *r = 0.1 + u * 0.9;
        sum += *r;
    }
    raw.iter()
        .map(|x| (x / sum) * TOTAL_TOPOLOGY_COST_NORM)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_soul_seed_to_personality_conservation() {
        // 驗證跨各種種子性格三軸總和嚴格守恆 1.50 且各軸處於 [0.12, 0.88]
        for seed in -1000..1000 {
            let p = expand_soul_seed_to_personality(seed);
            let sum = p.boldness + p.sensitivity + p.orderliness;
            assert!(
                (sum - 1.50).abs() < 1e-6,
                "Seed {seed} failed sum conservation: sum = {sum}"
            );
            assert!(
                p.boldness >= 0.12 && p.boldness <= 0.88,
                "Seed {seed} boldness out of bounds: {}",
                p.boldness
            );
            assert!(
                p.sensitivity >= 0.12 && p.sensitivity <= 0.88,
                "Seed {seed} sensitivity out of bounds: {}",
                p.sensitivity
            );
            assert!(
                p.orderliness >= 0.12 && p.orderliness <= 0.88,
                "Seed {seed} orderliness out of bounds: {}",
                p.orderliness
            );
        }
    }
}

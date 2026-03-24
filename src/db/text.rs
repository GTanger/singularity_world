// 字串相似度（LCS），對齊 Go db/archival.go `RuneLCSSimilarity`。

/// `2 * LCS / (len(a) + len(b))`，供摘要／去重（對齊 Go `RuneLCSSimilarity`）。
#[must_use]
pub fn rune_lcs_similarity(a: &str, b: &str) -> f64 {
    let ra: Vec<char> = a.chars().collect();
    let rb: Vec<char> = b.chars().collect();
    let la = ra.len();
    let lb = rb.len();
    if la == 0 || lb == 0 {
        return 0.0;
    }
    let mut prev = vec![0i32; lb + 1];
    let mut cur = vec![0i32; lb + 1];
    for i in 1..=la {
        cur[0] = 0;
        for j in 1..=lb {
            if ra[i - 1] == rb[j - 1] {
                cur[j] = prev[j - 1] + 1;
            } else {
                cur[j] = cur[j - 1].max(prev[j]);
            }
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    let lcs = prev[lb] as f64;
    (2.0 * lcs) / (la + lb) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_is_one() {
        assert!((rune_lcs_similarity("你好", "你好") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn empty_is_zero() {
        assert_eq!(rune_lcs_similarity("", "a"), 0.0);
    }
}

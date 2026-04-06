// 遊戲時間：1 真實小時 ＝ 1 遊戲日（依 scale），對齊既有 game/gametime。

const SECONDS_PER_GAME_DAY: i64 = 86400;

/// 依真實 Unix 秒與 epoch、scale 算出當前遊戲時間。
/// 回傳：遊戲內當日 0:00 起的秒數（0～86399）、時（0～23）、分（0～59）、自 epoch 起算的遊戲日數。
pub fn game_time_now(real_unix: i64, epoch: i64, scale: f64) -> (i32, i32, i32, i32) {
    let scale = if scale <= 0.0 { 24.0 } else { scale };
    let game_sec = (real_unix - epoch) as f64 * scale;
    let mut sec_total = game_sec as i64;
    if sec_total < 0 {
        sec_total = 0;
    }
    let days_since_epoch = (sec_total / SECONDS_PER_GAME_DAY) as i32;
    let sec_since_midnight = (sec_total % SECONDS_PER_GAME_DAY) as i32;
    let hour = sec_since_midnight / 3600;
    let min = (sec_since_midnight % 3600) / 60;
    (sec_since_midnight, hour, min, days_since_epoch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_at_epoch() {
        let (s, h, m, d) = game_time_now(0, 0, 24.0);
        assert_eq!((s, h, m, d), (0, 0, 0, 0));
    }

    #[test]
    fn one_game_day() {
        // (3600 real sec) * 24 scale = 86400 game sec = 1 game day
        let (s, h, m, d) = game_time_now(3600, 0, 24.0);
        assert_eq!(d, 1);
        assert_eq!((s, h, m), (0, 0, 0));
    }

    #[test]
    fn negative_clamped() {
        let (s, h, m, d) = game_time_now(0, 100, 24.0);
        assert_eq!((s, h, m, d), (0, 0, 0, 0));
    }

    #[test]
    fn default_scale_when_non_positive() {
        let (s, h, m, d) = game_time_now(150, 0, 0.0);
        // scale -> 24, game_sec = 3600 -> 0h1m
        assert_eq!(d, 0);
        assert_eq!(h, 1);
        assert_eq!(m, 0);
        assert_eq!(s, 3600);
    }
}

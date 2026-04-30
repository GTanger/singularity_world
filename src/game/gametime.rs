// 遊戲時間：1 真實小時 ＝ 1 遊戲日（依 scale），對齊既有 game/gametime。

use serde::{Deserialize, Serialize};

const SECONDS_PER_GAME_DAY: i64 = 86400;

// ─── 時辰段 ──────────────────────────────────────────────────────────

/// 24 小時切六段的時辰枚舉。
///
/// 邊界值：拂曉 4–7、晝前 8–10、正晝 11–14、黃昏 15–17、夜段 18–22、黎明前 23–3。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimePeriod {
    /// 拂曉  hour 4-7
    Dawn,
    /// 晝前  hour 8-10
    Forenoon,
    /// 正晝  hour 11-14
    Noon,
    /// 黃昏  hour 15-17
    Dusk,
    /// 夜段  hour 18-22
    Night,
    /// 黎明前 hour 23-3
    Predawn,
}

impl TimePeriod {
    /// 由小時（0-23）映射到時辰段。
    pub fn from_hour(hour: i32) -> Self {
        match hour {
            4..=7 => TimePeriod::Dawn,
            8..=10 => TimePeriod::Forenoon,
            11..=14 => TimePeriod::Noon,
            15..=17 => TimePeriod::Dusk,
            18..=22 => TimePeriod::Night,
            _ => TimePeriod::Predawn, // 23, 0, 1, 2, 3
        }
    }

    /// 轉成前端顯示用的中文標籤。
    pub fn label(self) -> &'static str {
        match self {
            TimePeriod::Dawn => "拂曉",
            TimePeriod::Forenoon => "晝前",
            TimePeriod::Noon => "正晝",
            TimePeriod::Dusk => "黃昏",
            TimePeriod::Night => "夜段",
            TimePeriod::Predawn => "黎明前",
        }
    }

    /// 回傳段位索引（0-5，用於時辰成本計算）。
    pub fn index(self) -> i32 {
        match self {
            TimePeriod::Predawn => 0,
            TimePeriod::Dawn => 1,
            TimePeriod::Forenoon => 2,
            TimePeriod::Noon => 3,
            TimePeriod::Dusk => 4,
            TimePeriod::Night => 5,
        }
    }
}

/// 由小時映射到時辰段（`TimePeriod::from_hour` 的獨立函式版，供舊呼叫方使用）。
pub fn time_period(hour: i32) -> TimePeriod {
    TimePeriod::from_hour(hour)
}

/// 時辰段名稱（中文），等同 `time_period(hour).label()`。
pub fn time_period_label(hour: i32) -> &'static str {
    TimePeriod::from_hour(hour).label()
}

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

    #[test]
    fn time_period_mapping() {
        assert_eq!(TimePeriod::from_hour(4), TimePeriod::Dawn);
        assert_eq!(TimePeriod::from_hour(7), TimePeriod::Dawn);
        assert_eq!(TimePeriod::from_hour(8), TimePeriod::Forenoon);
        assert_eq!(TimePeriod::from_hour(11), TimePeriod::Noon);
        assert_eq!(TimePeriod::from_hour(15), TimePeriod::Dusk);
        assert_eq!(TimePeriod::from_hour(18), TimePeriod::Night);
        assert_eq!(TimePeriod::from_hour(22), TimePeriod::Night);
        assert_eq!(TimePeriod::from_hour(23), TimePeriod::Predawn);
        assert_eq!(TimePeriod::from_hour(0), TimePeriod::Predawn);
        assert_eq!(TimePeriod::from_hour(3), TimePeriod::Predawn);
    }

    #[test]
    fn time_period_labels() {
        assert_eq!(TimePeriod::Dawn.label(), "拂曉");
        assert_eq!(TimePeriod::Night.label(), "夜段");
        assert_eq!(TimePeriod::Predawn.label(), "黎明前");
    }
}

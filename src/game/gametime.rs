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

/// 讀取全域快取的 `(game_time_epoch_unix, game_time_scale)`。
pub fn game_time_cfg() -> (i64, f64) {
    static TIME_CFG: std::sync::LazyLock<(i64, f64)> = std::sync::LazyLock::new(|| {
        let cfg = crate::config::Server::load_default();
        (cfg.game_time_epoch_unix, cfg.game_time_scale)
    });
    *TIME_CFG
}

/// 依據 `days_since_epoch` 計算開譜曆 `(year, season_name, day_in_season)`。
/// 四季正典：春融（91日）、夏漲（91日）、秋狩（91日）、冬凝（92日）。
pub fn kaipu_calendar_parts(days_since_epoch: i32) -> (i32, &'static str, i32) {
    let days = days_since_epoch.max(0);
    let year = (days / 365) + 1;
    let day_in_year = days % 365;
    let (season_name, day_in_season) = if day_in_year < 91 {
        ("春融", day_in_year + 1)
    } else if day_in_year < 182 {
        ("夏漲", day_in_year - 91 + 1)
    } else if day_in_year < 273 {
        ("秋狩", day_in_year - 182 + 1)
    } else {
        ("冬凝", day_in_year - 273 + 1)
    };
    (year, season_name, day_in_season)
}

/// 回傳開譜曆季節名稱（`春融` / `夏漲` / `秋狩` / `冬凝`）。
pub fn game_season_label(days_since_epoch: i32) -> &'static str {
    kaipu_calendar_parts(days_since_epoch).1
}

/// 格式化開譜曆日期標籤（對齊前端 `sw-clock.js`：如 `開譜曆 17 年・秋狩・27 日`）。
pub fn format_kaipu_date_label(days_since_epoch: i32) -> String {
    let (year, season_name, day_in_season) = kaipu_calendar_parts(days_since_epoch);
    let year_str = if year == 1 {
        "元".to_string()
    } else {
        year.to_string()
    };
    format!("開譜曆 {year_str} 年・{season_name}・{day_in_season} 日")
}

/// 依真實 Unix 秒計算 `(days_since_epoch, hour, min, season_label, period_label)`。
pub fn game_clock_at_unix(real_unix: i64) -> (i32, i32, i32, &'static str, &'static str) {
    let (epoch, scale) = game_time_cfg();
    let (_, h, m, days) = game_time_now(real_unix, epoch, scale);
    let season = game_season_label(days);
    let period = time_period_label(h);
    (days, h, m, season, period)
}

pub fn current_game_clock_full() -> (i32, i32, i32, &'static str, &'static str) {
    game_clock_at_unix(crate::game::now_unix())
}

pub fn current_game_hour_and_min() -> (i32, i32) {
    let (_, h, m, _, _) = current_game_clock_full();
    (h, m)
}

pub fn current_game_hour() -> i32 {
    current_game_hour_and_min().0
}

/// 格式化完整開譜曆日時刻錨點（如 `開譜曆 17 年・秋狩・27 日（正晝 11:36）`）。
pub fn format_game_calendar_stamp(real_unix: i64) -> String {
    let (day, h, m, _, period) = game_clock_at_unix(real_unix);
    format!(
        "{}（{period} {h:02}:{m:02}）",
        format_kaipu_date_label(day)
    )
}

pub fn format_elapsed_duration_zh(delta_real_sec: i64) -> String {
    let delta = delta_real_sec.max(0);
    if delta < 15 {
        "數秒".to_string()
    } else if delta < 60 {
        format!("約 {} 秒", delta)
    } else if delta < 1800 {
        format!("約 {} 分鐘", (delta / 60).max(1))
    } else if delta < 3600 {
        format!("約半個遊戲日（真實約 {} 分鐘）", (delta / 60).max(1))
    } else {
        let (_, scale) = game_time_cfg();
        let scale = if scale <= 0.0 { 24.0 } else { scale };
        let game_days = ((delta as f64 * scale) / 86400.0).round() as i64;
        if game_days <= 1 {
            "約 1 個遊戲日".to_string()
        } else {
            format!("約 {} 個遊戲日", game_days)
        }
    }
}

pub fn format_memory_temporal_anchor(event_unix: i64, now_unix: i64) -> String {
    if event_unix <= 0 || now_unix <= 0 {
        return "往昔".to_string();
    }
    let delta = (now_unix - event_unix).max(0);
    let (now_day, _, _, _, _) = game_clock_at_unix(now_unix);
    let (ev_day, ev_h, ev_m, _, ev_period) = game_clock_at_unix(event_unix);
    let (_, ev_season, ev_day_in_season) = kaipu_calendar_parts(ev_day);
    let day_diff = (now_day - ev_day).max(0);
    if delta < 60 {
        format!(
            "本場對話・約 {} 秒前（今日 {ev_period} {ev_h:02}:{ev_m:02}）",
            delta.max(1)
        )
    } else if delta < 1800 {
        format!(
            "本場交談・約 {} 分鐘前（今日 {ev_period} {ev_h:02}:{ev_m:02}）",
            (delta / 60).max(1)
        )
    } else if day_diff == 0 {
        format!("今日稍早・{ev_period} {ev_h:02}:{ev_m:02}")
    } else if day_diff == 1 {
        format!(
            "昨日（1個遊戲日前・{ev_season}{ev_day_in_season}日 {ev_period} {ev_h:02}:{ev_m:02}）"
        )
    } else {
        format!(
            "{day_diff}個遊戲日前（{ev_season}{ev_day_in_season}日 {ev_period} {ev_h:02}:{ev_m:02}）"
        )
    }
}

pub fn format_short_relative_time(event_unix: i64, now_unix: i64) -> String {
    if event_unix <= 0 || now_unix <= 0 {
        return String::new();
    }
    let delta = (now_unix - event_unix).max(0);
    let (now_day, _, _, _, _) = game_clock_at_unix(now_unix);
    let (ev_day, ev_h, ev_m, _, ev_period) = game_clock_at_unix(event_unix);
    let (_, ev_season, ev_day_in_season) = kaipu_calendar_parts(ev_day);
    let day_diff = (now_day - ev_day).max(0);
    if delta < 90 {
        format!("[片刻前 {ev_h:02}:{ev_m:02}]")
    } else if delta < 1800 {
        format!("[約{}分鐘前 {ev_h:02}:{ev_m:02}]", (delta / 60).max(1))
    } else if day_diff == 0 {
        format!("[今日稍早 {ev_period}{ev_h:02}:{ev_m:02}]")
    } else if day_diff == 1 {
        format!("[昨日 {ev_period}{ev_h:02}:{ev_m:02}]")
    } else {
        format!("[{day_diff}日前 {ev_season}{ev_day_in_season}日]")
    }
}

pub fn format_schedule_window_sense(
    start_hour: u8,
    end_hour: u8,
    now_hour: i32,
    now_min: i32,
) -> String {
    let sh = (start_hour as i32).clamp(0, 23);
    let eh = (end_hour as i32).clamp(0, 24);
    let nh = now_hour.clamp(0, 23);
    let nm = now_min.clamp(0, 59);
    let now_total_m = nh * 60 + nm;
    let start_total_m = sh * 60;
    let end_total_m = if eh <= sh { (eh + 24) * 60 } else { eh * 60 };
    let adj_now_m = if now_total_m < start_total_m && eh <= sh {
        now_total_m + 24 * 60
    } else {
        now_total_m
    };
    let elapsed_m = (adj_now_m - start_total_m).max(0);
    let remain_m = (end_total_m - adj_now_m).max(0);
    let elapsed_desc = if elapsed_m < 30 {
        "初開工不久".to_string()
    } else if elapsed_m < 60 {
        format!("已持續約 {} 分鐘", elapsed_m)
    } else {
        format!("已持續約 {} 小時", (elapsed_m + 30) / 60)
    };
    let remain_desc = if remain_m <= 30 {
        format!(
            "再過約 {} 分鐘即達 {:02}:00 換班／收工時刻",
            remain_m.max(1),
            eh % 24
        )
    } else {
        format!(
            "距 {:02}:00 換班／收工尚有約 {} 小時",
            eh % 24,
            (remain_m + 30) / 60
        )
    };
    format!(
        "本段作息 {:02}:00–{:02}:00（{}，{}）",
        sh,
        eh % 24,
        elapsed_desc,
        remain_desc
    )
}

pub fn format_game_time_prompt_context(hour: i32, min: i32) -> String {
    let h = hour.clamp(0, 23);
    let m = min.clamp(0, 59);
    match h {
        23 | 0 => format!(
            "{h:02}:{m:02}（黎明前・子夜深宵，四周一片漆黑、天尚未亮，距離破曉天亮還很久）"
        ),
        1 | 2 => format!(
            "{h:02}:{m:02}（黎明前・丑夜深沉，四周漆黑、天還沒亮，絕不可說『天還沒黑』或『天亮得透亮』）"
        ),
        3 => format!(
            "{h:02}:{m:02}（黎明前・破曉前夕，天還沒亮透、夜色尚暗，不可說『天還沒黑』或『趁天黑前』）"
        ),
        4..=7 => format!(
            "{h:02}:{m:02}（拂曉清晨・天色初亮，晨光微明、早市與早工初起）"
        ),
        8..=10 => format!(
            "{h:02}:{m:02}（晝前上午・白日天亮，晨光充足、市集熱絡）"
        ),
        11..=14 => format!(
            "{h:02}:{m:02}（正晝正午・大白天日頭當空，天色大亮）"
        ),
        15..=17 => format!(
            "{h:02}:{m:02}（黃昏傍晚・斜陽暮色，天還沒全黑但已近傍晚收工）"
        ),
        18..=20 => format!(
            "{h:02}:{m:02}（夜段入夜・天色已黑，街巷點起燈火）"
        ),
        _ => format!(
            "{h:02}:{m:02}（夜段深宵・夜色已深、天色漆黑，多數鋪子已歇業準備安歇）"
        ),
    }
}

pub fn current_game_time_prompt_context() -> String {
    let (day, h, m, _, _) = current_game_clock_full();
    format!(
        "{} ｜ {}",
        format_kaipu_date_label(day),
        format_game_time_prompt_context(h, m)
    )
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

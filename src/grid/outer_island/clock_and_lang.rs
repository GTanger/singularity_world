//! 太陽曆、一日十二具名鐘響與 Rust 源頭三段式異族語言裁切遮罩。

use crate::game::{self, now_unix};

#[derive(Debug, Clone)]
pub struct SolarClockSnapshot {
    pub real_unix: i64,
    pub game_days: i64,
    pub hour: u32,
    pub bell_index: u8,
    pub calendar_label: String,
    pub bell_label: String,
    pub six_phase_key: String,
    pub is_night: bool,
}

/// 依伺服器權威即時時鐘（`1 真實小時 = 1 遊戲日`）換算太陽曆與一日十二鐘響。
pub fn current_solar_clock() -> SolarClockSnapshot {
    let real_unix = now_unix();
    solar_clock_at(real_unix)
}

pub fn solar_clock_at(real_unix: i64) -> SolarClockSnapshot {
    let (epoch, scale) = game::game_time_cfg();
    let (_sec, raw_hour, _min, raw_days) = game::game_time_now(real_unix, epoch, scale);
    let days = i64::from(raw_days);
    let hour = raw_hour.clamp(0, 23) as u32;
    let day_of_year = days.rem_euclid(364);
    let (season, day_in_season) = if day_of_year < 91 {
        ("初春", day_of_year + 1)
    } else if day_of_year < 182 {
        ("仲夏", day_of_year - 90)
    } else if day_of_year < 273 {
        ("金秋", day_of_year - 181)
    } else {
        ("寒冬", day_of_year - 272)
    };
    let year = (days.div_euclid(364)).max(0) + 312;
    let calendar_label = format!("太陽曆 {year} 年・{season}・{} 日", day_in_season);

    let bell_idx = (hour / 2) % 12;
    let bell_label = match bell_idx {
        0 => "子夜沉鐘",
        1 => "殘夜二鐘",
        2 => "破曉初鐘",
        3 => "晨鐘二響",
        4 => "晨鐘三響",
        5 => "日高四響",
        6 => "正午鐘響",
        7 => "午後初鐘",
        8 => "申時二鐘",
        9 => "黃昏鐘響",
        10 => "暮色二鐘",
        _ => "入夜初鐘",
    }
    .to_string();

    let six_phase_key = format!("d{days}:p{}", hour / 4);
    let is_night = !(6..19).contains(&hour);

    SolarClockSnapshot {
        real_unix,
        game_days: days,
        hour,
        bell_index: bell_idx as u8,
        calendar_label,
        bell_label,
        six_phase_key,
        is_night,
    }
}

pub fn language_name_zh(lang_code: &str) -> &'static str {
    match lang_code {
        "harbor_trade" => "海港商語",
        "dwarven" => "矮人山嶽語",
        "elvish" => "精靈古林語",
        "orcish" => "獸人荒原語",
        "goblin" => "哥布林部族語",
        _ => "遠方異語",
    }
}

const EXTRACTABLE_KEYWORDS: &[&str] = &[
    "北林",
    "試煉坑",
    "斷橋",
    "符文",
    "原石",
    "銅幣",
    "銀幣",
    "糧食",
    "危險",
    "鐵錘",
    "鍛錘",
    "巨蛛",
    "暗門",
    "渡輪",
    "船票",
    "欠債",
    "木箱",
    "草藥",
    "礦坑",
    "水手",
];

/// Rust 源頭語言遮罩：在送交客戶端或 LLM 前直接裁切原文，徹底防堵提示詞注入代譯穿幫。
pub fn mask_speech_by_proficiency(
    speaker_name: &str,
    lang_code: &str,
    proficiency: u8,
    raw_utterance: &str,
) -> (String, bool) {
    let lang_zh = language_name_zh(lang_code);
    if proficiency >= 80 {
        return (format!("〔{lang_zh}〕「{raw_utterance}」"), true);
    }
    if proficiency >= 25 {
        let mut hits: Vec<&str> = Vec::new();
        for &kw in EXTRACTABLE_KEYWORDS {
            if raw_utterance.contains(kw) && !hits.contains(&kw) {
                hits.push(kw);
                if hits.len() >= 3 {
                    break;
                }
            }
        }
        if !hits.is_empty() {
            let joined = hits
                .iter()
                .map(|w| format!("「{w}」"))
                .collect::<Vec<_>>()
                .join("、");
            return (
                format!(
                    "〔{lang_zh}・略懂〕{speaker_name}低聲說了一串話，你只勉強辨出 {joined} 幾個詞。"
                ),
                false,
            );
        }
        return (
            format!(
                "〔{lang_zh}・略懂〕{speaker_name}急促說了幾句，語速太快，你沒能抓準關鍵字眼。"
            ),
            false,
        );
    }

    let texture = match lang_code {
        "dwarven" => "短促如石塊互擊的低沉音節",
        "elvish" => "輕柔綿長如風吹樹梢的音節",
        "orcish" => "粗重渾厚的喉音",
        "goblin" => "急促尖細的短促叫嚷",
        _ => "腔調陌生的異鄉口音",
    };
    (
        format!("{speaker_name}用{texture}說了幾句話，比了個手勢，你完全聽不懂其中的意思。"),
        false,
    )
}

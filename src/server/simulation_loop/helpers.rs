//! 主迴圈共用小工具：計時觸發、格式化、tick 節奏初值。

use std::time::{Duration, Instant};

use rand::Rng;

use crate::config::{sim, Server};

/// 間隔到期則更新錨點並回傳 true；`last` 為 `None` 時第一次立即觸發（對齊既有 `time.Time` 零值）。
pub(super) fn periodic_fire(last: &mut Option<Instant>, period: Duration) -> bool {
    let now = Instant::now();
    let fire = match *last {
        None => true,
        Some(t) => now.duration_since(t) >= period,
    };
    if fire {
        *last = Some(now);
    }
    fire
}

pub(super) fn sprintf_s(template: &str, args: &[&str]) -> String {
    let mut s = template.to_string();
    for a in args {
        if let Some(pos) = s.find("%s") {
            s.replace_range(pos..pos + 2, a);
        }
    }
    s
}

pub(super) fn sprintf_d(template: &str, n: i32) -> String {
    template.replacen("%d", &n.to_string(), 1)
}

/// `simulation.json` 的 `travel_tick_interval`；無效時 30。
pub(super) fn effective_travel_tick_interval() -> i32 {
    let v = sim().travel_tick_interval;
    if v <= 0 {
        30
    } else {
        v
    }
}

/// 首次閒置觸發門檻（對齊既有 `nextIdleTrigger` 初值）。
pub(super) fn initial_next_idle_trigger() -> i32 {
    let idle = &sim().idle;
    let mut span = idle.first_trigger_span;
    if span <= 0 {
        span = 1;
    }
    let mut rng = rand::rng();
    idle.first_trigger_min + rng.random_range(0..span)
}

/// 之後每次閒置觸發後重骰的間隔 tick。
pub(super) fn roll_next_idle_interval() -> i32 {
    let idle = &sim().idle;
    let mut span = idle.interval_span;
    if span <= 0 {
        span = 1;
    }
    let mut rng = rand::rng();
    idle.interval_min + rng.random_range(0..span)
}

pub(super) fn initial_random_dialogue_ticks() -> i32 {
    let rdt = &sim().random_npc_dialogue_ticks;
    let mut span = rdt.initial_span;
    if span <= 0 {
        span = 1;
    }
    let mut rng = rand::rng();
    rdt.initial_min + rng.random_range(0..span)
}

/// 無玩家在線或無法觸發 AI 時，用 `server_defaults` 的間隔重設倒數。
pub(super) fn reset_ticks_no_player(cfg: &Server) -> i32 {
    let mut min_e = cfg.npc_npc_social_tick_min_no_player;
    if min_e <= 0 {
        min_e = 80;
    }
    let mut extra = cfg.npc_npc_social_tick_extra_no_player;
    if extra <= 0 {
        extra = 40;
    }
    let span = extra.max(1);
    let mut rng = rand::rng();
    min_e + rng.random_range(0..span)
}

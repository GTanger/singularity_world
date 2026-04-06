//! NPC 行為文案快取（對齊既有 `npc/behavior`）。

use std::collections::HashMap;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

use rand::Rng;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone)]
struct RoleMovementJson {
    #[serde(default)]
    speed: i32,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RoleBehaviorJson {
    #[serde(default)]
    enter_reactions: Vec<String>,
    #[serde(default)]
    shift_arrive: String,
    #[serde(default)]
    shift_leave: String,
    #[serde(default)]
    movement: Option<RoleMovementJson>,
    #[serde(default)]
    idle: HashMap<String, Vec<String>>,
    #[serde(default)]
    wander_rooms: Vec<String>,
    #[serde(default)]
    wander_leave: String,
    #[serde(default)]
    wander_arrive: String,
}

#[derive(Debug, Deserialize, Default)]
struct BehaviorsFile {
    #[serde(default)]
    time_periods: HashMap<String, [i32; 2]>,
    #[serde(default)]
    roles: HashMap<String, RoleBehaviorJson>,
}

static BEHAVIORS: OnceLock<RwLock<Option<BehaviorsFile>>> = OnceLock::new();

fn cache() -> &'static RwLock<Option<BehaviorsFile>> {
    BEHAVIORS.get_or_init(|| RwLock::new(None))
}

/// 載入 `npc_behaviors.json`；可重複呼叫覆寫快取。
pub fn try_load_npc_behaviors(path: &Path) -> anyhow::Result<()> {
    let raw = std::fs::read_to_string(path)
        .or_else(|_| std::fs::read_to_string(Path::new("..").join(path)))?;
    let f: BehaviorsFile = serde_json::from_str(&raw)?;
    let n = f.roles.len();
    *cache().write().expect("behaviors poisoned") = Some(f);
    tracing::info!(target: "npc_behavior", "loaded {n} roles from {}", path.display());
    Ok(())
}

/// 換班敘事（`arriving == true` 為上班）；無職稱或檔未載入時回空字串。
#[must_use]
pub fn get_shift_flavor(title: &str, npc_name: &str, arriving: bool) -> String {
    let g = cache().read().expect("behaviors poisoned");
    let Some(ref bd) = *g else {
        return String::new();
    };
    let Some(role) = bd.roles.get(title) else {
        return String::new();
    };
    let s = if arriving {
        &role.shift_arrive
    } else {
        &role.shift_leave
    };
    s.replace("{name}", npc_name)
}

/// 依職稱讀取 `movement.speed`（對齊既有 `GetMovementDefForTitle` 之 speed）；無則為 1。
#[must_use]
pub fn movement_speed_for_title(title: &str) -> i32 {
    let g = cache().read().expect("behaviors poisoned");
    let Some(ref bd) = *g else {
        return 1;
    };
    let Some(role) = bd.roles.get(title) else {
        return 1;
    };
    role.movement
        .as_ref()
        .map(|m| m.speed)
        .filter(|&s| s > 0)
        .unwrap_or(1)
}

/// 遊戲小時（0–23）映射為 `morning`／`noon` 等（對齊既有 `GetTimePeriod`）。
#[must_use]
pub fn get_time_period(game_hour: i32) -> String {
    let g = cache().read().expect("behaviors poisoned");
    let Some(ref bd) = *g else {
        return "night".to_string();
    };
    for (name, bounds) in &bd.time_periods {
        let start = bounds[0];
        let end = bounds[1];
        if start <= end {
            if game_hour >= start && game_hour < end {
                return name.clone();
            }
        } else if game_hour >= start || game_hour < end {
            return name.clone();
        }
    }
    "night".to_string()
}

fn pick_random_line(lines: &[String]) -> Option<&str> {
    if lines.is_empty() {
        return None;
    }
    let mut rng = rand::rng();
    Some(lines[rng.random_range(0..lines.len())].as_str())
}

/// NPC 進房反應（對齊既有 `PickEnterReaction`）。
#[must_use]
pub fn pick_enter_reaction(title: &str, npc_name: &str) -> String {
    let g = cache().read().expect("behaviors poisoned");
    let Some(ref bd) = *g else {
        return String::new();
    };
    let Some(role) = bd.roles.get(title) else {
        return String::new();
    };
    let Some(line) = pick_random_line(&role.enter_reactions) else {
        return String::new();
    };
    line.replace("{name}", npc_name)
}

/// 閒置動作一句；`disposition` 極端時改讀 morning／night 池（對齊既有 `PickIdleEmote`）。
#[must_use]
pub fn pick_idle_emote(title: &str, period: &str, npc_name: &str, disposition: i32) -> String {
    let g = cache().read().expect("behaviors poisoned");
    let Some(ref bd) = *g else {
        return String::new();
    };
    let Some(role) = bd.roles.get(title) else {
        return String::new();
    };
    let mut effective = period.to_string();
    if disposition < -30 {
        effective = "night".to_string();
    } else if disposition > 30 {
        effective = "morning".to_string();
    }
    let mut pick_from: &[String] = role
        .idle
        .get(effective.as_str())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if pick_from.is_empty() {
        pick_from = role.idle.get(period).map(Vec::as_slice).unwrap_or(&[]);
    }
    let Some(line) = pick_random_line(pick_from) else {
        return String::new();
    };
    line.replace("{name}", npc_name)
}

/// 職稱對應的巡邏房間 id 列表（對齊既有 `GetWanderRooms`）。
#[must_use]
pub fn get_wander_rooms(title: &str) -> Vec<String> {
    let g = cache().read().expect("behaviors poisoned");
    let Some(ref bd) = *g else {
        return Vec::new();
    };
    bd.roles
        .get(title)
        .map(|r| r.wander_rooms.clone())
        .unwrap_or_default()
}

/// 巡邏離開／抵達敘事（對齊既有 `GetWanderFlavor`）。
#[must_use]
pub fn get_wander_flavor(title: &str, npc_name: &str, room_name: &str, leaving: bool) -> String {
    let g = cache().read().expect("behaviors poisoned");
    let Some(ref bd) = *g else {
        return String::new();
    };
    let Some(role) = bd.roles.get(title) else {
        return String::new();
    };
    let s = if leaving {
        &role.wander_leave
    } else {
        &role.wander_arrive
    };
    let mut out = s.replace("{name}", npc_name);
    if leaving {
        out = out.replace("{dest}", room_name);
    } else {
        out = out.replace("{from}", room_name);
    }
    out
}

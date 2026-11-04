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

/// 依場域一等物件合成 NPC 當值行為敘事句。
#[must_use]
pub fn synthesize_venue_role_behavior_line(title: &str, npc_name: &str, mode: &str) -> String {
    let t = title.trim();
    let name = npc_name.trim();
    if t.is_empty() || name.is_empty() {
        return String::new();
    }
    let Some(venue) = crate::grid::resolve_any_room_venue_object("", t) else {
        return String::new();
    };
    let zone_tag = if venue.has_counter_restricted_zone {
        "當值櫃檯內側"
    } else {
        "當值作業區"
    };
    match mode {
        "shift_arrive" => format!("{}整妥工作服步入{}{}，{}。", name, venue.venue_kind, zone_tag, venue.on_duty_fixture_activity),
        "shift_leave" => format!("{}收妥{}{}案頭器物，交接班次後邁步離去。", name, venue.venue_kind, zone_tag),
        "enter_reaction" => format!("{}自{}{}抬眼望來，{}。", name, venue.venue_kind, zone_tag, venue.on_duty_demeanor),
        "idle" => format!("{}立於{}{}，{}。", name, venue.venue_kind, zone_tag, venue.on_duty_fixture_activity),
        _ => String::new(),
    }
}

/// 換班敘事（`arriving == true` 為上工當值）；無職稱或檔未載入時回空字串。
#[must_use]
pub fn get_shift_flavor(title: &str, npc_name: &str, arriving: bool) -> String {
    let g = cache().read().expect("behaviors poisoned");
    if let Some(bd) = &*g {
        if let Some(role) = bd.roles.get(title) {
            let s = if arriving {
                &role.shift_arrive
            } else {
                &role.shift_leave
            };
            let replaced = s.replace("{name}", npc_name);
            if !replaced.trim().is_empty() {
                return replaced;
            }
        }
    }
    synthesize_venue_role_behavior_line(
        title,
        npc_name,
        if arriving { "shift_arrive" } else { "shift_leave" },
    )
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
    if let Some(bd) = &*g {
        if let Some(role) = bd.roles.get(title) {
            if let Some(line) = pick_random_line(&role.enter_reactions) {
                let replaced = line.replace("{name}", npc_name);
                if !replaced.trim().is_empty() {
                    return replaced;
                }
            }
        }
    }
    synthesize_venue_role_behavior_line(title, npc_name, "enter_reaction")
}

/// 閒置動作一句；`disposition` 極端時改讀 morning／night 池（對齊既有 `PickIdleEmote`）。
#[must_use]
pub fn pick_idle_emote(title: &str, period: &str, npc_name: &str, disposition: i32) -> String {
    let g = cache().read().expect("behaviors poisoned");
    if let Some(bd) = &*g {
        if let Some(role) = bd.roles.get(title) {
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
            if let Some(line) = pick_random_line(pick_from) {
                let replaced = line.replace("{name}", npc_name);
                if !replaced.trim().is_empty() {
                    return replaced;
                }
            }
        }
    }
    synthesize_venue_role_behavior_line(title, npc_name, "idle")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesize_venue_role_behavior_line_and_fallbacks() {
        // 1. synthesize_venue_role_behavior_line 直接測試
        let arrive_mill = synthesize_venue_role_behavior_line("當值石磨匠師", "石老爹", "shift_arrive");
        assert!(arrive_mill.contains("石老爹"));
        assert!(arrive_mill.contains("水輪石磨工坊"));
        assert!(arrive_mill.contains("當值櫃檯內側"));

        let leave_chef = synthesize_venue_role_behavior_line("食堂主廚", "張大廚", "shift_leave");
        assert!(leave_chef.contains("張大廚"));
        assert!(leave_chef.contains("大食堂"));
        assert!(leave_chef.contains("當值櫃檯內側"));

        let enter_mill = synthesize_venue_role_behavior_line("當值石磨匠師", "石老爹", "enter_reaction");
        assert!(enter_mill.contains("石老爹"));
        assert!(enter_mill.contains("水輪石磨工坊"));
        assert!(enter_mill.contains("抬眼望來"));

        let idle_chef = synthesize_venue_role_behavior_line("食堂主廚", "張大廚", "idle");
        assert!(idle_chef.contains("張大廚"));
        assert!(idle_chef.contains("大食堂"));
        assert!(idle_chef.contains("立於"));

        // 空字串或未知職稱
        assert!(synthesize_venue_role_behavior_line("", "張大廚", "idle").is_empty());
        assert!(synthesize_venue_role_behavior_line("食堂主廚", "", "idle").is_empty());
        assert!(synthesize_venue_role_behavior_line("未知神祕漫遊者", "小旅人", "idle").is_empty());

        // 2. 回退測試：快取未載入此類特定場域職稱或空檔時回退至合成句
        let shift_flavor = get_shift_flavor("當值石磨匠師", "石老爹", true);
        assert!(!shift_flavor.is_empty());
        assert!(shift_flavor.contains("石老爹"));

        let shift_leave_chef = get_shift_flavor("食堂主廚", "張大廚", false);
        assert!(!shift_leave_chef.is_empty());
        assert!(shift_leave_chef.contains("張大廚"));

        let reaction = pick_enter_reaction("當值石磨匠師", "石老爹");
        assert!(!reaction.is_empty());
        assert!(reaction.contains("石老爹"));

        let idle_emote = pick_idle_emote("食堂主廚", "noon", "張大廚", 0);
        assert!(!idle_emote.is_empty());
        assert!(idle_emote.contains("張大廚"));
    }
}

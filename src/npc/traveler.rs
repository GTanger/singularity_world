//! 地圖級 NPC 移動管理（對齊 Go `npc/movement.go` 之 `TravelerManager`／`NPCTraveler`）。
//! 目前完整實作 **排班型**；腦驅動型僅註冊，`compute_next_path` 回空直至決策引擎遷移。

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::db::{
    get_all_schedules, get_entity, get_entity_room, get_npc_display_label_at_hour, get_npc_ids_with_room,
    get_npc_title_from_assignments, get_schedule_target_room, set_entity_room, RoomGraph,
};
use crate::gametext;

use super::behavior::movement_speed_for_title;

/// 移動模式（對齊 Go `MovementType`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementType {
    Schedule,
    Brain,
}

/// 單一 NPC 的移動參數（對齊 Go `MovementDef` 子集）。
#[derive(Debug, Clone)]
pub struct MovementDef {
    pub kind: MovementType,
    pub speed: i32,
}

struct NpcTraveler {
    entity_id: String,
    move_def: MovementDef,
    path_queue: Vec<String>,
    active: bool,
    stay_until_hour: i32,
}

/// 管理所有正在進行地圖級移動的 NPC（對齊 Go `TravelerManager`）。
#[derive(Default)]
pub struct TravelerManager {
    travelers: HashMap<String, NpcTraveler>,
}

/// 一次「觀測」下完成的步驟（對齊 Go `NPCStep`；腦驅動抵達欄位留待擴充）。
#[derive(Debug, Clone)]
pub struct NpcStep {
    pub entity_id: String,
    pub old_room: String,
    pub new_room: String,
    pub npc_name: String,
}

impl TravelerManager {
    /// 建立空管理器。
    #[must_use]
    pub fn new() -> Self {
        Self {
            travelers: HashMap::new(),
        }
    }

    /// 註冊或覆寫 NPC 移動設定（對齊 `Register`）。
    pub fn register(&mut self, entity_id: String, mut def: MovementDef) {
        if def.speed <= 0 {
            def.speed = 1;
        }
        self.travelers.insert(
            entity_id.clone(),
            NpcTraveler {
                entity_id,
                move_def: def,
                path_queue: Vec::new(),
                active: true,
                stay_until_hour: -1,
            },
        );
    }

    /// 推進一步；`active` 為 `None` 時驅動全部（相容舊行為）；`Some` 且空則不驅動任何人。
    pub fn tick(&mut self, g: &RoomGraph, game_hour: i32, active: Option<&HashSet<String>>) -> Vec<NpcStep> {
        let mut steps: Vec<NpcStep> = Vec::new();
        let mut ids: Vec<String> = self.travelers.keys().cloned().collect();
        ids.sort();
        for eid in ids {
            let Some(t) = self.travelers.get_mut(&eid) else {
                continue;
            };
            if !t.active {
                continue;
            }
            let Ok(current_room) = get_entity_room(&t.entity_id) else {
                continue;
            };
            if let Some(set) = active {
                if set.is_empty() {
                    continue;
                }
                if !set.contains(&current_room) {
                    continue;
                }
            }
            if t.stay_until_hour >= 0 {
                if game_hour != t.stay_until_hour {
                    continue;
                }
                t.stay_until_hour = -1;
            }
            if t.path_queue.is_empty() {
                if let Some(p) = compute_next_path(t, &current_room, g, game_hour) {
                    t.path_queue = p;
                }
                if t.path_queue.is_empty() {
                    continue;
                }
            }
            let mut steps_to_take = t.move_def.speed.max(1) as usize;
            if steps_to_take > t.path_queue.len() {
                steps_to_take = t.path_queue.len();
            }
            let old_room = current_room.clone();
            let mut cur = current_room;
            for _ in 0..steps_to_take {
                let next = t.path_queue.remove(0);
                if set_entity_room(&t.entity_id, &next).is_ok() {
                    cur = next;
                }
            }
            if cur != old_room {
                let mut npc_name = get_npc_display_label_at_hour(&t.entity_id, game_hour).unwrap_or_default();
                if npc_name.is_empty() {
                    npc_name = t.entity_id.clone();
                }
                steps.push(NpcStep {
                    entity_id: t.entity_id.clone(),
                    old_room,
                    new_room: cur,
                    npc_name,
                });
            }
        }
        steps
    }
}

/// 依排班與遊戲小時決定下一條路徑；腦驅動尚未實作時回 `None`。
fn compute_next_path(
    t: &NpcTraveler,
    current_room: &str,
    g: &RoomGraph,
    game_hour: i32,
) -> Option<Vec<String>> {
    match t.move_def.kind {
        MovementType::Schedule => {
            let tr = get_schedule_target_room(&t.entity_id, game_hour)?;
            if tr.is_empty() || tr == current_room {
                return None;
            }
            g.find_path(current_room, &tr)
        }
        MovementType::Brain => None,
    }
}

/// 啟動時依 `schedules` 與 NPC 清單註冊 Traveler（對齊 Go `startSimulationMainLoop` 開頭）。
pub fn seed_traveler_manager(mgr: &Mutex<TravelerManager>) -> anyhow::Result<()> {
    let mut tm = TravelerManager::new();
    let schedules = get_all_schedules()?;
    let mut scheduled: HashSet<String> = HashSet::new();
    for s in schedules {
        let Ok(Some(ch)) = get_entity(&s.entity_id) else {
            continue;
        };
        if ch.vit <= 0 {
            continue;
        }
        scheduled.insert(s.entity_id.clone());
        let mut title = get_npc_title_from_assignments(&s.entity_id);
        if title.is_empty() {
            title = gametext::default_manager_title();
        }
        let speed = movement_speed_for_title(&title);
        tm.register(
            s.entity_id,
            MovementDef {
                kind: MovementType::Schedule,
                speed,
            },
        );
    }
    for id in get_npc_ids_with_room() {
        if scheduled.contains(&id) {
            continue;
        }
        tm.register(
            id,
            MovementDef {
                kind: MovementType::Brain,
                speed: 1,
            },
        );
    }
    *mgr.lock().expect("traveler mgr poisoned") = tm;
    Ok(())
}

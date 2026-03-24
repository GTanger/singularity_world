//! 地圖級 NPC 移動管理（對齊 Go `npc/movement.go` 之 `TravelerManager`／`NPCTraveler`）。

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use rand::Rng;

use crate::db::{
    get_all_schedules, get_entity, get_entity_room, get_npc_display_label_at_hour, get_npc_ids_with_room,
    get_npc_title_from_assignments, get_schedule_target_room, set_entity_room, RoomGraph,
};
use crate::gametext;

use super::behavior::movement_speed_for_title;
use super::decision::{
    build_decision_context, build_decision_state, decide, resolve_brain_path, Intent, IntentType,
};

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
    /// 腦驅動：產生當前路徑的意圖，抵達後供到達效果。
    last_intent: Option<Intent>,
    /// 腦驅動：出發敘事，首次移動時發往舊房並清空。
    departure_narrative: String,
}

/// 管理所有正在進行地圖級移動的 NPC（對齊 Go `TravelerManager`）。
#[derive(Default)]
pub struct TravelerManager {
    travelers: HashMap<String, NpcTraveler>,
}

/// 一次「觀測」下完成的步驟（對齊 Go `NPCStep`）。
#[derive(Debug, Clone)]
pub struct NpcStep {
    pub entity_id: String,
    pub old_room: String,
    pub new_room: String,
    pub npc_name: String,
    /// 腦驅動抵達意圖目標時非空。
    pub arrival_intent: Option<IntentType>,
    /// 腦驅動決策時的出發敘事，發往 `old_room`。
    pub decision_narrative: String,
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
                last_intent: None,
                departure_narrative: String::new(),
            },
        );
    }

    /// 推進一步；`active` 為 `None` 時驅動全部；`Some` 且空則不驅動任何人。
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
                let mut step = NpcStep {
                    entity_id: t.entity_id.clone(),
                    old_room,
                    new_room: cur,
                    npc_name,
                    arrival_intent: None,
                    decision_narrative: String::new(),
                };
                if t.path_queue.is_empty()
                    && t.move_def.kind == MovementType::Brain
                    && let Some(li) = t.last_intent.take()
                {
                    let ty = li.ty;
                    let stay = compute_stay_brain(ty);
                    if stay > 0 {
                        t.stay_until_hour = (game_hour + stay).rem_euclid(24);
                    }
                    step.arrival_intent = Some(ty);
                }
                if !t.departure_narrative.is_empty() {
                    step.decision_narrative = std::mem::take(&mut t.departure_narrative);
                }
                steps.push(step);
            }
            if t.path_queue.is_empty() && t.move_def.kind != MovementType::Brain {
                let stay = compute_stay_schedule(t, game_hour);
                if stay > 0 {
                    t.stay_until_hour = (game_hour + stay).rem_euclid(24);
                }
            }
        }
        steps
    }
}

fn compute_stay_brain(ty: IntentType) -> i32 {
    let (min_h, max_h) = match ty {
        IntentType::Beg => (2, 4),
        IntentType::Gather => (1, 3),
        IntentType::SeekJob => (1, 2),
        IntentType::Trade => (2, 5),
        IntentType::Socialize => (1, 3),
        IntentType::Wander => (1, 4),
        IntentType::Work => (3, 6),
        IntentType::Idle => (1, 2),
    };
    if max_h <= 0 {
        return 0;
    }
    let mut min = min_h;
    let max = max_h;
    if min > max {
        min = max;
    }
    if min == max {
        return min;
    }
    let mut rng = rand::rng();
    min + rng.random_range(0..=(max - min))
}

fn compute_stay_schedule(_t: &NpcTraveler, _game_hour: i32) -> i32 {
    0
}

fn build_departure_narrative(npc_name: &str, intent_type: IntentType) -> String {
    match intent_type {
        IntentType::SeekJob => format!("【{npc_name}】望著荷包，拍拍衣袖，打算出門碰碰運氣。"),
        IntentType::Beg => format!("【{npc_name}】長嘆一聲，朝熱鬧處走去。"),
        IntentType::Gather => format!("【{npc_name}】拿起背簍，悄悄往郊外去了。"),
        IntentType::Trade => format!("【{npc_name}】把包袱繫緊，往人多的地方找買家去了。"),
        IntentType::Socialize => format!("【{npc_name}】閒不住，往人多的地方晃去。"),
        _ => String::new(),
    }
}

/// 依排班或腦決策決定下一條路徑。
fn compute_next_path(
    t: &mut NpcTraveler,
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
        MovementType::Brain => {
            let state = build_decision_state(&t.entity_id).ok()?;
            let ctx = build_decision_context(g, current_room, 20);
            let intent = decide(state, &ctx);
            t.last_intent = Some(intent.clone());
            let mut rng = rand::rng();
            if rng.random_range(0.0..1.0) < 0.20 {
                let mut npc_name = get_npc_display_label_at_hour(&t.entity_id, game_hour).unwrap_or_default();
                if npc_name.is_empty() {
                    npc_name = t.entity_id.clone();
                }
                let nar = build_departure_narrative(&npc_name, intent.ty);
                if !nar.is_empty() {
                    t.departure_narrative = nar;
                }
            }
            let path = resolve_brain_path(g, &intent, current_room, &ctx, 25);
            if path.is_empty() {
                None
            } else {
                Some(path)
            }
        }
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

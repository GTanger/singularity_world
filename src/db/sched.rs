// 排班目標與 ApplySchedules，對齊既有 db/schedule（行程／敘事用）。

use crate::store;

use super::npc_display::{get_npc_display_label_at_hour, NpcSchedule};
use super::ErrNoStore;

/// 排班目標房間與是否為上班地（對齊既有 `ScheduleTarget`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleTarget {
    pub room: String,
    pub is_work: bool,
}

/// 排班敘事用一筆移動意圖（對齊既有 `ScheduleMove`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleMove {
    pub entity_id: String,
    pub title: String,
    pub old_room: String,
    pub new_room: String,
}

/// 依排班與遊戲小時回傳目標房；無排班則 `None`（對齊既有 `GetScheduleTargetRoom`）。
#[must_use]
pub fn get_schedule_target_room(entity_id: &str, game_hour: i32) -> Option<String> {
    get_schedule_target(entity_id, game_hour).map(|t| t.room)
}

/// 排班目標與是否為上班地（對齊既有 `GetScheduleTarget`）。
#[must_use]
pub fn get_schedule_target(entity_id: &str, game_hour: i32) -> Option<ScheduleTarget> {
    let arc = store::get_store()?;
    let s = arc.read();
    let sch = s.get_schedule(entity_id)?;
    let ns = NpcSchedule {
        entity_id: sch.entity_id.clone(),
        work_room: sch.work_room.clone(),
        rest_room: sch.rest_room.clone(),
        shift_start: sch.shift_start,
        shift_end: sch.shift_end,
    };
    if ns.is_on_duty(game_hour) {
        Some(ScheduleTarget {
            room: sch.work_room.clone(),
            is_work: true,
        })
    } else {
        Some(ScheduleTarget {
            room: sch.rest_room.clone(),
            is_work: false,
        })
    }
}

/// 所有 NPC 排班（對齊既有 `GetAllSchedules`）。
pub fn get_all_schedules() -> anyhow::Result<Vec<NpcSchedule>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read();
    Ok(s
        .get_all_schedules()
        .into_iter()
        .map(|sch| NpcSchedule {
            entity_id: sch.entity_id,
            work_room: sch.work_room,
            rest_room: sch.rest_room,
            shift_start: sch.shift_start,
            shift_end: sch.shift_end,
        })
        .collect())
}

/// 設定 NPC 排班；寫入 `schedules.json`（對齊既有 `db.InsertSchedule`）。
pub fn insert_schedule(
    entity_id: &str,
    work_room: &str,
    rest_room: &str,
    shift_start: i32,
    shift_end: i32,
) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.insert_schedule(entity_id, work_room, rest_room, shift_start, shift_end)
}

/// 移除某實體排班（對齊既有 `RemoveScheduleForEntity`）。
pub fn remove_schedule_for_entity(entity_id: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.remove_schedule_for_entity(entity_id)
}

/// 依當前遊戲小時回傳「應前往房間」清單，不寫入 `entity_room`（對齊既有 `ApplySchedules`）。
pub fn apply_schedules(game_hour: i32) -> anyhow::Result<Vec<ScheduleMove>> {
    let schedules = get_all_schedules()?;
    let mut moves = Vec::new();
    for s in schedules {
        let arc = store::get_store().ok_or(ErrNoStore)?;
        let (vit_ok, current_room, target_room) = {
            let st = arc.read();
            let Some(ent) = st.get_entity(&s.entity_id) else {
                continue;
            };
            if ent.vit <= 0 {
                (false, String::new(), String::new())
            } else {
                let target = if s.is_on_duty(game_hour) {
                    s.work_room.clone()
                } else {
                    s.rest_room.clone()
                };
                (true, st.get_entity_room(&s.entity_id), target)
            }
        };
        if !vit_ok {
            continue;
        }
        if current_room == target_room {
            continue;
        }
        let title = get_npc_display_label_at_hour(&s.entity_id, game_hour).unwrap_or_default();
        moves.push(ScheduleMove {
            entity_id: s.entity_id.clone(),
            title,
            old_room: current_room,
            new_room: target_room,
        });
    }
    Ok(moves)
}

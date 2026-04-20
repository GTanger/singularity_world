// NPC 同房顯示字串、真名產生，對齊既有 db/schedule（顯示相關）與 db/npc_names。

use crate::store::{self, Store};

use super::npc_names::generate_npc_name;
use super::ErrNoStore;

/// 排班：班次起迄判斷（對齊既有 `NPCSchedule`）。
#[derive(Debug, Clone)]
pub struct NpcSchedule {
    pub entity_id: String,
    pub work_room: String,
    pub rest_room: String,
    pub shift_start: i32,
    pub shift_end: i32,
}

impl NpcSchedule {
    /// 遊戲時 `game_hour`（0–23）是否在班。
    #[must_use]
    pub fn is_on_duty(&self, game_hour: i32) -> bool {
        if self.shift_start <= self.shift_end {
            game_hour >= self.shift_start && game_hour < self.shift_end
        } else {
            game_hour >= self.shift_start || game_hour < self.shift_end
        }
    }
}

/// 取得單一實體排班（對齊既有 `GetScheduleForEntity`）。
#[must_use]
pub fn get_schedule_for_entity(entity_id: &str) -> Option<NpcSchedule> {
    let arc = store::get_store()?;
    let s = arc.read();
    s.get_schedule(entity_id).map(|sch| NpcSchedule {
        entity_id: sch.entity_id.clone(),
        work_room: sch.work_room.clone(),
        rest_room: sch.rest_room.clone(),
        shift_start: sch.shift_start,
        shift_end: sch.shift_end,
    })
}

/// 解析「職稱|真名」；無 `|` 則整段為真名（對齊既有 `SplitNPCListDisplayLabel`）。
#[must_use]
pub fn split_npc_list_display_label(label: &str) -> (String, String) {
    let label = label.trim();
    let Some(i) = label.find('|') else {
        return (String::new(), label.to_string());
    };
    let occ = label[..i].trim().to_string();
    let person = label[i + 1..].trim().to_string();
    (occ, person)
}

/// 從列表顯示字串取出真名（對齊既有 `PersonNameFromNPCListLabel`）。
#[must_use]
pub fn person_name_from_npc_list_label(label: &str) -> String {
    let (_, p) = split_npc_list_display_label(label);
    if !p.is_empty() {
        return p;
    }
    label.trim().to_string()
}

fn occupation_for_entity_at_room(s: &Store, entity_id: &str, room_id: &str) -> String {
    if room_id.is_empty() {
        return String::new();
    }
    for a in s.get_assignments_for_entity(entity_id) {
        if a.occupation_id.is_empty() {
            continue;
        }
        if s.is_room_in_venue(room_id, &a.venue_id) {
            return a.occupation_id;
        }
    }
    String::new()
}

/// 在已持有 `&mut Store` 下確保 NPC 有真名並寫回（對齊既有 `GetNPCPersonDisplayName` 行為）。
pub(crate) fn npc_person_display_name_locked(s: &mut Store, entity_id: &str) -> String {
    let Some(e) = s.get_entity(entity_id) else {
        return String::new();
    };
    if !e.display_title.is_empty() {
        return e.display_title;
    }
    if e.kind != "npc" {
        return String::new();
    }
    let name = generate_npc_name(&e.gender);
    let name_clone = name.clone();
    let _ = s.update_entity(entity_id, |ent| {
        ent.display_title = name_clone;
    });
    name
}

fn npc_list_display_and_behavior_role_locked(
    s: &mut Store,
    entity_id: &str,
    room_id: &str,
    game_hour: i32,
) -> (String, String) {
    let person = npc_person_display_name_locked(s, entity_id);
    let occ = occupation_for_entity_at_room(s, entity_id, room_id);
    if occ.is_empty() {
        return (person, String::new());
    }
    let mut off_duty = false;
    if let Some(sch) = s.get_schedule(entity_id) {
        let ns = NpcSchedule {
            entity_id: sch.entity_id.clone(),
            work_room: sch.work_room.clone(),
            rest_room: sch.rest_room.clone(),
            shift_start: sch.shift_start,
            shift_end: sch.shift_end,
        };
        if (0..=23).contains(&game_hour) && !ns.is_on_duty(game_hour) {
            off_duty = true;
        }
    }
    if off_duty {
        return (person, String::new());
    }
    (format!("{occ}|{person}"), occ)
}

/// 同房列表／視野用顯示字串（對齊既有 `GetNPCTitleInRoomAtHour`）。
pub fn get_npc_title_in_room_at_hour(
    entity_id: &str,
    room_id: &str,
    game_hour: i32,
) -> anyhow::Result<String> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    let (list_display, _) = npc_list_display_and_behavior_role_locked(&mut s, entity_id, room_id, game_hour);
    Ok(list_display)
}

/// 不傳遊戲小時：在職場內一律「職稱|真名」（對齊既有 `GetNPCTitleInRoom`）。
pub fn get_npc_title_in_room(entity_id: &str, room_id: &str) -> anyhow::Result<String> {
    get_npc_title_in_room_at_hour(entity_id, room_id, -1)
}

/// 敘事／移動用（對齊既有 `GetNPCDisplayLabelAtHour`）。
pub fn get_npc_display_label_at_hour(entity_id: &str, game_hour: i32) -> anyhow::Result<String> {
    let room_id = super::get_entity_room(entity_id)?;
    get_npc_title_in_room_at_hour(entity_id, &room_id, game_hour)
}

/// 無遊戲小時語境時等同 `hour = -1`（對齊既有 `GetNPCTitle`）。
pub fn get_npc_title(entity_id: &str) -> anyhow::Result<String> {
    let room_id = super::get_entity_room(entity_id)?;
    get_npc_title_in_room_at_hour(entity_id, &room_id, -1)
}

/// 回傳 NPC 真名；無則自動產生並寫回 store（對齊既有 `GetNPCPersonDisplayName`）。
pub fn get_npc_person_display_name(entity_id: &str) -> anyhow::Result<String> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    Ok(npc_person_display_name_locked(&mut s, entity_id))
}

/// 供 `get_entities_in_room` 等在已持有 `write` 鎖時呼叫。
pub(crate) fn npc_title_in_room_locked(
    s: &mut Store,
    entity_id: &str,
    room_id: &str,
    game_hour: i32,
) -> String {
    npc_list_display_and_behavior_role_locked(s, entity_id, room_id, game_hour).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_on_duty_same_day_span() {
        let s = NpcSchedule {
            entity_id: "a".into(),
            work_room: "".into(),
            rest_room: "".into(),
            shift_start: 9,
            shift_end: 18,
        };
        assert!(!s.is_on_duty(8));
        assert!(s.is_on_duty(9));
        assert!(s.is_on_duty(17));
        assert!(!s.is_on_duty(18));
    }

    #[test]
    fn is_on_duty_cross_midnight() {
        let s = NpcSchedule {
            entity_id: "a".into(),
            work_room: "".into(),
            rest_room: "".into(),
            shift_start: 18,
            shift_end: 7,
        };
        assert!(s.is_on_duty(18));
        assert!(s.is_on_duty(23));
        assert!(s.is_on_duty(0));
        assert!(!s.is_on_duty(7));
        assert!(!s.is_on_duty(12));
    }

    #[test]
    fn split_label() {
        assert_eq!(
            split_npc_list_display_label(" 店長|張三 "),
            ("店長".to_string(), "張三".to_string())
        );
        assert_eq!(
            split_npc_list_display_label("李四"),
            (String::new(), "李四".to_string())
        );
    }
}

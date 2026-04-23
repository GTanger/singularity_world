//! store 工作分派與排班 — assignments + schedules 的讀寫。
//! 寫入路徑同時走 writer queue（PG 權威）和 persist 備份（JSON fallback）。

use super::{Assignment, Schedule, Store};

impl Store {
    // ── Assignments ──

    pub fn get_assignments_for_entity(&self, entity_id: &str) -> Vec<Assignment> {
        self.assignments
            .get(entity_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_assignment_count_by_venue(&self, venue_id: &str) -> usize {
        self.assignments
            .values()
            .flat_map(|v| v.iter())
            .filter(|a| a.venue_id == venue_id)
            .count()
    }

    /// 該場所既有指派中的第一個職業 ID（對齊既有 `GetFirstOccupationIDForVenue`）。
    pub fn get_first_occupation_id_for_venue(&self, venue_id: &str) -> String {
        for list in self.assignments.values() {
            for a in list {
                if a.venue_id == venue_id {
                    return a.occupation_id.clone();
                }
            }
        }
        String::new()
    }

    /// 新增指派；同 entity+occupation+venue 已存在則忽略（對齊既有 `InsertAssignment`）。
    pub fn insert_assignment(
        &mut self,
        entity_id: &str,
        occupation_id: &str,
        venue_id: &str,
        assigned_by: &str,
    ) -> anyhow::Result<()> {
        let list = self.assignments.entry(entity_id.to_string()).or_default();
        for a in list.iter() {
            if a.occupation_id == occupation_id && a.venue_id == venue_id {
                return Ok(());
            }
        }
        list.push(Assignment {
            entity_id: entity_id.to_string(),
            occupation_id: occupation_id.to_string(),
            venue_id: venue_id.to_string(),
            assigned_by: assigned_by.to_string(),
        });
        crate::pg::writer::submit(crate::pg::writer::WriteOp::InsertAssignment {
            entity_id: entity_id.to_string(),
            occupation_id: occupation_id.to_string(),
            venue_id: venue_id.to_string(),
            assigned_by: assigned_by.to_string(),
        });
        self.persist_assignments()
    }

    pub fn remove_assignments_for_entity(&mut self, entity_id: &str) -> anyhow::Result<()> {
        self.assignments.remove(entity_id);
        crate::pg::writer::submit(crate::pg::writer::WriteOp::RemoveAssignments {
            entity_id: entity_id.to_string(),
        });
        self.persist_assignments()
    }

    // ── Schedules ──

    pub fn get_all_schedules(&self) -> Vec<Schedule> {
        self.schedules.values().cloned().collect()
    }

    pub fn get_schedule(&self, entity_id: &str) -> Option<&Schedule> {
        self.schedules.get(entity_id)
    }

    pub fn insert_schedule(
        &mut self,
        entity_id: &str,
        work_room: &str,
        rest_room: &str,
        shift_start: i32,
        shift_end: i32,
    ) -> anyhow::Result<()> {
        self.schedules.insert(
            entity_id.to_string(),
            Schedule {
                entity_id: entity_id.to_string(),
                work_room: work_room.to_string(),
                rest_room: rest_room.to_string(),
                shift_start,
                shift_end,
            },
        );
        crate::pg::writer::submit(crate::pg::writer::WriteOp::InsertSchedule {
            entity_id: entity_id.to_string(),
            work_room: work_room.to_string(),
            rest_room: rest_room.to_string(),
            shift_start,
            shift_end,
        });
        self.persist_schedules()
    }

    pub fn remove_schedule_for_entity(&mut self, entity_id: &str) -> anyhow::Result<()> {
        self.schedules.remove(entity_id);
        crate::pg::writer::submit(crate::pg::writer::WriteOp::RemoveSchedule {
            entity_id: entity_id.to_string(),
        });
        self.persist_schedules()
    }
}

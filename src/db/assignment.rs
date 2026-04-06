// 場所與指派門面，對齊既有 db/assignment。

pub use crate::store::{Assignment, Venue};

use crate::store;

use super::ErrNoStore;

/// 確保預設場所存在；現由 `venues.json` 於 store 初始化載入，此函式為 no-op（對齊既有 `SeedVenues`）。
pub fn seed_venues() -> anyhow::Result<()> {
    Ok(())
}

/// 新增一筆指派；同 entity+occupation+venue 已存在則忽略（對齊既有 `InsertAssignment`）。
pub fn insert_assignment(
    entity_id: &str,
    occupation_id: &str,
    venue_id: &str,
    assigned_by: &str,
) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.insert_assignment(entity_id, occupation_id, venue_id, assigned_by)
}

/// 某實體的全部指派（對齊既有 `GetAssignmentsForEntity`）。
pub fn get_assignments_for_entity(entity_id: &str) -> anyhow::Result<Vec<Assignment>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_assignments_for_entity(entity_id))
}

/// 依指派推導職稱；無指派則空字串（對齊既有 `GetNPCTitleFromAssignments`）。
#[must_use]
pub fn get_npc_title_from_assignments(entity_id: &str) -> String {
    let Ok(list) = get_assignments_for_entity(entity_id) else {
        return String::new();
    };
    list.first().map(|a| a.occupation_id.clone()).unwrap_or_default()
}

/// 移除某實體的全部指派（對齊既有 `RemoveAssignmentsForEntity`）。
pub fn remove_assignments_for_entity(entity_id: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.remove_assignments_for_entity(entity_id)
}

/// 該場所職缺上限；0 用 `default_max`（對齊既有 `GetVenueMaxStaff`）。
#[must_use]
pub fn get_venue_max_staff(venue_id: &str, default_max: i32) -> i32 {
    let Some(arc) = store::get_store() else {
        return default_max;
    };
    let s = arc.read().unwrap();
    s.get_venue_max_staff(venue_id, default_max)
}

/// 該場所既有指派中的第一個職業 ID（對齊既有 `GetFirstOccupationIDForVenue`）。
#[must_use]
pub fn get_first_occupation_id_for_venue(venue_id: &str) -> String {
    let Some(arc) = store::get_store() else {
        return String::new();
    };
    let s = arc.read().unwrap();
    s.get_first_occupation_id_for_venue(venue_id)
}

/// 該場所涵蓋的房間 ID（對齊既有 `GetRoomIDsForVenue`）。
pub fn get_room_ids_for_venue(venue_id: &str) -> anyhow::Result<Option<Vec<String>>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_venue(venue_id).map(|v| v.room_ids.clone()))
}

/// 房間是否在該場所內（對齊既有 `IsRoomInVenue`）。
pub fn is_room_in_venue(room_id: &str, venue_id: &str) -> anyhow::Result<bool> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.is_room_in_venue(room_id, venue_id))
}

/// 實體在指定房間時是否處於任一指派場所內（對齊既有 `EntityInVenueAtRoom`）。
pub fn entity_in_venue_at_room(entity_id: &str, room_id: &str) -> anyhow::Result<bool> {
    for a in get_assignments_for_entity(entity_id)? {
        if is_room_in_venue(room_id, &a.venue_id)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// 包含該房間的所有場所 ID（對齊既有 `GetVenueIDsForRoom`）。
pub fn get_venue_ids_for_room(room_id: &str) -> anyhow::Result<Vec<String>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_venue_ids_for_room(room_id))
}

/// 該場所目前的指派數量（對齊既有 `GetAssignmentCountByVenue`）。
pub fn get_assignment_count_by_venue(venue_id: &str) -> anyhow::Result<usize> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_assignment_count_by_venue(venue_id))
}

/// 所有場所 ID（對齊既有 `GetAllVenueIDs`）。
pub fn get_all_venue_ids() -> anyhow::Result<Vec<String>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_all_venue_ids())
}

/// 所有場所涵蓋的房間 ID（對齊既有 `GetAllVenueRoomIDs`）。
pub fn get_all_venue_room_ids() -> anyhow::Result<Vec<String>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read().unwrap();
    Ok(s.get_all_venue_room_ids())
}

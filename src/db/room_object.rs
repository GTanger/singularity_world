//! 房間內可互動物件查詢（對齊既有 `db/room_object` 子集）。

use crate::model;
use crate::store;

/// 依物件敘事表回傳動詞對應文字；無則空字串。
#[must_use]
pub fn object_response(obj: &model::RoomObject, action: &str) -> String {
    obj.responses.get(action).cloned().unwrap_or_default()
}

/// 在指定房間內依物件 ID 找物件（同房列表為準）。
#[must_use]
pub fn get_object_by_id_in_room(room_id: &str, object_id: &str) -> Option<model::RoomObject> {
    if room_id.is_empty() || object_id.is_empty() {
        return None;
    }
    let arc = store::get_store()?;
    let s = arc.read().unwrap();
    let room = s.get_room(room_id)?;
    room.objects.into_iter().find(|o| o.id == object_id)
}

/// 在指定房間內依顯示名稱找物件。
#[must_use]
pub fn get_object_by_name_in_room(room_id: &str, name: &str) -> Option<model::RoomObject> {
    let name = name.trim();
    if room_id.is_empty() || name.is_empty() {
        return None;
    }
    let arc = store::get_store()?;
    let s = arc.read().unwrap();
    let room = s.get_room(room_id)?;
    room.objects.into_iter().find(|o| o.name == name)
}

/// 依物件 ID 全域找物件與所屬房間 ID。
#[must_use]
pub fn get_object_and_room(object_id: &str) -> Option<(model::RoomObject, String)> {
    if object_id.is_empty() {
        return None;
    }
    let arc = store::get_store()?;
    let s = arc.read().unwrap();
    for rid in s.room_ids() {
        let room = s.get_room(&rid)?;
        if let Some(obj) = room.objects.iter().find(|o| o.id == object_id) {
            return Some((obj.clone(), rid));
        }
    }
    None
}

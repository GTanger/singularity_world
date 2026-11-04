//! 未觀測時輕量背景模擬（對齊既有 `npc/unobserved`）；不發敘事。

use rand::Rng;

use crate::db::{
    RoomGraph, add_to_inventory, get_all_venue_ids, get_assignment_count_by_venue,
    get_assignments_for_entity, get_entity, get_entity_room, get_npc_ids_with_room,
    get_schedule_target_room, insert_assignment, npc_street_sell_one, set_entity_room,
};
use crate::world::materials;

/// 與常數 `SurvivalLineMg` 一致：低於此鎂才可能在未觀測時補鎂。
pub const SURVIVAL_LINE_MG: i32 = 50;
/// 每輪最多處理的 NPC 數（對齊 `UnobservedMaxNPCsPerTick`）。
pub const UNOBSERVED_MAX_NPCS_PER_TICK: usize = 50;

const UNOBSERVED_EFFECT_DENOM: i32 = 4;
const DEFAULT_OCC_FOR_UNOBSERVED_HIRE: &str = "服務生";

/// 依據房間名稱解析未觀測雇傭時的預設職業稱謂（優先使用場館物件預設稱謂）。
pub fn resolve_unobserved_hire_occupation_for_room(room_name: &str) -> &'static str {
    if let Some(venue) = crate::grid::resolve_any_room_venue_object(room_name.trim(), "") {
        venue.default_host_title
    } else {
        DEFAULT_OCC_FOR_UNOBSERVED_HIRE
    }
}

/// Fisher–Yates 洗牌。
fn shuffle_vec<T>(v: &mut [T], rng: &mut impl Rng) {
    for i in (1..v.len()).rev() {
        let j = rng.random_range(0..=i);
        v.swap(i, j);
    }
}

/// 在無人處於「觀測圈」時呼叫：抽樣最多 `max_npcs` 個 NPC，讓事實持續（排班走一步／無排班隨機效果）。不發敘事。
pub fn run_unobserved_world_tick(
    graph: Option<&RoomGraph>,
    game_hour: i32,
    max_npcs: usize,
    now_unix: i64,
) {
    let mut max_n = max_npcs;
    if max_n == 0 {
        max_n = UNOBSERVED_MAX_NPCS_PER_TICK;
    }
    let mut ids = get_npc_ids_with_room();
    if ids.is_empty() {
        return;
    }
    let mut rng = rand::rng();
    shuffle_vec(&mut ids, &mut rng);
    if ids.len() > max_n {
        ids.truncate(max_n);
    }
    let Ok(venue_ids) = get_all_venue_ids() else {
        return;
    };
    for entity_id in ids {
        let Ok(current_room) = get_entity_room(&entity_id) else {
            continue;
        };
        if let Some(ref tr) = get_schedule_target_room(&entity_id, game_hour)
            && !tr.is_empty()
        {
            if current_room != *tr
                && let Some(g) = graph
                && let Some(path) = g.find_path(&current_room, tr)
                && let Some(next) = path.first()
            {
                let _ = set_entity_room(&entity_id, next);
            }
            continue;
        }
        let effect = rng.random_range(0..UNOBSERVED_EFFECT_DENOM);
        if effect == 0 {
            let Ok(Some(ch)) = get_entity(&entity_id) else {
                continue;
            };
            if ch.magnesium < SURVIVAL_LINE_MG {
                // 低鎂只會嘗試把實際持物賣給同地真買家，不再背景補發不存在的錢。
                let _ = npc_street_sell_one(&entity_id, &current_room, now_unix);
            }
        } else if effect == 1 {
            if !current_room.starts_with("wt-physical-")
                && let Some(gathered) =
                    materials::try_gather_material_for_room_preserving(&current_room, now_unix, 1)
            {
                let _ = add_to_inventory(&entity_id, &gathered.item_id, 1);
                let room_name = crate::db::get_room_name(&current_room)
                    .unwrap_or_else(|_| current_room.clone());
                let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
                    &room_name,
                    &gathered.name,
                );
            }
        } else if effect == 2 {
            let Ok(list) = get_assignments_for_entity(&entity_id) else {
                continue;
            };
            if !list.is_empty() || venue_ids.is_empty() {
                continue;
            }
            let mut vids = venue_ids.clone();
            shuffle_vec(&mut vids, &mut rng);
            for vid in vids {
                let Ok(n) = get_assignment_count_by_venue(&vid) else {
                    continue;
                };
                if n < 2 {
                    let venue_room_name = crate::db::get_room_ids_for_venue(&vid)
                        .ok()
                        .flatten()
                        .and_then(|rids| rids.into_iter().next())
                        .and_then(|rid| crate::db::get_room_name(&rid).ok())
                        .unwrap_or_default();
                    let occ = resolve_unobserved_hire_occupation_for_room(&venue_room_name);
                    let _ = insert_assignment(&entity_id, occ, &vid, "");
                    break;
                }
            }
        } else {
            let Some(g) = graph else {
                continue;
            };
            if current_room.is_empty() {
                continue;
            }
            let neighbors = g.neighbors(&current_room);
            if neighbors.is_empty() {
                continue;
            }
            let idx = rng.random_range(0..neighbors.len());
            let next = neighbors[idx].clone();
            let _ = set_entity_room(&entity_id, &next);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_unobserved_hire_occupation_for_room() {
        // 1. 水輪石磨工坊 -> 當值石磨匠師
        let mill_occ = resolve_unobserved_hire_occupation_for_room("老堰頭・水輪石磨工坊");
        assert_eq!(mill_occ, "當值石磨匠師");

        // 2. 三水大食堂 -> 食堂主廚
        let canteen_occ = resolve_unobserved_hire_occupation_for_room("三水大食堂");
        assert_eq!(canteen_occ, "食堂主廚");

        // 3. 無特定場館物件的普通地界 -> 服務生
        let default_occ = resolve_unobserved_hire_occupation_for_room("荒蕪礫石空地");
        assert_eq!(default_occ, "服務生");
    }
}

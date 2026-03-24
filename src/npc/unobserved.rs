//! 未觀測時輕量背景模擬（對齊 Go `npc/unobserved.go`）；不發敘事。

use rand::Rng;

use crate::db::{
    add_magnesium, add_to_inventory, get_all_venue_ids, get_assignment_count_by_venue,
    get_assignments_for_entity, get_entity, get_entity_room, get_npc_ids_with_room, get_schedule_target_room,
    insert_assignment, set_entity_room, RoomGraph,
};

/// 與 Go `npc.SurvivalLineMg` 一致：低於此鎂才可能在未觀測時補鎂。
pub const SURVIVAL_LINE_MG: i32 = 50;
/// 每輪最多處理的 NPC 數（對齊 `UnobservedMaxNPCsPerTick`）。
pub const UNOBSERVED_MAX_NPCS_PER_TICK: usize = 50;

const UNOBSERVED_EFFECT_DENOM: i32 = 4;
const DEFAULT_OCC_FOR_UNOBSERVED_HIRE: &str = "服務生";

/// Fisher–Yates 洗牌。
fn shuffle_vec<T>(v: &mut [T], rng: &mut impl Rng) {
    for i in (1..v.len()).rev() {
        let j = rng.random_range(0..=i);
        v.swap(i, j);
    }
}

/// 在無人處於「觀測圈」時呼叫：抽樣最多 `max_npcs` 個 NPC，讓事實持續（排班走一步／無排班隨機效果）。不發敘事。
pub fn run_unobserved_world_tick(graph: Option<&RoomGraph>, game_hour: i32, max_npcs: usize) {
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
                let delta = 3 + rng.random_range(0..8);
                let _ = add_magnesium(&entity_id, delta);
            }
        } else if effect == 1 {
            let _ = add_to_inventory(&entity_id, "wild_herb", 1);
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
                    let _ = insert_assignment(&entity_id, DEFAULT_OCC_FOR_UNOBSERVED_HIRE, &vid, "");
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

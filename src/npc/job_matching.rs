//! 求職撮合（對齊既有 `npc/job_matching`）。

use rand::Rng;

use crate::db::{
    get_all_venue_ids, get_assignment_count_by_venue, get_assignments_for_entity, get_entity,
    get_entity_room, get_first_occupation_id_for_venue, get_npc_ids_with_room, get_room_ids_for_venue,
    get_spawn_room_id, get_venue_max_staff, insert_assignment, insert_schedule, RoomGraph,
};

/// 鎂閾值預設（config 為 0 時使用）。
pub const SEEK_JOB_MG_THRESHOLD_DEFAULT: i32 = 50;
/// 鎂≥閾值且開啟穩定撮合時，參與機率。
pub const JOB_MATCH_STABLE_PROBABILITY: f32 = 0.3;
/// 無既有職業時寫入的預設職業 id。
pub const DEFAULT_OCCUPATION_FOR_HIRE: &str = "服務生";
const DEFAULT_SHIFT_START: i32 = 6;
const DEFAULT_SHIFT_END: i32 = 19;

/// 撮合參數（對齊既有 `JobMatchParams`）。
#[derive(Debug, Clone)]
pub struct JobMatchParams {
    pub max_per_venue: i32,
    pub mg_threshold: i32,
    pub job_match_when_stable: bool,
}

/// Fisher–Yates 洗牌。
fn shuffle_vec<T>(v: &mut [T], rng: &mut impl Rng) {
    for i in (1..v.len()).rev() {
        let j = rng.random_range(0..=i);
        v.swap(i, j);
    }
}

fn do_assign(entity_id: &str, venue_id: &str, spawn_room: &str) -> bool {
    let mut occupation = get_first_occupation_id_for_venue(venue_id);
    if occupation.is_empty() {
        occupation = DEFAULT_OCCUPATION_FOR_HIRE.to_string();
    }
    if insert_assignment(entity_id, &occupation, venue_id, "job_matching").is_err() {
        return false;
    }
    let room_ids = get_room_ids_for_venue(venue_id)
        .ok()
        .flatten()
        .unwrap_or_default();
    let work_room = room_ids.first().cloned().unwrap_or_default();
    if !work_room.is_empty() && !spawn_room.is_empty() {
        let _ = insert_schedule(
            entity_id,
            &work_room,
            spawn_room,
            DEFAULT_SHIFT_START,
            DEFAULT_SHIFT_END,
        );
    }
    true
}

/// 一輪求職撮合；`graph` 為 `None` 時改隨機配對（與既有行為一致）。回傳本輪新入職 entity id。
#[must_use]
pub fn run_job_matching_tick(graph: Option<&RoomGraph>, mut p: JobMatchParams) -> Vec<String> {
    let mut added = Vec::new();
    if p.max_per_venue <= 0 {
        p.max_per_venue = 2;
    }
    let mut threshold = p.mg_threshold;
    if threshold <= 0 {
        threshold = SEEK_JOB_MG_THRESHOLD_DEFAULT;
    }

    let ids = get_npc_ids_with_room();
    if ids.is_empty() {
        return added;
    }

    let mut rng = rand::rng();
    let mut unassigned: Vec<String> = Vec::new();
    for entity_id in ids {
        let Ok(list) = get_assignments_for_entity(&entity_id) else {
            continue;
        };
        if !list.is_empty() {
            continue;
        }
        let Ok(Some(ch)) = get_entity(&entity_id) else {
            continue;
        };
        if ch.magnesium < threshold
            || (p.job_match_when_stable && rng.random_bool(f64::from(JOB_MATCH_STABLE_PROBABILITY)))
        {
            unassigned.push(entity_id);
        }
    }
    if unassigned.is_empty() {
        return added;
    }

    let Ok(venue_ids) = get_all_venue_ids() else {
        return added;
    };
    if venue_ids.is_empty() {
        return added;
    }

    let mut slots: Vec<String> = Vec::new();
    for vid in venue_ids {
        let Ok(n) = get_assignment_count_by_venue(&vid) else {
            continue;
        };
        let limit = get_venue_max_staff(&vid, p.max_per_venue);
        for _ in 0..limit.saturating_sub(n as i32) {
            slots.push(vid.clone());
        }
    }
    if slots.is_empty() {
        return added;
    }

    let spawn_room = get_spawn_room_id();

    if let Some(g) = graph {
        shuffle_vec(&mut slots, &mut rng);
        let mut un = unassigned;
        for venue_id in slots {
            let room_ids = get_room_ids_for_venue(&venue_id)
                .ok()
                .flatten()
                .unwrap_or_default();
            if room_ids.is_empty() {
                continue;
            }
            let venue_first = room_ids[0].clone();
            let mut best_idx: Option<usize> = None;
            let mut best_dist = 9999i32;
            for (i, entity_id) in un.iter().enumerate() {
                let Ok(current) = get_entity_room(entity_id) else {
                    continue;
                };
                if current.is_empty() {
                    continue;
                }
                let dist = if current == venue_first {
                    0
                } else if let Some(path) = g.find_path(&current, &venue_first) {
                    path.len() as i32
                } else {
                    9999
                };
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = Some(i);
                }
            }
            let Some(idx) = best_idx else {
                continue;
            };
            let entity_id = un.remove(idx);
            if do_assign(&entity_id, &venue_id, &spawn_room) {
                added.push(entity_id);
            }
            if un.is_empty() {
                break;
            }
        }
    } else {
        shuffle_vec(&mut unassigned, &mut rng);
        shuffle_vec(&mut slots, &mut rng);
        let n = unassigned.len().min(slots.len());
        for i in 0..n {
            if do_assign(&unassigned[i], &slots[i], &spawn_room) {
                added.push(unassigned[i].clone());
            }
        }
    }

    added
}

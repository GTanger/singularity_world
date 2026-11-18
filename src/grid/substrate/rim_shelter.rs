//! 開譜曆山雨氣候、單平面圍合室內判定、`竹編睡榻`休養與避雨閉環模組。

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::rim_sim::{ActorFaction, PendingTask, RimWorldState};
use super::rim_world::{MicroCellOverlay, StructureKind, find_micro_path_crowded};
use super::runtime::SubstrateRuntime;

const MAX_ROOM_INTERIOR: usize = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RimWeatherKind {
    Clear,
    MountainMist,
    SummerSquall,
}

impl RimWeatherKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::MountainMist => "drizzle",
            Self::SummerSquall => "rain",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Clear => "晴風和暢",
            Self::MountainMist => "山嵐微雨",
            Self::SummerSquall => "瀟瀟山雨",
        }
    }

    pub fn is_raining(self) -> bool {
        matches!(self, Self::MountainMist | Self::SummerSquall)
    }
}

/// 依權威開譜曆季節與兩時辰窗口決定當前天氣（每 2 遊戲小時 ≈ 5 分鐘真實時間切換一次）。
pub fn weather_at_unix(real_unix: i64) -> RimWeatherKind {
    let (epoch, scale) = crate::game::game_time_cfg();
    let (_, hour, _, days) = crate::game::game_time_now(real_unix, epoch, scale);
    let slot = i64::from(days).saturating_mul(12).saturating_add(i64::from(hour / 2));
    let doy = days.rem_euclid(365);

    let mut z = (slot as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(0x517C_C1B7_2722_0A95);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let roll = ((z ^ (z >> 31)) % 100) as u32;

    if (91..182).contains(&doy) {
        // 夏漲多午後山雨
        if roll < 24 {
            RimWeatherKind::SummerSquall
        } else if roll < 42 {
            RimWeatherKind::MountainMist
        } else {
            RimWeatherKind::Clear
        }
    } else if doy < 91 {
        // 春融細雨濛濛
        if roll < 14 {
            RimWeatherKind::SummerSquall
        } else if roll < 32 {
            RimWeatherKind::MountainMist
        } else {
            RimWeatherKind::Clear
        }
    } else if roll < 10 {
        RimWeatherKind::SummerSquall
    } else if roll < 20 {
        RimWeatherKind::MountainMist
    } else {
        RimWeatherKind::Clear
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EnclosedRoom {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub doors: Vec<(i32, i32)>,
    pub cells: Vec<(i32, i32)>,
}

fn is_wall_or_door(ov: Option<&MicroCellOverlay>) -> bool {
    matches!(
        ov.and_then(|o| o.structure),
        Some(StructureKind::BambooWall | StructureKind::BambooDoor)
    )
}

/// 掃描由竹木圍牆與竹編院門四連通閉合之單平面室內空間（自動涵蓋玩家自建屋舍）。
pub fn detect_enclosed_rooms(overlays: &HashMap<(i32, i32), MicroCellOverlay>) -> Vec<EnclosedRoom> {
    let mut walls = Vec::new();
    for (&pos, ov) in overlays {
        if matches!(ov.structure, Some(StructureKind::BambooWall)) {
            walls.push(pos);
        }
    }
    if walls.len() < 4 {
        return Vec::new();
    }

    let mut checked_outside: HashSet<(i32, i32)> = HashSet::new();
    let mut seen_interior: HashSet<(i32, i32)> = HashSet::new();
    let mut rooms: Vec<EnclosedRoom> = Vec::new();

    for (wx, wy) in walls {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let seed = (wx + dx, wy + dy);
            if seen_interior.contains(&seed)
                || checked_outside.contains(&seed)
                || is_wall_or_door(overlays.get(&seed))
            {
                continue;
            }

            let mut comp: HashSet<(i32, i32)> = HashSet::new();
            let mut queue = VecDeque::new();
            let mut adj_doors: BTreeSet<(i32, i32)> = BTreeSet::new();
            let mut wall_touch_count = 0usize;
            let mut escaped = false;

            comp.insert(seed);
            queue.push_back(seed);

            while let Some((cx, cy)) = queue.pop_front() {
                if comp.len() > MAX_ROOM_INTERIOR {
                    escaped = true;
                    break;
                }
                for (nx_d, ny_d) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let np = (cx + nx_d, cy + ny_d);
                    let ov = overlays.get(&np);
                    if let Some(st) = ov.and_then(|o| o.structure) {
                        if st == StructureKind::BambooDoor {
                            adj_doors.insert(np);
                            continue;
                        }
                        if st == StructureKind::BambooWall {
                            wall_touch_count += 1;
                            continue;
                        }
                    }
                    if !comp.contains(&np) {
                        if checked_outside.contains(&np) {
                            escaped = true;
                            break;
                        }
                        comp.insert(np);
                        queue.push_back(np);
                    }
                }
                if escaped {
                    break;
                }
            }

            if escaped || comp.len() < 2 || wall_touch_count < 4 {
                checked_outside.extend(comp);
                continue;
            }

            let mut min_x = i32::MAX;
            let mut min_y = i32::MAX;
            let mut max_x = i32::MIN;
            let mut max_y = i32::MIN;
            for &(ix, iy) in &comp {
                seen_interior.insert((ix, iy));
                min_x = min_x.min(ix);
                min_y = min_y.min(iy);
                max_x = max_x.max(ix);
                max_y = max_y.max(iy);
            }
            let mut cells: Vec<(i32, i32)> = comp.into_iter().collect();
            cells.sort_unstable();
            rooms.push(EnclosedRoom {
                x0: min_x - 1,
                y0: min_y - 1,
                x1: max_x + 1,
                y1: max_y + 1,
                doors: adj_doors.into_iter().collect(),
                cells,
            });
        }
    }

    rooms.sort_by_key(|r| (r.x0, r.y0));
    rooms
}

pub fn sheltered_cell_set(overlays: &HashMap<(i32, i32), MicroCellOverlay>) -> HashSet<(i32, i32)> {
    let mut set = HashSet::new();
    for r in detect_enclosed_rooms(overlays) {
        for c in r.cells {
            set.insert(c);
        }
        for d in r.doors {
            set.insert(d);
        }
    }
    for (&pos, ov) in overlays {
        if ov.structure == Some(StructureKind::CoveredArcade) {
            set.insert(pos);
        }
    }
    set
}

/// 冪等為既有先鋒草寮與水尾寮茶寮補齊西側院門與竹編睡榻。
pub fn upgrade_existing_shelters(overlays: &mut HashMap<(i32, i32), MicroCellOverlay>) -> bool {
    let mut changed = false;

    // 若水尾寮 (28..=36, -10..=-4) 尚無西側院門 (28, -7)，補安竹編院門閉合茶寮
    if overlays
        .get(&(28, -6))
        .and_then(|o| o.structure)
        .is_some_and(|s| s == StructureKind::BambooWall)
    {
        let e = overlays.entry((28, -7)).or_default();
        if e.structure.is_none() && e.ground_item.is_none() {
            e.cleared_natural = true;
            e.structure = Some(StructureKind::BambooDoor);
            changed = true;
        }
    }

    // 若草寮與茶寮室內空位未置床榻，冪等補入竹編睡榻
    let bed_spots = [
        ((7, 1), (8, 4)),
        ((7, 1), (8, 2)),
        ((31, -7), (35, -5)),
        ((31, -7), (35, -9)),
    ];
    for (anchor, bed_pos) in bed_spots {
        if overlays.get(&anchor).and_then(|o| o.structure).is_some() {
            let e = overlays.entry(bed_pos).or_default();
            if e.structure.is_none() && e.ground_item.is_none() && e.blueprint.is_none() {
                e.cleared_natural = true;
                e.structure = Some(StructureKind::BambooBed);
                changed = true;
            }
        }
    }
    changed
}

/// 推進角色淋雨濕冷累積、竹笠遮雨、室內乾燥與竹編睡榻加倍養傷回氣。
pub fn step_weather_and_bed_recovery(world: &mut RimWorldState, weather: RimWeatherKind) {
    let sheltered = sheltered_cell_set(&world.overlays);
    for a in &mut world.actors {
        if a.faction != ActorFaction::Settler || a.hp <= 0 || a.downed_ticks > 0 {
            continue;
        }
        let pos = a.micro_pos();
        let is_sheltered = sheltered.contains(&pos);
        let on_bed = world
            .overlays
            .get(&pos)
            .and_then(|o| o.structure)
            == Some(StructureKind::BambooBed);

        if is_sheltered {
            a.wet_ticks = a.wet_ticks.saturating_sub(12);
        } else if weather.is_raining() {
            if a.equipped_hat {
                a.wet_ticks = a.wet_ticks.saturating_sub(2);
            } else {
                a.wet_ticks = a.wet_ticks.saturating_add(8).min(60);
                a.stamina = a.stamina.saturating_sub(2);
            }
        } else {
            a.wet_ticks = a.wet_ticks.saturating_sub(6);
        }

        if on_bed && a.path.is_empty() {
            a.stamina = a.stamina.saturating_add(18).min(100);
            a.hp = (a.hp + 3).min(a.max_hp);
            if a.injury_ticks > 0 {
                a.injury_ticks = a.injury_ticks.saturating_sub(5);
                if a.injury_ticks == 0 {
                    a.injury_part = None;
                    a.push_diary("在竹編睡榻歇息調理，傷勢已消");
                }
            }
            if a.task == PendingTask::None {
                a.activity_tag = "榻上安歇".into();
            }
        }
    }
}

/// 自主角色帶傷疲憊或遇山雨無笠時，主動尋求空閒竹編睡榻或入屋避雨。
pub fn try_seek_bed_or_rain_shelter(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    weather: RimWeatherKind,
) -> bool {
    let start = world.actors[idx].micro_pos();
    let sheltered = sheltered_cell_set(&world.overlays);
    let already_sheltered = sheltered.contains(&start);
    let need_bed = world.actors[idx].injury_ticks > 0
        || world.actors[idx].stamina < 42
        || world.actors[idx].directive == "rest";
    let need_rain_cover = weather.is_raining()
        && !already_sheltered
        && (!world.actors[idx].equipped_hat || world.actors[idx].wet_ticks >= 16);

    if !need_bed && !need_rain_cover {
        return false;
    }

    let claimed_beds: HashSet<(i32, i32)> = world
        .actors
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != idx)
        .filter_map(|(_, a)| match a.task {
            PendingTask::SleepBed { tx, ty } => Some((tx, ty)),
            _ => {
                let p = a.micro_pos();
                (world.overlays.get(&p).and_then(|o| o.structure) == Some(StructureKind::BambooBed))
                    .then_some(p)
            }
        })
        .collect();

    let my_key = world.actors[idx].actor_key.clone();
    let occ = world.occupied_cells(Some(idx));
    let own_bed = super::rim_town::preferred_bed_for(&my_key);
    let best_bed = own_bed
        .filter(|pos| {
            world.overlays.get(pos).and_then(|o| o.structure) == Some(StructureKind::BambooBed)
                && !claimed_beds.contains(pos)
                && (pos.0 - start.0).abs() + (pos.1 - start.1).abs() <= 44
        })
        .or_else(|| {
            world
                .overlays
                .iter()
                .filter(|&(pos, ov)| {
                    ov.structure == Some(StructureKind::BambooBed)
                        && !claimed_beds.contains(pos)
                        && !super::rim_town::is_other_household_bed(&my_key, *pos)
                })
                .filter(|&(&(bx, by), _)| (bx - start.0).abs() + (by - start.1).abs() <= 28)
                .min_by_key(|&(&(bx, by), _)| (bx - start.0).abs() + (by - start.1).abs())
                .map(|(&pos, _)| pos)
        });
    if let Some((bx, by)) = best_bed {
        if (bx - start.0).abs() + (by - start.1).abs() == 0 {
            world.actors[idx].activity_tag = "榻上安歇".into();
            return true;
        }
        if let Some(p) = find_micro_path_crowded(rt, &world.overlays, start, (bx, by), false, false, &occ) {
            world.actors[idx].path = p.into();
            world.actors[idx].task = PendingTask::SleepBed { tx: bx, ty: by };
            world.actors[idx].activity_tag = if need_rain_cover { "入屋就榻" } else { "前往就榻" }.into();
            return true;
        }
    }

    if need_rain_cover
        && let Some(&(sx, sy)) = sheltered
            .iter()
            .filter(|&pos| {
                !occ.contains(pos)
                    && world
                        .overlays
                        .get(pos)
                        .and_then(|o| o.structure)
                        .is_none_or(|s| !s.blocks_actor(false))
            })
            .filter(|&&(sx, sy)| (sx - start.0).abs() + (sy - start.1).abs() <= 26)
            .min_by_key(|&&(sx, sy)| (sx - start.0).abs() + (sy - start.1).abs())
        && let Some(p) = find_micro_path_crowded(rt, &world.overlays, start, (sx, sy), false, false, &occ)
    {
        world.actors[idx].path = p.into();
        world.actors[idx].task = PendingTask::RestAt { tx: sx, ty: sy };
        world.actors[idx].activity_tag = "入屋避雨".into();
        return true;
    }

    false
}

pub fn complete_sleep_bed(world: &mut RimWorldState, idx: usize, tx: i32, ty: i32) {
    let (ax, ay) = world.actors[idx].micro_pos();
    if (ax - tx).abs() <= 1
        && (ay - ty).abs() <= 1
        && world.overlays.get(&(tx, ty)).and_then(|o| o.structure) == Some(StructureKind::BambooBed)
    {
        world.actors[idx].px = tx as f32;
        world.actors[idx].py = ty as f32;
        world.actors[idx].facing_rad = std::f32::consts::PI;
        world.actors[idx].activity_tag = "榻上安歇".into();
        world.actors[idx].pop_text = Some(("上榻安歇".into(), world.tick + 8));
        world.actors[idx].push_diary("登竹編睡榻安歇調氣");
    } else {
        world.actors[idx].activity_tag = "駐足待命".into();
    }
    world.actors[idx].task = PendingTask::None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::rim_seed::seed_outpost;
    use crate::grid::substrate::rim_sim::RimActor;

    #[test]
    fn enclosed_rooms_detect_west_cabin_and_shuoweiliao_and_ignore_broken_walls() {
        let (mut overlays, _, _) = seed_outpost(None);
        let rooms = detect_enclosed_rooms(&overlays);
        assert!(rooms.iter().any(|r| r.x0 == 7 && r.x1 == 11 && r.y0 == 1 && r.y1 == 5));
        assert!(rooms.iter().any(|r| r.x0 == 28 && r.x1 == 36 && r.y0 == -10 && r.y1 == -4));

        // 拆掉北牆一塊後，該屋不應再判定為閉合室內
        overlays.remove(&(9, 5));
        let rooms_after = detect_enclosed_rooms(&overlays);
        assert!(!rooms_after.iter().any(|r| r.x0 == 7 && r.x1 == 11));
    }

    #[test]
    fn rain_wets_hatless_outdoor_settlers_while_hat_and_roof_shield_them() {
        let (overlays, mut actors, _) = seed_outpost(None);
        let mut world = RimWorldState {
            tick: 24,
            overlays,
            actors: Vec::new(),
            events: VecDeque::new(),
            economy: Default::default(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };
        let mut a_no_hat = actors.remove(0);
        a_no_hat.px = 2.0;
        a_no_hat.py = 0.0;
        a_no_hat.equipped_hat = false;
        a_no_hat.wet_ticks = 0;

        let mut a_indoor = actors.remove(0);
        a_indoor.px = 8.0;
        a_indoor.py = 3.0;
        a_indoor.equipped_hat = false;
        a_indoor.wet_ticks = 20;

        world.actors = vec![a_no_hat, a_indoor];
        step_weather_and_bed_recovery(&mut world, RimWeatherKind::SummerSquall);
        assert!(world.actors[0].wet_ticks > 0);
        assert!(world.actors[1].wet_ticks < 20);
    }

    #[test]
    fn bamboo_bed_heals_injury_and_restores_stamina_faster() {
        let (overlays, mut actors, _) = seed_outpost(None);
        let mut bed_actor: RimActor = actors.remove(0);
        bed_actor.px = 8.0;
        bed_actor.py = 4.0;
        bed_actor.stamina = 40;
        bed_actor.injury_ticks = 4;
        bed_actor.injury_part = Some("左肩扭挫".into());
        let mut world = RimWorldState {
            tick: 24,
            overlays,
            actors: vec![bed_actor],
            events: VecDeque::new(),
            economy: Default::default(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };
        step_weather_and_bed_recovery(&mut world, RimWeatherKind::SummerSquall);
        assert!(world.actors[0].stamina > 50);
        assert_eq!(world.actors[0].injury_ticks, 0);
        assert!(world.actors[0].injury_part.is_none());
    }

    #[test]
    fn hatless_settler_seeks_indoor_shelter_or_bed_when_raining() {
        let (overlays, mut actors, _) = seed_outpost(None);
        let mut a = actors.remove(0);
        a.px = 5.0;
        a.py = 3.0;
        a.equipped_hat = false;
        a.wet_ticks = 18;
        a.path.clear();
        a.task = PendingTask::None;
        let mut world = RimWorldState {
            tick: 25,
            overlays,
            actors: vec![a],
            events: VecDeque::new(),
            economy: Default::default(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };
        let assigned = try_seek_bed_or_rain_shelter(&mut world, None, 0, RimWeatherKind::SummerSquall);
        assert!(assigned);
        assert!(!world.actors[0].path.is_empty());
    }
}

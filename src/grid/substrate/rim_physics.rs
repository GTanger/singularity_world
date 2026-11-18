//! 實心圓體碰撞（PBD 零重疊位置投影）、動量加權推擠、營火圍坐擇位與窄道禮讓引擎。

use std::collections::HashMap;

use super::rim_sim::{ActorFaction, PendingTask, RimActor, RimWorldState};
use super::rim_world::{
    MicroCellOverlay, find_micro_path_crowded, is_circle_walkable, micro_step_cost,
};
use super::runtime::SubstrateRuntime;

/// 人類角色間最小實體中心距（公尺，與前端白描肩膀 0.69m／竹笠 0.72m 吻合且留細微縫隙）。
pub const HUMAN_MIN_SEP: f32 = 0.82;
/// 含山海異獸時之最小實體中心距（保持小於近身拆招半徑 1.45m，兼顧實體阻擋與接戰）。
pub const BEAST_MIN_SEP: f32 = 0.90;
/// 角色重心對牆體、營火、工作台與水岸之防嵌入半徑（兼顧貼邊免穿透與 1.0m 單格門／窄橋順暢滑行）。
pub const WALL_CLEAR_R: f32 = 0.15;

fn actor_inv_mass(a: &RimActor) -> f32 {
    if a.running && !a.path.is_empty() {
        0.15
    } else if a.is_player && !a.autopilot && !a.path.is_empty() {
        0.32
    } else if !a.path.is_empty() {
        0.45
    } else {
        0.78
    }
}

fn can_occupy(
    rt: Option<&SubstrateRuntime>,
    overlays: &HashMap<(i32, i32), MicroCellOverlay>,
    old_x: f32,
    old_y: f32,
    new_x: f32,
    new_y: f32,
    is_beast: bool,
) -> bool {
    let (ox, oy) = (old_x.round() as i32, old_y.round() as i32);
    let (nx, ny) = (new_x.round() as i32, new_y.round() as i32);
    if micro_step_cost(rt, overlays, nx, ny, is_beast).is_none() {
        return false;
    }
    if ox != nx
        && oy != ny
        && (micro_step_cost(rt, overlays, nx, oy, is_beast).is_none()
            || micro_step_cost(rt, overlays, ox, ny, is_beast).is_none())
    {
        return false;
    }
    if is_circle_walkable(rt, overlays, new_x, new_y, WALL_CLEAR_R, is_beast) {
        return true;
    }
    if !is_circle_walkable(rt, overlays, old_x, old_y, WALL_CLEAR_R, is_beast) {
        let old_d = (old_x - ox as f32).hypot(old_y - oy as f32);
        let new_d = (new_x - nx as f32).hypot(new_y - ny as f32);
        return new_d < old_d;
    }
    false
}

fn slide_move(
    rt: Option<&SubstrateRuntime>,
    overlays: &HashMap<(i32, i32), MicroCellOverlay>,
    px: f32,
    py: f32,
    dx: f32,
    dy: f32,
    is_beast: bool,
) -> (f32, f32) {
    if dx.abs() < 1e-6 && dy.abs() < 1e-6 {
        return (px, py);
    }
    if can_occupy(rt, overlays, px, py, px + dx, py + dy, is_beast) {
        return (px + dx, py + dy);
    }
    if dx.abs() > 1e-5 && can_occupy(rt, overlays, px, py, px + dx, py, is_beast) {
        return (px + dx, py);
    }
    if dy.abs() > 1e-5 && can_occupy(rt, overlays, px, py, px, py + dy, is_beast) {
        return (px, py + dy);
    }
    (px, py)
}

impl RimWorldState {
    pub(crate) fn occupied_cells(&self, exclude_idx: Option<usize>) -> Vec<(i32, i32)> {
        self.actors
            .iter()
            .enumerate()
            .filter(|(i, a)| Some(*i) != exclude_idx && a.hp > 0)
            .map(|(_, a)| a.micro_pos())
            .collect()
    }

    /// 挑選營火／工作台旁尚未被人佔坐之鄰近空地，避免所有人擠進同一火堆格。
    pub(crate) fn pick_campfire_seat(
        &self,
        rt: Option<&SubstrateRuntime>,
        fx: i32,
        fy: i32,
        actor_id: usize,
    ) -> (i32, i32) {
        const RING: [(i32, i32); 12] = [
            (0, -1),
            (1, 0),
            (0, 1),
            (-1, 0),
            (1, -1),
            (-1, 1),
            (-1, -1),
            (1, 1),
            (0, -2),
            (2, 0),
            (0, 2),
            (-2, 0),
        ];
        let rot = actor_id % RING.len();
        let mut fallback = None;
        for step in 0..RING.len() {
            let (ox, oy) = RING[(rot + step) % RING.len()];
            let cand = (fx + ox, fy + oy);
            if micro_step_cost(rt, &self.overlays, cand.0, cand.1, false).is_none() {
                continue;
            }
            if fallback.is_none() {
                fallback = Some(cand);
            }
            let free = self.actors.iter().all(|a| {
                a.id == actor_id
                    || a.hp <= 0
                    || (a.px - cand.0 as f32).hypot(a.py - cand.1 as f32) >= 0.85
            });
            if free {
                return cand;
            }
        }
        fallback.unwrap_or((fx, fy))
    }

    fn unstick_actor_from_solid(&mut self, rt: Option<&SubstrateRuntime>, idx: usize) {
        let is_beast = self.actors[idx].faction == ActorFaction::WildBeast;
        let ax = self.actors[idx].px.round() as i32;
        let ay = self.actors[idx].py.round() as i32;
        if micro_step_cost(rt, &self.overlays, ax, ay, is_beast).is_some() {
            return;
        }
        let my_id = self.actors[idx].id;
        let mut fallback = None;
        for r in 1..=2_i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let cand = (ax + dx, ay + dy);
                    if micro_step_cost(rt, &self.overlays, cand.0, cand.1, is_beast).is_none() {
                        continue;
                    }
                    if fallback.is_none() {
                        fallback = Some(cand);
                    }
                    let free = self.actors.iter().all(|o| {
                        o.id == my_id
                            || o.hp <= 0
                            || (o.px - cand.0 as f32).hypot(o.py - cand.1 as f32) >= HUMAN_MIN_SEP
                    });
                    if free {
                        self.actors[idx].px = cand.0 as f32;
                        self.actors[idx].py = cand.1 as f32;
                        return;
                    }
                }
            }
        }
        if let Some((cx, cy)) = fallback {
            self.actors[idx].px = cx as f32;
            self.actors[idx].py = cy as f32;
        }
    }

    /// 20Hz（每 50ms）等速連續折線滑行，並具備人潮預判右偏轉向與牆緣沿切滑動。
    /// 1 細格 = 1.0 公尺：常態步行 1.35 m/s、雙擊疾奔 3.80 m/s、野生山海獸 1.60 m/s。
    pub(crate) fn glide_actor_50ms(&mut self, rt: Option<&SubstrateRuntime>, idx: usize) {
        self.unstick_actor_from_solid(rt, idx);
        if self.actors[idx].downed_ticks > 0 {
            return;
        }
        let is_beast = self.actors[idx].faction == ActorFaction::WildBeast;
        let base_speed = if is_beast {
            1.60_f32
        } else if self.actors[idx].running {
            3.80_f32
        } else {
            1.35_f32
        };
        let speed_per_sec = if self.actors[idx].injury_ticks > 0 {
            base_speed * 0.86
        } else {
            base_speed
        };
        let mut budget = speed_per_sec * 0.05;

        while budget > 0.0005 {
            let Some(&(nx, ny)) = self.actors[idx].path.front() else {
                self.actors[idx].running = false;
                break;
            };
            if micro_step_cost(rt, &self.overlays, nx, ny, is_beast).is_none() {
                self.actors[idx].path.clear();
                self.actors[idx].running = false;
                break;
            }
            let px = self.actors[idx].px;
            let py = self.actors[idx].py;
            let dx = nx as f32 - px;
            let dy = ny as f32 - py;
            let dist = dx.hypot(dy);
            let crowded = self.actors.iter().enumerate().any(|(j, other)| {
                j != idx
                    && other.hp > 0
                    && (other.px - nx as f32).hypot(other.py - ny as f32) < 0.86
            });
            if crowded {
                if self.actors[idx].path.len() > 1 && dist <= 0.78 {
                    self.actors[idx].path.pop_front();
                    continue;
                }
                if self.actors[idx].path.len() == 1 && dist <= 0.98 {
                    self.actors[idx].path.pop_front();
                    self.actors[idx].running = false;
                    break;
                }
            }
            if dist > 0.005 {
                self.actors[idx].facing_rad = dy.atan2(dx);
            }
            let (mut ux, mut uy) = if dist > 1e-4 {
                (dx / dist, dy / dist)
            } else {
                self.actors[idx].path.pop_front();
                continue;
            };
            let steer_threat = self.actors.iter().enumerate().any(|(j, other)| {
                if j == idx || other.hp <= 0 {
                    return false;
                }
                let ox = other.px - px;
                let oy = other.py - py;
                let fwd = ox * ux + oy * uy;
                let lat = (-ox * uy + oy * ux).abs();
                fwd > 0.05 && fwd < 1.05 && lat < 0.76
            });
            if !crowded
                && !steer_threat
                && dist <= budget
                && can_occupy(rt, &self.overlays, px, py, nx as f32, ny as f32, is_beast)
            {
                self.actors[idx].px = nx as f32;
                self.actors[idx].py = ny as f32;
                self.actors[idx].path.pop_front();
                budget -= dist;
                continue;
            }
            if steer_threat {
                let sx = ux - uy * 0.32;
                let sy = uy + ux * 0.32;
                let sl = sx.hypot(sy);
                if sl > 1e-4 {
                    ux = sx / sl;
                    uy = sy / sl;
                }
            }
            let step = dist.min(budget);
            let (new_x, new_y) =
                slide_move(rt, &self.overlays, px, py, ux * step, uy * step, is_beast);
            self.actors[idx].px = new_x;
            self.actors[idx].py = new_y;
            break;
        }
    }

    /// 多輪實心圓體位置投影（PBD）：同服所有活體嚴格零重疊、依行止輕重推擠撞開，並守死牆水界。
    pub(crate) fn resolve_solid_collisions(
        &mut self,
        rt: Option<&SubstrateRuntime>,
        iterations: usize,
    ) {
        let len = self.actors.len();
        for idx in 0..len {
            if self.actors[idx].hp > 0 {
                self.unstick_actor_from_solid(rt, idx);
            }
        }
        for _iter in 0..iterations {
            for i in 0..len {
                for j in (i + 1)..len {
                    if self.actors[i].hp <= 0 || self.actors[j].hp <= 0 {
                        continue;
                    }
                    let is_b_i = self.actors[i].faction == ActorFaction::WildBeast;
                    let is_b_j = self.actors[j].faction == ActorFaction::WildBeast;
                    let min_sep = if is_b_i || is_b_j {
                        BEAST_MIN_SEP
                    } else {
                        HUMAN_MIN_SEP
                    };
                    let dx = self.actors[j].px - self.actors[i].px;
                    let dy = self.actors[j].py - self.actors[i].py;
                    let dist = dx.hypot(dy);
                    if dist >= min_sep {
                        continue;
                    }
                    let (nx, ny) = if dist > 0.001 {
                        (dx / dist, dy / dist)
                    } else {
                        let a = (i * 7 + j * 13) as f32;
                        (a.cos(), a.sin())
                    };
                    let overlap = min_sep - dist + 0.002;
                    let inv_i = actor_inv_mass(&self.actors[i]);
                    let inv_j = actor_inv_mass(&self.actors[j]);
                    let inv_sum = (inv_i + inv_j).max(0.001);
                    let wi = inv_i / inv_sum;
                    let wj = inv_j / inv_sum;

                    let (ix, iy) = slide_move(
                        rt,
                        &self.overlays,
                        self.actors[i].px,
                        self.actors[i].py,
                        -nx * overlap * wi,
                        -ny * overlap * wi,
                        is_b_i,
                    );
                    self.actors[i].px = ix;
                    self.actors[i].py = iy;

                    let (jx, jy) = slide_move(
                        rt,
                        &self.overlays,
                        self.actors[j].px,
                        self.actors[j].py,
                        nx * overlap * wj,
                        ny * overlap * wj,
                        is_b_j,
                    );
                    self.actors[j].px = jx;
                    self.actors[j].py = jy;

                    // 靠牆／窄道保底投影：若單軸頂牆造成剩餘重疊，於另一自由軸精確補齊所需弦距
                    let dx2 = self.actors[j].px - self.actors[i].px;
                    let dy2 = self.actors[j].py - self.actors[i].py;
                    let dist2 = dx2.hypot(dy2);
                    if dist2 < min_sep {
                        let (nx2, ny2) = if dist2 > 0.001 {
                            (dx2 / dist2, dy2 / dist2)
                        } else {
                            (nx, ny)
                        };
                        let rem = min_sep - dist2 + 0.001;
                        let (fx_i, fy_i) = slide_move(
                            rt,
                            &self.overlays,
                            self.actors[i].px,
                            self.actors[i].py,
                            -nx2 * rem,
                            -ny2 * rem,
                            is_b_i,
                        );
                        self.actors[i].px = fx_i;
                        self.actors[i].py = fy_i;

                        let dx3 = self.actors[j].px - self.actors[i].px;
                        let dy3 = self.actors[j].py - self.actors[i].py;
                        let dist3 = dx3.hypot(dy3);
                        if dist3 < min_sep {
                            let (nx3, ny3) = if dist3 > 0.001 {
                                (dx3 / dist3, dy3 / dist3)
                            } else {
                                (nx2, ny2)
                            };
                            let rem3 = min_sep - dist3 + 0.001;
                            let (fx_j, fy_j) = slide_move(
                                rt,
                                &self.overlays,
                                self.actors[j].px,
                                self.actors[j].py,
                                nx3 * rem3,
                                ny3 * rem3,
                                is_b_j,
                            );
                            self.actors[j].px = fx_j;
                            self.actors[j].py = fy_j;
                        }

                        let dx4 = self.actors[j].px - self.actors[i].px;
                        let dy4 = self.actors[j].py - self.actors[i].py;
                        if dx4.hypot(dy4) < min_sep && dx4.abs() < min_sep {
                            let req_y = (min_sep * min_sep - dx4 * dx4).max(0.0).sqrt() + 0.002;
                            let need_y = (req_y - dy4.abs()).max(0.0);
                            if need_y > 0.0 {
                                let sy = if dy4 >= 0.0 { 1.0 } else { -1.0 };
                                let (ax, ay) = slide_move(
                                    rt,
                                    &self.overlays,
                                    self.actors[i].px,
                                    self.actors[i].py,
                                    0.0,
                                    -sy * need_y,
                                    is_b_i,
                                );
                                self.actors[i].px = ax;
                                self.actors[i].py = ay;
                                let rem_dy = (self.actors[j].py - self.actors[i].py).abs();
                                let left_y = (req_y - rem_dy).max(0.0);
                                if left_y > 0.0 {
                                    let (bx, by) = slide_move(
                                        rt,
                                        &self.overlays,
                                        self.actors[j].px,
                                        self.actors[j].py,
                                        0.0,
                                        sy * left_y,
                                        is_b_j,
                                    );
                                    self.actors[j].px = bx;
                                    self.actors[j].py = by;
                                }
                            }
                        }

                        let dx5 = self.actors[j].px - self.actors[i].px;
                        let dy5 = self.actors[j].py - self.actors[i].py;
                        if dx5.hypot(dy5) < min_sep && dy5.abs() < min_sep {
                            let req_x = (min_sep * min_sep - dy5 * dy5).max(0.0).sqrt() + 0.002;
                            let need_x = (req_x - dx5.abs()).max(0.0);
                            if need_x > 0.0 {
                                let sx = if dx5 >= 0.0 { 1.0 } else { -1.0 };
                                let (ax, ay) = slide_move(
                                    rt,
                                    &self.overlays,
                                    self.actors[i].px,
                                    self.actors[i].py,
                                    -sx * need_x,
                                    0.0,
                                    is_b_i,
                                );
                                self.actors[i].px = ax;
                                self.actors[i].py = ay;
                                let rem_dx = (self.actors[j].px - self.actors[i].px).abs();
                                let left_x = (req_x - rem_dx).max(0.0);
                                if left_x > 0.0 {
                                    let (bx, by) = slide_move(
                                        rt,
                                        &self.overlays,
                                        self.actors[j].px,
                                        self.actors[j].py,
                                        sx * left_x,
                                        0.0,
                                        is_b_j,
                                    );
                                    self.actors[j].px = bx;
                                    self.actors[j].py = by;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// 窄道或門洞對峙卡住 1 秒時，自動退開至最近開闊避讓格（「側身禮讓」）讓對方先過。
    pub(crate) fn try_yield_choke_point(&mut self, rt: Option<&SubstrateRuntime>, idx: usize) {
        self.actors[idx].path.clear();
        self.actors[idx].running = false;
        if self.actors[idx].autopilot {
            self.actors[idx].task = PendingTask::None;
        }
        let start = self.actors[idx].micro_pos();
        let is_beast = self.actors[idx].faction == ActorFaction::WildBeast;
        let occ = self.occupied_cells(Some(idx));
        let mut cands: Vec<(i32, (i32, i32))> = Vec::new();
        for dy in -4..=4_i32 {
            for dx in -4..=4_i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let tx = start.0 + dx;
                let ty = start.1 + dy;
                if micro_step_cost(rt, &self.overlays, tx, ty, is_beast).is_none() {
                    continue;
                }
                let orth_open = [(0, 1), (1, 0), (0, -1), (-1, 0)]
                    .into_iter()
                    .filter(|(ox, oy)| {
                        micro_step_cost(rt, &self.overlays, tx + ox, ty + oy, is_beast).is_some()
                    })
                    .count();
                if orth_open < 3 {
                    continue;
                }
                let free = self.actors.iter().enumerate().all(|(j, other)| {
                    j == idx
                        || other.hp <= 0
                        || (other.px - tx as f32).hypot(other.py - ty as f32) >= 0.95
                });
                if free {
                    cands.push((dx.abs() + dy.abs(), (tx, ty)));
                }
            }
        }
        cands.sort_by_key(|&(d, _)| d);
        for (_, tgt) in cands {
            if let Some(p) =
                find_micro_path_crowded(rt, &self.overlays, start, tgt, false, is_beast, &occ)
            {
                self.actors[idx].path = p.into();
                self.actors[idx].activity_tag = "側身禮讓".into();
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::rim_world::StructureKind;
    use std::collections::VecDeque;

    fn test_settler(id: usize, px: f32, py: f32, running: bool) -> RimActor {
        RimActor {
            id,
            code: format!("t_{id}"),
            actor_key: format!("t_{id}"),
            entity_id: None,
            glyph: "測".into(),
            name: format!("測試{id}"),
            gender: "M".into(),
            soul_seed: id as i64,
            role_tag: "測試".into(),
            faction: ActorFaction::Settler,
            is_player: false,
            autopilot: false,
            running,
            px,
            py,
            facing_rad: 0.0,
            hp: 100,
            max_hp: 100,
            equipped_tool: None,
            carrying: None,
            inventory: Vec::new(),
            directive: String::new(),
            path: VecDeque::new(),
            task: PendingTask::None,
            activity_tag: "駐足".into(),
            pop_text: None,
            cooldown_ticks: 0,
            last_active_unix: 0,
            hunger: 90,
            thirst: 90,
            stamina: 90,
            magnesium: 10,
            equipped_hat: false,
            equipped_pack: false,
            known_by: Vec::new(),
            practice_unarmed: 0,
            practice_blade: 0,
            practice_blunt: 0,
            craft_harvest: 0,
            craft_forge: 0,
            craft_culinary: 0,
            craft_wayfare: 0,
            anatomy_winged: 0,
            anatomy_hoofed: 0,
            injury_part: None,
            injury_ticks: 0,
            downed_ticks: 0,
            caravan_leg: 0,
            wet_ticks: 0,
            recent_diary: VecDeque::new(),
        }
    }

    #[test]
    fn solid_collision_guarantees_zero_overlap_under_sprint_and_crowd() {
        let mut overlays = HashMap::new();
        for y in -3..=3 {
            for x in -4..=4 {
                overlays.insert(
                    (x, y),
                    MicroCellOverlay {
                        cleared_natural: true,
                        structure: (x == 3).then_some(StructureKind::BambooWall),
                        ..Default::default()
                    },
                );
            }
        }
        let mut charger = test_settler(1, -2.0, 0.0, true);
        charger.is_player = true;
        charger.path = VecDeque::from([(-1, 0), (0, 0), (1, 0), (2, 0)]);

        let a2 = test_settler(2, 2.0, 0.0, false);
        let a3 = test_settler(3, 1.2, 0.0, false);
        let a4 = test_settler(4, 1.2, 0.7, false);
        let a5 = test_settler(5, 1.2, -0.7, false);

        let mut world = RimWorldState {
            tick: 1,
            overlays,
            actors: vec![charger, a2, a3, a4, a5],
            events: VecDeque::new(),
            economy: Default::default(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };
        world.resolve_solid_collisions(None, 6);
        let pinned_before = world.actors[1].px;

        for _ in 0..80 {
            world.step_physics_50ms();
            for i in 0..world.actors.len() {
                for j in (i + 1)..world.actors.len() {
                    let d = (world.actors[j].px - world.actors[i].px)
                        .hypot(world.actors[j].py - world.actors[i].py);
                    assert!(
                        d >= 0.81,
                        "actors {i} and {j} overlapped: dist={d:.3} at tick={}",
                        world.tick
                    );
                }
            }
        }
        let pushed_someone = world.actors[2].px != 1.2
            || world.actors[2].py != 0.0
            || world.actors[1].px != pinned_before;
        assert!(pushed_someone, "expected sprinting actor to jostle crowd");
    }

    #[test]
    fn narrow_bridge_head_on_walkers_yield_and_cross_without_overlap() {
        let mut overlays = HashMap::new();
        for y in 7..=16 {
            for x in -2..=2 {
                if (10..=13).contains(&y) && x != 0 {
                    continue;
                }
                overlays.insert(
                    (x, y),
                    MicroCellOverlay {
                        cleared_natural: true,
                        structure: (x == 0 && (9..=14).contains(&y))
                            .then_some(StructureKind::WoodBridge),
                        ..Default::default()
                    },
                );
            }
        }
        let mut south = test_settler(1, 0.0, 9.0, false);
        south.path = VecDeque::from([(0, 10), (0, 11), (0, 12), (0, 13), (0, 14), (0, 15)]);
        let mut north = test_settler(2, 0.0, 14.0, false);
        north.path = VecDeque::from([(0, 13), (0, 12), (0, 11), (0, 10), (0, 9), (0, 8)]);

        let mut world = RimWorldState {
            tick: 1,
            overlays,
            actors: vec![south, north],
            events: VecDeque::new(),
            economy: Default::default(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };
        for _ in 0..280 {
            world.step_physics_50ms();
            let d = (world.actors[1].px - world.actors[0].px)
                .hypot(world.actors[1].py - world.actors[0].py);
            assert!(d >= 0.81, "bridge overlap dist={d:.3}");
        }
        assert!(
            world.actors[0].py >= 14.0 && world.actors[1].py <= 9.0,
            "expected both walkers to cross 1m bridge cleanly: a=({:.2},{:.2},len={}) b=({:.2},{:.2},len={})",
            world.actors[0].px,
            world.actors[0].py,
            world.actors[0].path.len(),
            world.actors[1].px,
            world.actors[1].py,
            world.actors[1].path.len()
        );
    }
}

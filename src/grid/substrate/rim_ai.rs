//! 輕量版 RimWorld 同伴／異獸自主工單調度、連續位移與圓體碰撞／田忌賽馬拆招引擎。

use super::rim_sim::{ActorFaction, PendingTask, RimWorldState};
use super::rim_world::{
    BlueprintJob, GroundStack, NaturalProp, RimItemKind, StructureKind, derive_micro_pristine,
    find_micro_path, find_micro_path_crowded,
};
use super::runtime::SubstrateRuntime;

impl RimWorldState {
    pub(crate) fn assign_autonomous_task(&mut self, rt: Option<&SubstrateRuntime>, idx: usize) {
        let start = self.actors[idx].micro_pos();
        if self.actors[idx].faction == ActorFaction::WildBeast {
            let h = self
                .tick
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(self.actors[idx].id as u64 * 97);
            if start.0 < -28 || start.0 > 42 || start.1 < -20 || start.1 > 28 {
                let goal = if self.actors[idx].id.is_multiple_of(2) {
                    (6, 18)
                } else {
                    (-14, 5)
                };
                if let Some(p) = find_micro_path(rt, &self.overlays, start, goal, false, true) {
                    self.actors[idx].path = p.into();
                    self.actors[idx].activity_tag = "林間巡棲".into();
                } else {
                    self.actors[idx].px = goal.0 as f32;
                    self.actors[idx].py = goal.1 as f32;
                }
            } else if h.is_multiple_of(7) {
                let dx = ((h >> 8) % 7) as i32 - 3;
                let dy = ((h >> 16) % 7) as i32 - 3;
                let goal = (start.0 + dx, start.1 + dy);
                if let Some(p) = find_micro_path(rt, &self.overlays, start, goal, false, true) {
                    self.actors[idx].path = p.into();
                    self.actors[idx].activity_tag = "林間覓食".into();
                }
            }
            return;
        }

        if self.actors[idx].downed_ticks > 0 {
            return;
        }

        // 健康聚落成員優先趕赴扶助附近因交鋒力竭倒地的同伴
        if super::rim_life::try_rescue_downed_ally(self, rt, idx) {
            return;
        }

        let home_pt = if start.0 >= 22 { (32, -7) } else { (0, 0) };
        if super::rim_economy::try_guard_or_market_livelihood(self, rt, idx, start, home_pt) {
            return;
        }

        // 若同伴被託付為『follow（跟隨同行）』，貼近最近在線玩家行動
        if self.actors[idx].entity_id.is_none()
            && self.actors[idx].directive == "follow"
            && let Some(p) = self
                .actors
                .iter()
                .filter(|a| a.entity_id.is_some() && a.hp > 0 && a.downed_ticks == 0)
                .max_by_key(|a| a.last_active_unix)
        {
            let p_pos = p.micro_pos();
            if (p_pos.0 - start.0).abs() + (p_pos.1 - start.1).abs() > 3
                && let Some(path) = find_micro_path(rt, &self.overlays, start, p_pos, true, false)
            {
                self.actors[idx].path = path.into();
                self.actors[idx].activity_tag = "跟隨同行".into();
            } else {
                self.actors[idx].activity_tag = "隨行護衛".into();
            }
            return;
        }

        // 優先照顧飲水、炊食與歇息等切身生理需求（同服平權：NPC 與暫離玩家共用）
        if super::rim_life::try_satisfy_urgent_needs(self, rt, idx) {
            return;
        }

        // 負傷時優先敷用或前往案台搗製敷傷藥膏消解部位傷勢
        if super::rim_ecology::try_urgent_salve_heal(self, rt, idx) {
            return;
        }

        // 傷疲靜養或山雨無笠時，優先前往空閒竹編睡榻或入屋避雨
        let weather = super::rim_shelter::weather_at_unix(super::rim_sim::now_unix_sec());
        if super::rim_shelter::try_seek_bed_or_rain_shelter(self, rt, idx, weather) {
            return;
        }

        let (epoch, scale) = crate::game::game_time_cfg();
        let (_, game_hour, _, _) =
            crate::game::game_time_now(super::rim_sim::now_unix_sec(), epoch, scale);
        if super::rim_town::try_town_night_sleep(self, rt, idx, game_hour as i32) {
            return;
        }

        // 若駐世方針為「rest（爐畔靜養）」，回就近營火調息不再遠出勞作
        if self.actors[idx].directive == "rest" {
            let occ = self.occupied_cells(Some(idx));
            if let Some(&(fx, fy)) = self
                .overlays
                .iter()
                .filter(|(_, ov)| ov.structure == Some(StructureKind::Campfire))
                .min_by_key(|&(&(cx, cy), _)| (cx - start.0).abs() + (cy - start.1).abs())
                .map(|(pos, _)| pos)
            {
                if (fx - start.0).abs().max((fy - start.1).abs()) <= 2 {
                    self.actors[idx].activity_tag = "爐畔靜養".into();
                    return;
                }
                let seat = self.pick_campfire_seat(rt, fx, fy, self.actors[idx].id);
                if let Some(p) =
                    find_micro_path_crowded(rt, &self.overlays, start, seat, false, false, &occ)
                {
                    self.actors[idx].path = p.into();
                    self.actors[idx].task = PendingTask::RestAt { tx: fx, ty: fy };
                    self.actors[idx].activity_tag = "歸爐靜養".into();
                    return;
                }
            }
            return;
        }

        let home = home_pt;
        if (start.0 - home.0).abs().max((start.1 - home.1).abs()) > 42 {
            let step_x = start.0 + (home.0 - start.0).clamp(-20, 20);
            let step_y = start.1 + (home.1 - start.1).clamp(-20, 20);
            if let Some(p) =
                find_micro_path(rt, &self.overlays, start, (step_x, step_y), true, false)
            {
                self.actors[idx].path = p.into();
                self.actors[idx].activity_tag = "循徑歸營".into();
                return;
            }
        }

        let occ = self.occupied_cells(Some(idx));

        // 開譜曆夜間（19:00～04:59）：無急務之自主居民回到就近營火旁空席圍爐歇息
        let game_hour = crate::game::current_game_hour();
        let is_night = !(5..19).contains(&game_hour);
        if is_night && self.actors[idx].entity_id.is_none() {
            if let Some(&(fx, fy)) = self
                .overlays
                .iter()
                .filter(|(_, ov)| ov.structure == Some(StructureKind::Campfire))
                .min_by_key(|&(&(cx, cy), _)| (cx - start.0).abs() + (cy - start.1).abs())
                .map(|(pos, _)| pos)
            {
                let dist = (fx - start.0).abs().max((fy - start.1).abs());
                if dist <= 2 {
                    self.actors[idx].facing_rad =
                        ((fy - start.1) as f32).atan2((fx - start.0) as f32);
                    self.actors[idx].activity_tag = "圍爐歇息".into();
                    return;
                }
                let seat = self.pick_campfire_seat(rt, fx, fy, self.actors[idx].id);
                if let Some(p) =
                    find_micro_path_crowded(rt, &self.overlays, start, seat, false, false, &occ)
                {
                    self.actors[idx].path = p.into();
                    self.actors[idx].task = PendingTask::RestAt { tx: fx, ty: fy };
                    self.actors[idx].activity_tag = "歸營烤火".into();
                    return;
                }
            }
        }

        // 雙聚落行腳貨郎自主挑貨往返水尾寮與西麓湧泉
        if !is_night && super::rim_phase::try_assign_caravan_route(self, rt, idx) {
            return;
        }

        // 若同伴為『patrol（自由巡護）』，於各自聚落附近巡守不遠走
        if self.actors[idx].entity_id.is_none() && self.actors[idx].directive == "patrol" {
            if self.tick % 13 == (self.actors[idx].id as u64 % 13) {
                let angle = (self.tick as f32 * 0.55) + self.actors[idx].id as f32;
                let gx = home.0 + (angle.cos() * 4.5).round() as i32;
                let gy = home.1 + (angle.sin() * 3.5).round() as i32;
                if let Some(p) =
                    find_micro_path_crowded(rt, &self.overlays, start, (gx, gy), false, false, &occ)
                {
                    self.actors[idx].path = p.into();
                    self.actors[idx].activity_tag = "四處巡走".into();
                }
            }
            return;
        }

        let claimed: Vec<(i32, i32)> = self
            .actors
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != idx)
            .filter_map(|(_, a)| match a.task {
                PendingTask::ChopAt { tx, ty }
                | PendingTask::WorkBuild { tx, ty }
                | PendingTask::DeconstructAt { tx, ty }
                | PendingTask::PickupEquip { tx, ty }
                | PendingTask::StorehouseTrade { tx, ty }
                | PendingTask::FarmPlot { tx, ty, .. } => Some((tx, ty)),
                PendingTask::FetchForBuild { dst_x, dst_y, .. }
                | PendingTask::HaulToStockpile {
                    src_x: dst_x,
                    src_y: dst_y,
                    ..
                } => Some((dst_x, dst_y)),
                _ => None,
            })
            .collect();

        // 優先級 A：建造藍圖（取竹材 ➔ 築造）
        let mut build_targets: Vec<((i32, i32), StructureKind, u8)> = self
            .overlays
            .iter()
            .filter_map(|(&pos, ov)| match ov.blueprint {
                Some(BlueprintJob::Build {
                    structure,
                    delivered_bamboo,
                    ..
                }) if !claimed.contains(&pos) => Some((pos, structure, delivered_bamboo)),
                _ => None,
            })
            .collect();
        build_targets.sort_by_key(|(pos, _, _)| (pos.0 - start.0).abs() + (pos.1 - start.1).abs());

        for ((bx, by), st, delivered) in build_targets {
            let need = st.bamboo_cost().saturating_sub(delivered);
            if need == 0
                || self.actors[idx]
                    .carrying
                    .is_some_and(|c| c.item == RimItemKind::BambooPole && c.count > 0)
            {
                if let Some(p) =
                    find_micro_path_crowded(rt, &self.overlays, start, (bx, by), true, false, &occ)
                {
                    self.actors[idx].path = p.into();
                    self.actors[idx].task = PendingTask::WorkBuild { tx: bx, ty: by };
                    self.actors[idx].activity_tag = format!("築造{}", st.label());
                    return;
                }
            } else if let Some((sx, sy)) =
                self.find_nearest_ground_item(start, RimItemKind::BambooPole, false)
                && let Some(p) =
                    find_micro_path_crowded(rt, &self.overlays, start, (sx, sy), true, false, &occ)
            {
                self.actors[idx].path = p.into();
                self.actors[idx].task = PendingTask::FetchForBuild {
                    src_x: sx,
                    src_y: sy,
                    dst_x: bx,
                    dst_y: by,
                };
                self.actors[idx].activity_tag = "取竹赴工".into();
                return;
            }
        }

        // 優先級 B：拆除工單
        if let Some((&(dx, dy), _)) = self
            .overlays
            .iter()
            .filter(|(pos, ov)| {
                matches!(ov.blueprint, Some(BlueprintJob::Deconstruct { .. }))
                    && !claimed.contains(pos)
            })
            .min_by_key(|(pos, _)| (pos.0 - start.0).abs() + (pos.1 - start.1).abs())
            && let Some(p) =
                find_micro_path_crowded(rt, &self.overlays, start, (dx, dy), true, false, &occ)
        {
            self.actors[idx].path = p.into();
            self.actors[idx].task = PendingTask::DeconstructAt { tx: dx, ty: dy };
            self.actors[idx].activity_tag = "拆除舊構".into();
            return;
        }

        // 優先級 C：砍伐採集工單
        if let Some((&(cx, cy), _)) = self
            .overlays
            .iter()
            .filter(|(pos, ov)| {
                matches!(ov.blueprint, Some(BlueprintJob::Chop { .. })) && !claimed.contains(pos)
            })
            .min_by_key(|(pos, _)| (pos.0 - start.0).abs() + (pos.1 - start.1).abs())
            && let Some(p) =
                find_micro_path_crowded(rt, &self.overlays, start, (cx, cy), true, false, &occ)
        {
            self.actors[idx].path = p.into();
            self.actors[idx].task = PendingTask::ChopAt { tx: cx, ty: cy };
            self.actors[idx].activity_tag = "採伐竹材".into();
            return;
        }

        // 優先級 D：將散落在外之物資自動搬回儲物地墊
        if self.actors[idx].carrying.is_none()
            && let Some((lx, ly, item)) = self.find_loose_item_outside_stockpile(start, &claimed)
            && let Some((kx, ky)) = self.find_available_stockpile_slot(start, item)
            && let Some(p) =
                find_micro_path_crowded(rt, &self.overlays, start, (lx, ly), true, false, &occ)
        {
            self.actors[idx].path = p.into();
            self.actors[idx].task = PendingTask::HaulToStockpile {
                src_x: lx,
                src_y: ly,
                dst_x: kx,
                dst_y: ky,
            };
            self.actors[idx].activity_tag = format!("搬回{}", item.label());
            return;
        }

        if super::rim_economy::try_assign_market_livelihood(self, rt, idx, start, home, &claimed, &occ) {
            return;
        }

        if self.tick % 19 == (self.actors[idx].id as u64 % 19) {
            let angle = (self.tick as f32 * 0.7) + self.actors[idx].id as f32;
            let gx = home.0 + (angle.cos() * 3.5).round() as i32;
            let gy = home.1 + (angle.sin() * 3.5).round() as i32;
            if let Some(p) =
                find_micro_path_crowded(rt, &self.overlays, start, (gx, gy), false, false, &occ)
            {
                self.actors[idx].path = p.into();
                self.actors[idx].activity_tag = "營畔歇息".into();
            }
        }
    }

    pub(crate) fn advance_actor(&mut self, rt: Option<&SubstrateRuntime>, idx: usize) {
        let is_beast = self.actors[idx].faction == ActorFaction::WildBeast;
        if let PendingTask::RescueAlly { target_id } = self.actors[idx].task {
            super::rim_life::complete_rescue_ally(self, rt, idx, target_id);
            return;
        }
        if let PendingTask::ChaseTarget { target_id } = self.actors[idx].task {
            if let Some(t) = self.actors.iter().find(|a| a.id == target_id && a.hp > 0) {
                let dist = (t.px - self.actors[idx].px).hypot(t.py - self.actors[idx].py);
                if dist <= 1.35 {
                    self.actors[idx].path.clear();
                    return;
                }
                if self.tick % 3 == 0 {
                    let s = self.actors[idx].micro_pos();
                    if let Some(p) =
                        find_micro_path(rt, &self.overlays, s, t.micro_pos(), true, is_beast)
                    {
                        self.actors[idx].path = p.into();
                    }
                }
            } else {
                self.actors[idx].task = PendingTask::None;
                self.actors[idx].activity_tag = "收勢待命".into();
            }
        }

        if !self.actors[idx].path.is_empty() {
            return;
        }

        let (ax, ay) = self.actors[idx].micro_pos();
        let tool_bonus: u8 = match self.actors[idx].equipped_tool {
            Some(RimItemKind::AlloyBlade | RimItemKind::AlloyPick) => 3,
            Some(RimItemKind::StonePick | RimItemKind::BambooBlade) => 2,
            _ => 1,
        };

        match self.actors[idx].task {
            PendingTask::None => {
                if self.actors[idx].activity_tag == "行進中"
                    || self.actors[idx].activity_tag == "疾奔中"
                {
                    self.actors[idx].activity_tag = "駐足待命".into();
                }
            }
            PendingTask::ChopAt { tx, ty } => {
                if (ax - tx).abs() > 1 || (ay - ty).abs() > 1 {
                    self.actors[idx].task = PendingTask::None;
                    return;
                }
                self.actors[idx].facing_rad = ((ty - ay) as f32).atan2((tx - ax) as f32);
                let (_, nat) = derive_micro_pristine(rt, tx, ty);
                let Some(prop) = nat else {
                    if let Some(ov) = self.overlays.get_mut(&(tx, ty)) {
                        ov.blueprint = None;
                    }
                    self.actors[idx].task = PendingTask::None;
                    return;
                };
                let entry = self.overlays.entry((tx, ty)).or_default();
                let next_prog = match entry.blueprint {
                    Some(BlueprintJob::Chop { progress }) => progress.saturating_add(tool_bonus),
                    _ => tool_bonus,
                };
                if next_prog >= 3 {
                    entry.cleared_natural = true;
                    entry.blueprint = None;
                    let (drop_item, drop_cnt) = match prop {
                        NaturalProp::BambooClump => (RimItemKind::BambooPole, 2),
                        NaturalProp::BambooShoot => (RimItemKind::BambooShoot, 1),
                        NaturalProp::RiverStone => (RimItemKind::RiverStone, 1),
                        NaturalProp::WildHerb => (RimItemKind::MountainHerb, 2),
                    };
                    let inv_forage: u16 = self.actors[idx]
                        .inventory
                        .iter()
                        .filter(|s| s.item == drop_item)
                        .map(|s| s.count)
                        .sum();
                    if (self.actors[idx].entity_id.is_some() && !self.actors[idx].autopilot)
                        || (matches!(
                            prop,
                            NaturalProp::BambooShoot | NaturalProp::WildHerb
                        ) && inv_forage < 6)
                    {
                        self.actors[idx].add_inv(drop_item, drop_cnt);
                    } else {
                        self.add_ground_item(tx, ty, drop_item, drop_cnt);
                    }
                    let glyph = self.actors[idx].glyph.clone();
                    self.actors[idx].craft_harvest =
                        self.actors[idx].craft_harvest.saturating_add(1);
                    if prop == NaturalProp::BambooShoot
                        || self.actors[idx].craft_harvest.is_multiple_of(3)
                    {
                        self.actors[idx].push_diary(format!("在林緣採得{}", drop_item.label()));
                    }
                    self.actors[idx].pop_text = Some((
                        format!("得{}×{}", drop_item.short_char(), drop_cnt),
                        self.tick + 8,
                    ));
                    self.actors[idx].task = PendingTask::None;
                    self.actors[idx].activity_tag = "採伐完畢".into();
                    self.push_event(format!("{glyph} 砍開{}", prop.label()));
                    super::rim_economy::record_tool_wear(self, idx);
                } else {
                    entry.blueprint = Some(BlueprintJob::Chop {
                        progress: next_prog,
                    });
                }
            }
            PendingTask::FetchForBuild {
                src_x,
                src_y,
                dst_x,
                dst_y,
            } => {
                if (ax - src_x).abs() <= 1 && (ay - src_y).abs() <= 1 {
                    let got = self.take_ground_item(src_x, src_y, RimItemKind::BambooPole, 2);
                    if got > 0 {
                        self.actors[idx].carrying = Some(GroundStack {
                            item: RimItemKind::BambooPole,
                            count: got,
                        });
                        if let Some(p) = find_micro_path(
                            rt,
                            &self.overlays,
                            (ax, ay),
                            (dst_x, dst_y),
                            true,
                            false,
                        ) {
                            self.actors[idx].path = p.into();
                            self.actors[idx].task = PendingTask::WorkBuild {
                                tx: dst_x,
                                ty: dst_y,
                            };
                            self.actors[idx].activity_tag = "抱竹赴工".into();
                            return;
                        }
                    }
                }
                self.actors[idx].task = PendingTask::None;
            }
            PendingTask::WorkBuild { tx, ty } => {
                if (ax - tx).abs() > 1 || (ay - ty).abs() > 1 {
                    self.actors[idx].task = PendingTask::None;
                    return;
                }
                let Some(BlueprintJob::Build {
                    structure,
                    mut delivered_bamboo,
                    mut progress,
                }) = self.overlays.get(&(tx, ty)).and_then(|o| o.blueprint)
                else {
                    self.actors[idx].task = PendingTask::None;
                    return;
                };
                let cost = structure.bamboo_cost();
                if delivered_bamboo < cost {
                    let from_inv = self.actors[idx]
                        .take_inv(RimItemKind::BambooPole, u16::from(cost - delivered_bamboo));
                    if from_inv > 0 {
                        delivered_bamboo += from_inv as u8;
                    }
                }
                if delivered_bamboo < cost {
                    if let Some(mut carry) = self.actors[idx].carrying
                        && carry.item == RimItemKind::BambooPole
                        && carry.count > 0
                    {
                        let put = (cost - delivered_bamboo).min(carry.count as u8);
                        delivered_bamboo += put;
                        carry.count -= u16::from(put);
                        self.actors[idx].carrying = (carry.count > 0).then_some(carry);
                    } else if let Some((sx, sy)) =
                        self.find_nearest_ground_item((ax, ay), RimItemKind::BambooPole, false)
                        && let Some(p) =
                            find_micro_path(rt, &self.overlays, (ax, ay), (sx, sy), true, false)
                    {
                        self.actors[idx].path = p.into();
                        self.actors[idx].task = PendingTask::FetchForBuild {
                            src_x: sx,
                            src_y: sy,
                            dst_x: tx,
                            dst_y: ty,
                        };
                        self.actors[idx].activity_tag = "取竹補料".into();
                        return;
                    } else {
                        self.actors[idx].pop_text = Some(("缺莿竹稈".into(), self.tick + 8));
                        self.actors[idx].task = PendingTask::None;
                        return;
                    }
                }

                progress = progress.saturating_add(tool_bonus);
                if progress >= structure.work_ticks_needed() {
                    let ov = self.overlays.entry((tx, ty)).or_default();
                    ov.cleared_natural = true;
                    ov.structure = Some(structure);
                    ov.blueprint = None;
                    let glyph = self.actors[idx].glyph.clone();
                    self.actors[idx].craft_forge = self.actors[idx].craft_forge.saturating_add(2);
                    self.actors[idx].push_diary(format!("合力建成{}", structure.label()));
                    self.actors[idx].pop_text =
                        Some((format!("建成{}", structure.label()), self.tick + 10));
                    self.actors[idx].task = PendingTask::None;
                    self.actors[idx].activity_tag = "完工待命".into();
                    self.push_event(format!("{glyph} 建成［{}］", structure.label()));
                    super::rim_economy::record_tool_wear(self, idx);
                } else if let Some(ov) = self.overlays.get_mut(&(tx, ty)) {
                    ov.blueprint = Some(BlueprintJob::Build {
                        structure,
                        delivered_bamboo,
                        progress,
                    });
                }
            }
            PendingTask::DeconstructAt { tx, ty } => {
                if (ax - tx).abs() <= 1 && (ay - ty).abs() <= 1 {
                    let refund = self
                        .overlays
                        .get(&(tx, ty))
                        .and_then(|o| o.structure)
                        .map_or(0, StructureKind::bamboo_cost);
                    if let Some(ov) = self.overlays.get_mut(&(tx, ty)) {
                        ov.structure = None;
                        ov.blueprint = None;
                    }
                    if refund > 0 {
                        self.add_ground_item(tx, ty, RimItemKind::BambooPole, u16::from(refund));
                    }
                    self.push_event("已拆除設施退回竹材");
                }
                self.actors[idx].task = PendingTask::None;
            }
            PendingTask::HaulToStockpile {
                src_x,
                src_y,
                dst_x,
                dst_y,
            } => {
                if self.actors[idx].carrying.is_none() {
                    if (ax - src_x).abs() <= 1
                        && (ay - src_y).abs() <= 1
                        && let Some(item) = self
                            .overlays
                            .get(&(src_x, src_y))
                            .and_then(|o| o.ground_item)
                            .map(|g| g.item)
                    {
                        let got = self.take_ground_item(src_x, src_y, item, 4);
                        if got > 0 {
                            self.actors[idx].carrying = Some(GroundStack { item, count: got });
                            if let Some(p) = find_micro_path(
                                rt,
                                &self.overlays,
                                (ax, ay),
                                (dst_x, dst_y),
                                true,
                                false,
                            ) {
                                self.actors[idx].path = p.into();
                                return;
                            }
                        }
                    }
                    self.actors[idx].task = PendingTask::None;
                } else if (ax - dst_x).abs() <= 1 && (ay - dst_y).abs() <= 1 {
                    if let Some(carry) = self.actors[idx].carrying.take() {
                        self.add_ground_item(dst_x, dst_y, carry.item, carry.count);
                    }
                    self.actors[idx].task = PendingTask::None;
                    self.actors[idx].activity_tag = "歸倉完畢".into();
                } else {
                    self.actors[idx].task = PendingTask::None;
                }
            }
            PendingTask::PickupEquip { tx, ty } => {
                if (ax - tx).abs() <= 1
                    && (ay - ty).abs() <= 1
                    && let Some(g) = self.overlays.get(&(tx, ty)).and_then(|o| o.ground_item)
                {
                    if self.actors[idx].entity_id.is_some()
                        || g.item.is_edible()
                        || g.item.is_medical()
                        || matches!(
                            g.item,
                            RimItemKind::BambooShoot
                                | RimItemKind::BeastMeat
                                | RimItemKind::MountainHerb
                        )
                    {
                        let max_take = if self.actors[idx].entity_id.is_some() {
                            6
                        } else {
                            2
                        };
                        let got = self.take_ground_item(tx, ty, g.item, max_take);
                        if got > 0 {
                            self.actors[idx].add_inv(g.item, got);
                            self.actors[idx].pop_text = Some((
                                format!("收入{}×{}", g.item.short_char(), got),
                                self.tick + 8,
                            ));
                        }
                    } else if g.item.is_hand_tool() {
                        let _ = self.take_ground_item(tx, ty, g.item, 1);
                        self.actors[idx].equipped_tool = Some(g.item);
                        self.actors[idx].pop_text =
                            Some((format!("持{}", g.item.label()), self.tick + 10));
                    } else if self.actors[idx].carrying.is_none() {
                        let got = self.take_ground_item(tx, ty, g.item, 4);
                        self.actors[idx].carrying = Some(GroundStack {
                            item: g.item,
                            count: got,
                        });
                        self.actors[idx].pop_text =
                            Some((format!("抱起{}", g.item.label()), self.tick + 8));
                    }
                }
                self.actors[idx].task = PendingTask::None;
            }
            PendingTask::CraftAt { tx, ty, item } => {
                if item == RimItemKind::WoundSalve {
                    super::rim_ecology::complete_craft_salve(self, idx, tx, ty);
                    self.actors[idx].task = PendingTask::None;
                    return;
                }
                if (ax - tx).abs() <= 1 && (ay - ty).abs() <= 1 {
                    let has_bench = self.overlays.get(&(tx, ty)).and_then(|o| o.structure)
                        == Some(StructureKind::Workbench);
                    if has_bench {
                        let need_bamboo: u16 = if item == RimItemKind::BambooPack {
                            2
                        } else {
                            1
                        };
                        let need_stone = matches!(
                            item,
                            RimItemKind::StonePick
                                | RimItemKind::AlloyBlade
                                | RimItemKind::AlloyPick
                        );
                        let inv_b = self.actors[idx]
                            .inventory
                            .iter()
                            .filter(|s| s.item == RimItemKind::BambooPole)
                            .map(|s| s.count)
                            .sum::<u16>();
                        let b_pos =
                            self.find_nearest_ground_item((tx, ty), RimItemKind::BambooPole, false);
                        let have_b = inv_b >= need_bamboo || b_pos.is_some();

                        let inv_s = self.actors[idx]
                            .inventory
                            .iter()
                            .filter(|s| s.item == RimItemKind::RiverStone)
                            .map(|s| s.count)
                            .sum::<u16>();
                        let s_pos = if need_stone {
                            self.find_nearest_ground_item((tx, ty), RimItemKind::RiverStone, false)
                        } else {
                            Some((tx, ty))
                        };
                        let have_s = !need_stone || inv_s >= 1 || s_pos.is_some();

                        if have_b && have_s {
                            let from_inv =
                                self.actors[idx].take_inv(RimItemKind::BambooPole, need_bamboo);
                            let rem_b = need_bamboo - from_inv;
                            if rem_b > 0
                                && let Some((bx, by)) = b_pos
                            {
                                let _ =
                                    self.take_ground_item(bx, by, RimItemKind::BambooPole, rem_b);
                            }
                            if need_stone {
                                let taken_s = self.actors[idx].take_inv(RimItemKind::RiverStone, 1);
                                if taken_s == 0
                                    && let Some((sx, sy)) = s_pos
                                {
                                    let _ =
                                        self.take_ground_item(sx, sy, RimItemKind::RiverStone, 1);
                                }
                            }
                            if item == RimItemKind::BambooHat && !self.actors[idx].equipped_hat {
                                self.actors[idx].equipped_hat = true;
                            } else if item == RimItemKind::BambooPack
                                && !self.actors[idx].equipped_pack
                            {
                                self.actors[idx].equipped_pack = true;
                            } else if item.is_hand_tool()
                                && self.actors[idx].equipped_tool.is_none()
                            {
                                self.actors[idx].equipped_tool = Some(item);
                            } else {
                                self.actors[idx].add_inv(item, 1);
                            }
                            self.actors[idx].craft_forge =
                                self.actors[idx].craft_forge.saturating_add(2);
                            self.actors[idx].push_diary(format!("於案台製成{}", item.label()));
                            self.actors[idx].pop_text =
                                Some((format!("製成{}", item.label()), self.tick + 12));
                            self.push_event(format!(
                                "{} 製成［{}］",
                                self.actors[idx].glyph,
                                item.label()
                            ));
                        } else {
                            self.actors[idx].pop_text =
                                Some(("行囊或儲物墊缺竹石原材".into(), self.tick + 10));
                        }
                    }
                }
                self.actors[idx].task = PendingTask::None;
            }
            PendingTask::RestAt { tx, ty } => {
                if (ax - tx).abs() <= 2 && (ay - ty).abs() <= 2 {
                    self.actors[idx].facing_rad = ((ty - ay) as f32).atan2((tx - ax) as f32);
                    self.actors[idx].activity_tag = "圍爐歇息".into();
                } else {
                    self.actors[idx].activity_tag = "駐足待命".into();
                }
                self.actors[idx].task = PendingTask::None;
            }
            PendingTask::DrinkAt { tx, ty } => {
                if (ax - tx).abs() <= 2 && (ay - ty).abs() <= 2 {
                    self.actors[idx].facing_rad = ((ty - ay) as f32).atan2((tx - ax) as f32);
                    self.actors[idx].thirst = 100;
                    self.actors[idx].stamina = self.actors[idx].stamina.saturating_add(10).min(100);
                    self.actors[idx].activity_tag = "掬泉飲水".into();
                    self.actors[idx].pop_text = Some(("掬泉潤喉".into(), self.tick + 8));
                }
                self.actors[idx].task = PendingTask::None;
            }
            PendingTask::CookAt { tx, ty, raw } => {
                if (ax - tx).abs() <= 2
                    && (ay - ty).abs() <= 2
                    && self.overlays.get(&(tx, ty)).and_then(|o| o.structure)
                        == Some(StructureKind::Campfire)
                {
                    let dish = match raw {
                        RimItemKind::BeastMeat => RimItemKind::RoastedMeat,
                        RimItemKind::SweetTuber => RimItemKind::RoastedTuber,
                        _ => RimItemKind::RoastedShoot,
                    };
                    let mut got = self.actors[idx].take_inv(raw, 1);
                    if got == 0
                        && let Some((sx, sy)) = self.find_nearest_ground_item((tx, ty), raw, false)
                    {
                        got = self.take_ground_item(sx, sy, raw, 1);
                    }
                    if got > 0 {
                        self.actors[idx].add_inv(dish, 1);
                        self.actors[idx].craft_culinary =
                            self.actors[idx].craft_culinary.saturating_add(2);
                        self.actors[idx].push_diary(format!("於營火邊烹成{}", dish.label()));
                        self.actors[idx].activity_tag = format!("烹製{}", dish.label());
                        self.actors[idx].pop_text =
                            Some((format!("備妥{}", dish.label()), self.tick + 10));
                        self.push_event(format!(
                            "{} 烤好［{}］",
                            self.actors[idx].glyph,
                            dish.label()
                        ));
                    } else {
                        self.actors[idx].pop_text =
                            Some((format!("身上無{}", raw.label()), self.tick + 8));
                    }
                }
                self.actors[idx].task = PendingTask::None;
            }
            PendingTask::ChaseTarget { .. } | PendingTask::RescueAlly { .. } => {}
            PendingTask::CaravanMove { tx, ty, leg } => {
                if (ax - tx).abs() + (ay - ty).abs() <= 5 {
                    super::rim_phase::complete_caravan_arrival(self, rt, idx, leg);
                } else {
                    self.actors[idx].task = PendingTask::None;
                }
            }
            PendingTask::SleepBed { tx, ty } => {
                super::rim_shelter::complete_sleep_bed(self, idx, tx, ty);
            }
            PendingTask::FarmPlot { tx, ty, op } => {
                super::rim_ecology::complete_farm_plot(self, rt, idx, tx, ty, op);
            }
            PendingTask::StorehouseTrade { tx, ty } => {
                super::rim_economy::complete_storehouse_trade(self, idx, tx, ty);
            }
        }
    }

    pub(crate) fn resolve_collisions_and_combat(&mut self) {
        super::rim_phase::resolve_world_combat_round(self);
    }

    pub(crate) fn add_ground_item(&mut self, mx: i32, my: i32, item: RimItemKind, count: u16) {
        if count == 0 {
            return;
        }
        let candidates = [
            (mx, my),
            (mx + 1, my),
            (mx - 1, my),
            (mx, my + 1),
            (mx, my - 1),
        ];
        for (cx, cy) in candidates {
            let entry = self.overlays.entry((cx, cy)).or_default();
            match &mut entry.ground_item {
                None => {
                    entry.ground_item = Some(GroundStack { item, count });
                    return;
                }
                Some(g) if g.item == item => {
                    g.count = g.count.saturating_add(count);
                    return;
                }
                _ => continue,
            }
        }
    }

    pub(crate) fn take_ground_item(
        &mut self,
        mx: i32,
        my: i32,
        want: RimItemKind,
        max_take: u16,
    ) -> u16 {
        let Some(ov) = self.overlays.get_mut(&(mx, my)) else {
            return 0;
        };
        let Some(mut g) = ov.ground_item else {
            return 0;
        };
        if g.item != want {
            return 0;
        }
        let taken = g.count.min(max_take);
        g.count -= taken;
        ov.ground_item = if g.count > 0 { Some(g) } else { None };
        if ov.is_pristine() {
            self.overlays.remove(&(mx, my));
        }
        taken
    }

    pub(crate) fn find_nearest_ground_item(
        &self,
        from: (i32, i32),
        want: RimItemKind,
        exclude_stockpile: bool,
    ) -> Option<(i32, i32)> {
        self.overlays
            .iter()
            .filter(|(_, ov)| {
                if exclude_stockpile && ov.structure == Some(StructureKind::Stockpile) {
                    return false;
                }
                ov.ground_item
                    .is_some_and(|g| g.item == want && g.count > 0)
            })
            .min_by_key(|(pos, _)| (pos.0 - from.0).abs() + (pos.1 - from.1).abs())
            .map(|(&pos, _)| pos)
    }

    pub(crate) fn find_loose_item_outside_stockpile(
        &self,
        from: (i32, i32),
        claimed: &[(i32, i32)],
    ) -> Option<(i32, i32, RimItemKind)> {
        self.overlays
            .iter()
            .filter_map(|(&pos, ov)| {
                if ov.structure == Some(StructureKind::Stockpile) || claimed.contains(&pos) {
                    return None;
                }
                if (pos.0 - from.0).abs() + (pos.1 - from.1).abs() > 26 {
                    return None;
                }
                let g = ov.ground_item?;
                (g.count > 0 && self.find_available_stockpile_slot(from, g.item).is_some())
                    .then_some((pos.0, pos.1, g.item))
            })
            .min_by_key(|(x, y, _)| (x - from.0).abs() + (y - from.1).abs())
    }

    pub(crate) fn find_available_stockpile_slot(
        &self,
        from: (i32, i32),
        item: RimItemKind,
    ) -> Option<(i32, i32)> {
        super::rim_economy::find_best_stockpile_slot_for(&self.overlays, from, item, None)
    }
}

//! 輕量版 RimWorld 2D 地圖玩家指令分派與藍圖指定模組。

use super::rim_sim::{PendingTask, RimWorldState, now_unix_sec};
use super::rim_world::{
    BlueprintJob, MicroGround, NaturalProp, RimItemKind, StructureKind, derive_micro_pristine,
    find_micro_path,
};
use super::runtime;

/// 將任意舊稱或空白行事方針正規化為四項標準駐世方針碼之一。
pub fn normalize_directive_code(raw: &str) -> &'static str {
    match raw.trim() {
        "craft" | "assist" | "百工營造" => "craft",
        "guard" | "patrol" | "護衛獵巡" => "guard",
        "rest" | "爐畔靜養" => "rest",
        _ => "forage",
    }
}

impl RimWorldState {
    /// 玩家下達框選藍圖／砍伐／拆除指令。
    pub fn designate_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, tool: &str) -> usize {
        let rt = runtime::global();
        let min_x = x0.min(x1);
        let max_x = x0.max(x1).min(min_x + 36);
        let min_y = y0.min(y1);
        let max_y = y0.max(y1).min(min_y + 36);
        let mut count = 0usize;

        for my in min_y..=max_y {
            for mx in min_x..=max_x {
                let (ground, nat) = derive_micro_pristine(rt, mx, my);
                match tool {
                    "chop" => {
                        let cleared = self
                            .overlays
                            .get(&(mx, my))
                            .is_some_and(|o| o.cleared_natural || o.structure.is_some());
                        if !cleared && nat.is_some() {
                            self.overlays.entry((mx, my)).or_default().blueprint =
                                Some(BlueprintJob::Chop { progress: 0 });
                            count += 1;
                        }
                    }
                    "deconstruct" => {
                        if let Some(ov) = self.overlays.get_mut(&(mx, my)) {
                            if ov.blueprint.is_some() {
                                ov.blueprint = None;
                                count += 1;
                            } else if ov.structure.is_some() {
                                ov.blueprint = Some(BlueprintJob::Deconstruct { progress: 0 });
                                count += 1;
                            }
                            if ov.is_pristine() {
                                self.overlays.remove(&(mx, my));
                            }
                        }
                    }
                    _ => {
                        let Some(st) = (match tool {
                            "wall" => Some(StructureKind::BambooWall),
                            "door" => Some(StructureKind::BambooDoor),
                            "bridge" => Some(StructureKind::WoodBridge),
                            "campfire" => Some(StructureKind::Campfire),
                            "workbench" => Some(StructureKind::Workbench),
                            "stockpile" => Some(StructureKind::Stockpile),
                            "bed" => Some(StructureKind::BambooBed),
                            "paving" => Some(StructureKind::StonePaving),
                            "arcade" => Some(StructureKind::CoveredArcade),
                            "lamp" => Some(StructureKind::StreetLamp),
                            _ => None,
                        }) else {
                            continue;
                        };
                        if st == StructureKind::WoodBridge {
                            if !matches!(
                                ground,
                                MicroGround::ShallowStream | MicroGround::PebbleBank
                            ) {
                                continue;
                            }
                        } else if ground.is_water() || ground == MicroGround::SteepCliff {
                            continue;
                        }
                        let existing = self.overlays.get(&(mx, my));
                        if existing.and_then(|o| o.structure).is_some() {
                            continue;
                        }
                        let cleared = existing.is_some_and(|o| o.cleared_natural);
                        if !cleared && nat.is_some_and(NaturalProp::blocks_movement) {
                            self.overlays.entry((mx, my)).or_default().blueprint =
                                Some(BlueprintJob::Chop { progress: 0 });
                            count += 1;
                            continue;
                        }
                        if matches!(st, StructureKind::Stockpile | StructureKind::StonePaving) {
                            let ov = self.overlays.entry((mx, my)).or_default();
                            ov.cleared_natural = true;
                            ov.structure = Some(st);
                            ov.blueprint = None;
                            count += 1;
                        } else {
                            self.overlays.entry((mx, my)).or_default().blueprint =
                                Some(BlueprintJob::Build {
                                    structure: st,
                                    delivered_bamboo: 0,
                                    progress: 0,
                                });
                            count += 1;
                        }
                    }
                }
            }
        }
        if count > 0 {
            self.dirty_unsaved_ticks += 5;
        }
        count
    }

    pub fn command_player(
        &mut self,
        pid: &str,
        mx: i32,
        my: i32,
        action: Option<&str>,
        target_actor: Option<usize>,
        run: bool,
    ) -> Result<String, String> {
        let rt = runtime::global();
        let Some(p_idx) = self
            .actors
            .iter()
            .position(|a| a.entity_id.as_deref() == Some(pid))
        else {
            return Err("找不到對應角色實體".into());
        };
        if self.actors[p_idx].downed_ticks > 0 {
            return Err("此刻力竭未起，尚待同伴扶持或緩氣".into());
        }
        let start = self.actors[p_idx].micro_pos();
        let aid = self.actors[p_idx].id;
        self.actors[p_idx].autopilot = false;
        self.actors[p_idx].running = run;
        self.actors[p_idx].last_active_unix = now_unix_sec();
        self.stall_tracker.remove(&aid);
        self.economy.parcel_queues.remove(&aid);
        self.dirty_unsaved_ticks += 4;

        if let Some(tid) = target_actor
            && let Some(target) = self
                .actors
                .iter()
                .find(|a| a.id == tid && a.hp > 0 && a.downed_ticks == 0)
        {
            let tpos = target.micro_pos();
            let path =
                find_micro_path(rt, &self.overlays, start, tpos, true, false).unwrap_or_default();
            self.actors[p_idx].path = path.into();
            self.actors[p_idx].task = PendingTask::ChaseTarget { target_id: tid };
            self.actors[p_idx].activity_tag = "上前交鋒".into();
            return Ok("鎖定目標上前拆招".into());
        }

        match action.unwrap_or("move") {
            "move" => {
                let Some(path) = find_micro_path(rt, &self.overlays, start, (mx, my), true, false)
                else {
                    self.actors[p_idx].running = false;
                    return Err("水深或密林阻隔，需架橋或開路".into());
                };
                self.actors[p_idx].path = path.into();
                self.actors[p_idx].task = PendingTask::None;
                self.actors[p_idx].activity_tag = if run { "疾奔中" } else { "行進中" }.into();
                Ok(if run { "疾步奔往" } else { "循路前往" }.into())
            }
            "rest" => {
                let Some(path) = find_micro_path(rt, &self.overlays, start, (mx, my), true, false)
                else {
                    self.actors[p_idx].running = false;
                    return Err("無法走近營火".into());
                };
                self.actors[p_idx].path = path.into();
                self.actors[p_idx].task = PendingTask::RestAt { tx: mx, ty: my };
                self.actors[p_idx].activity_tag = "前往歇息".into();
                Ok("前往圍爐歇息".into())
            }
            "sleep_bed" => {
                let Some(path) = find_micro_path(rt, &self.overlays, start, (mx, my), false, false)
                else {
                    self.actors[p_idx].running = false;
                    return Err("竹編睡榻通路受阻".into());
                };
                self.actors[p_idx].path = path.into();
                self.actors[p_idx].task = PendingTask::SleepBed { tx: mx, ty: my };
                self.actors[p_idx].activity_tag = "前往就榻".into();
                Ok("前往登榻歇息".into())
            }
            "chop" => {
                self.designate_rect(mx, my, mx, my, "chop");
                let Some(path) = find_micro_path(rt, &self.overlays, start, (mx, my), true, false)
                else {
                    return Err("無法靠近該處".into());
                };
                self.actors[p_idx].path = path.into();
                self.actors[p_idx].task = PendingTask::ChopAt { tx: mx, ty: my };
                self.actors[p_idx].activity_tag = "親手採伐".into();
                Ok("前往親手採伐".into())
            }
            "build" => {
                let Some(path) = find_micro_path(rt, &self.overlays, start, (mx, my), true, false)
                else {
                    return Err("無法靠近營造處".into());
                };
                self.actors[p_idx].path = path.into();
                self.actors[p_idx].task = PendingTask::WorkBuild { tx: mx, ty: my };
                self.actors[p_idx].activity_tag = "親手築造".into();
                Ok("前往親手施工".into())
            }
            "pickup" => {
                let Some(path) = find_micro_path(rt, &self.overlays, start, (mx, my), true, false)
                else {
                    return Err("無法靠近物品".into());
                };
                self.actors[p_idx].path = path.into();
                self.actors[p_idx].task = PendingTask::PickupEquip { tx: mx, ty: my };
                self.actors[p_idx].activity_tag = "前往拾取".into();
                Ok("前往拾取".into())
            }
            "drink" => {
                let mut best_bank = None;
                for (ox, oy) in [
                    (0, -1),
                    (0, 1),
                    (-1, 0),
                    (1, 0),
                    (-1, -1),
                    (1, -1),
                    (-1, 1),
                    (1, 1),
                ] {
                    let bank = (mx + ox, my + oy);
                    if let Some(path) =
                        find_micro_path(rt, &self.overlays, start, bank, true, false)
                        && best_bank
                            .as_ref()
                            .is_none_or(|(bp, _): &(Vec<(i32, i32)>, _)| path.len() < bp.len())
                    {
                        best_bank = Some((path, bank));
                    }
                }
                let Some((path, _)) = best_bank else {
                    return Err("水岸難近，無法掬泉飲水".into());
                };
                self.actors[p_idx].path = path.into();
                self.actors[p_idx].task = PendingTask::DrinkAt { tx: mx, ty: my };
                self.actors[p_idx].activity_tag = "前往飲泉".into();
                Ok("循溪岸前往掬泉解渴".into())
            }
            "cook_shoot" | "cook_meat" | "cook_tuber" => {
                let raw = match action {
                    Some("cook_meat") => RimItemKind::BeastMeat,
                    Some("cook_tuber") => RimItemKind::SweetTuber,
                    _ => RimItemKind::BambooShoot,
                };
                let Some(path) = find_micro_path(rt, &self.overlays, start, (mx, my), true, false)
                else {
                    return Err("無法走近營火".into());
                };
                self.actors[p_idx].path = path.into();
                self.actors[p_idx].task = PendingTask::CookAt {
                    tx: mx,
                    ty: my,
                    raw,
                };
                self.actors[p_idx].activity_tag = format!("赴火炙{}", raw.short_char());
                Ok(format!("前往營火旁備妥{}", raw.label()))
            }
            "craft_blade"
            | "craft_pick"
            | "craft_alloy_blade"
            | "craft_alloy_pick"
            | "craft_hat"
            | "craft_pack"
            | "craft_salve" => {
                let item = match action {
                    Some("craft_pick") => RimItemKind::StonePick,
                    Some("craft_alloy_blade") => RimItemKind::AlloyBlade,
                    Some("craft_alloy_pick") => RimItemKind::AlloyPick,
                    Some("craft_hat") => RimItemKind::BambooHat,
                    Some("craft_pack") => RimItemKind::BambooPack,
                    Some("craft_salve") => RimItemKind::WoundSalve,
                    _ => RimItemKind::BambooBlade,
                };
                let Some(path) = find_micro_path(rt, &self.overlays, start, (mx, my), true, false)
                else {
                    return Err("無法靠近工作台".into());
                };
                self.actors[p_idx].path = path.into();
                self.actors[p_idx].task = PendingTask::CraftAt {
                    tx: mx,
                    ty: my,
                    item,
                };
                self.actors[p_idx].activity_tag = format!("製作{}", item.label());
                Ok(format!("前往工作台製作{}", item.label()))
            }
            "till_ridge" | "plant_herb" | "plant_tuber" | "harvest_plot" => {
                use super::rim_ecology::{FarmOp, enqueue_contiguous_parcel};
                let (op, act_tag, ok_msg) = match action {
                    Some("till_ridge") => (FarmOp::Till, "整畦翻土", "前往持鎬整畦翻鬆田壟"),
                    Some("plant_herb") => (FarmOp::PlantHerb, "整畦栽藥", "前往整畦栽下青草藥苗"),
                    Some("plant_tuber") => (FarmOp::PlantTuber, "整畦栽薯", "前往整畦扦插紅土甘薯"),
                    _ => (FarmOp::Harvest, "整畦採收", "前往整畦採收成熟作物"),
                };
                let Some(path) = find_micro_path(rt, &self.overlays, start, (mx, my), true, false)
                else {
                    return Err("無法走近該處田地".into());
                };
                self.actors[p_idx].path = path.into();
                self.actors[p_idx].task = PendingTask::FarmPlot { tx: mx, ty: my, op };
                enqueue_contiguous_parcel(self, p_idx, mx, my, op);
                self.actors[p_idx].activity_tag = act_tag.into();
                Ok(ok_msg.into())
            }
            _ => Err("無法執行此動作".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::rim_ecology::FarmOp;
    use super::super::rim_sim::{ActorFaction, RimActor, RimWorldState};
    use super::super::rim_world::MicroCellOverlay;
    use super::*;
    use std::collections::{HashMap, VecDeque};

    fn cmd_actor(id: usize, eid: Option<&str>, px: f32, py: f32) -> RimActor {
        RimActor {
            id,
            code: format!("c_{id}"),
            actor_key: format!("c_{id}"),
            entity_id: eid.map(str::to_string),
            glyph: if eid.is_some() { "我" } else { "友" }.into(),
            name: "測試角色".into(),
            gender: "M".into(),
            soul_seed: 7,
            role_tag: "測試".into(),
            faction: ActorFaction::Settler,
            is_player: eid.is_some(),
            autopilot: eid.is_some(),
            running: false,
            px,
            py,
            facing_rad: 0.0,
            hp: 100,
            max_hp: 100,
            equipped_tool: None,
            carrying: None,
            inventory: Vec::new(),
            directive: "forage".into(),
            path: VecDeque::new(),
            task: PendingTask::None,
            activity_tag: String::new(),
            pop_text: None,
            cooldown_ticks: 0,
            last_active_unix: 0,
            hunger: 88,
            thirst: 88,
            stamina: 92,
            magnesium: 18,
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
    fn player_intervention_clears_autopilot_queues_and_moves_monotonically() {
        let mut overlays = HashMap::new();
        for x in -6..=4 {
            for y in -2..=2 {
                overlays.insert((x, y), MicroCellOverlay::cleared(None));
            }
        }
        let mut player = cmd_actor(1, Some("tester"), 0.0, 0.0);
        player.path = VecDeque::from([(1, 0), (2, 0), (3, 0)]);
        let npc = cmd_actor(2, None, -1.0, 1.0);
        let mut world = RimWorldState {
            tick: 0,
            overlays,
            actors: vec![player, npc],
            events: VecDeque::new(),
            economy: Default::default(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::from([(1, ((0.0, 0.0), 3))]),
        };
        world
            .economy
            .parcel_queues
            .insert(1, VecDeque::from([(1, 0, FarmOp::Harvest), (2, 0, FarmOp::Harvest)]));

        assert!(world.command_player("tester", -5, 0, Some("move"), None, true).is_ok());
        assert!(!world.actors[0].autopilot);
        assert!(!world.economy.parcel_queues.contains_key(&1));
        assert!(!world.stall_tracker.contains_key(&1));

        let mut prev_x = world.actors[0].px;
        for _ in 0..40 {
            world.step_physics_50ms();
            let cur_x = world.actors[0].px;
            assert!(cur_x <= prev_x + 1e-4, "player rubber-banded: prev={prev_x} cur={cur_x}");
            assert_ne!(world.actors[0].activity_tag, "側身禮讓");
            prev_x = cur_x;
        }
        assert!(world.actors[0].px < -2.5, "px={}", world.actors[0].px);
    }
}

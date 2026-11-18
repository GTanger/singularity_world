//! 土地動態演變（1m 真實倉廩與成片連壟農田、單次整畦連作、敷傷藥膏與山林雨露再生）模組。

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::rim_economy::find_best_stockpile_slot_for;
use super::rim_shelter::RimWeatherKind;
use super::rim_sim::{ActorFaction, PendingTask, RimActor, RimWorldState};
use super::rim_world::{
    GroundStack, MicroCellOverlay, MicroGround, NaturalProp, RimItemKind, StructureKind,
    derive_micro_pristine, find_micro_path,
};
use super::runtime::SubstrateRuntime;

/// 八處顯式成片田畦邊界 `(x0, x1, y0, y1)`（西麓南坡 4 畦 + 水尾寮溪坪 4 畦）。
const FARM_PARCELS: [(i32, i32, i32, i32); 8] = [
    (-19, -15, -16, -15),
    (-13, -9, -16, -15),
    (-19, -15, -19, -18),
    (-13, -9, -19, -18),
    (29, 32, -13, -12),
    (34, 37, -13, -12),
    (29, 32, -16, -15),
    (34, 37, -16, -15),
];

/// 田壟農事單一步驟（翻土、播種藥苗、扦插甘薯、採收留壟）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmOp {
    Till,
    PlantHerb,
    PlantTuber,
    Harvest,
}

/// 聚落廣場、倉廩棚屋、田區與雙聚落步道淨空帶（林木再生時避開此範圍以免阻路）。
pub fn is_settlement_keep_clear(mx: i32, my: i32) -> bool {
    ((-22..=20).contains(&mx) && (-21..=6).contains(&my))
        || (mx == 0 && (3..=18).contains(&my))
        || ((20..=39).contains(&mx) && (-18..=-3).contains(&my))
}

fn ensure_cell_struct(
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
    pos: (i32, i32),
    st: Option<StructureKind>,
) -> bool {
    let e = overlays.entry(pos).or_default();
    if e.structure != st || !e.cleared_natural || e.blueprint.is_some() {
        e.cleared_natural = true;
        e.structure = st;
        e.blueprint = None;
        e.eco_ticks = 0;
        return true;
    }
    false
}

/// 冪等建置 1m 真實尺度倉廩棚屋、竹木石料場與 72 格成片農莊田畦，並無損遷入舊有廣場窄墊與花盆田物資。
pub fn reconcile_real_scale_layout_and_clutter(
    rt: Option<&SubstrateRuntime>,
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
) -> bool {
    let mut changed = false;

    // 1. 西麓倉廩棚屋 (-13..=-6, -1..=5)、竹木石料場 (-13..=-10, -5/-3) 與水尾寮山貨倉房 (21..=26, -12..=-8)
    let mut seed_struct = |rx: std::ops::RangeInclusive<i32>, ry: std::ops::RangeInclusive<i32>, f: &dyn Fn(i32, i32) -> Option<StructureKind>| {
        for x in rx {
            for y in ry.clone() {
                let (g, _) = derive_micro_pristine(rt, x, y);
                if g.is_water() || g == MicroGround::SteepCliff {
                    continue;
                }
                let e = overlays.entry((x, y)).or_default();
                if e.structure.is_none() && e.blueprint.is_none() && e.ground_item.is_none() {
                    e.cleared_natural = true;
                    e.structure = f(x, y);
                    changed = true;
                }
            }
        }
    };
    seed_struct(-13..=-6, -1..=5, &|x, y| {
        if x == -13 || x == -6 || y == -1 || y == 5 {
            Some(if (x == -6 && y == 2) || (x == -9 && y == -1) { StructureKind::BambooDoor } else { StructureKind::BambooWall })
        } else if y == 4 || (y == 2 && (-12..=-9).contains(&x)) || (y == 0 && x != -9) {
            Some(StructureKind::Stockpile)
        } else {
            None
        }
    });
    seed_struct(-13..=-10, 6..=7, &|_, _| Some(StructureKind::Stockpile));
    seed_struct(21..=26, -12..=-8, &|x, y| {
        if x == 21 || x == 26 || y == -12 || y == -8 {
            Some(if x == 24 && (y == -8 || y == -12) { StructureKind::BambooDoor } else { StructureKind::BambooWall })
        } else if y == -11 || (y == -9 && x != 24) {
            Some(StructureKind::Stockpile)
        } else {
            None
        }
    });
    seed_struct(21..=23, -14..=-14, &|_, _| Some(StructureKind::Stockpile));
    if overlays.get(&(32, -10)).and_then(|o| o.structure) == Some(StructureKind::BambooWall) {
        changed |= ensure_cell_struct(overlays, (33, -10), Some(StructureKind::BambooDoor));
    }

    // 4. 清理舊廣場露天地墊、舊街心料場 (-13..=-10,-5..=-3) 與舊街心農田 (-4..=8,-12..=-6)，無損遷物至新倉
    let mut migrate_items: Vec<GroundStack> = Vec::new();
    let old_stock_pos = (-4..=-3).flat_map(|x| (-4..=-1).map(move |y| (x, y)))
        .chain((-13..=-10).flat_map(|x| (-5..=-3).map(move |y| (x, y))));
    for pos in old_stock_pos {
        if let Some(ov) = overlays.get_mut(&pos)
            && ov.structure == Some(StructureKind::Stockpile)
        {
            if let Some(g) = ov.ground_item.take() {
                migrate_items.push(g);
            }
            ov.structure = (pos.1 == -5).then_some(StructureKind::StonePaving);
            ov.eco_ticks = 0;
            changed = true;
        }
    }
    let old_plot_pos = [(1, -5), (2, -5), (3, -5), (2, -4), (3, -4), (26, -6), (26, -5)]
        .into_iter()
        .chain((-4..=8).flat_map(|x| (-12..=-6).map(move |y| (x, y))));
    for pos in old_plot_pos {
        if let Some(ov) = overlays.get_mut(&pos)
            && ov.structure.is_some_and(StructureKind::is_garden_plot)
        {
            if let Some(g) = ov.ground_item.take() {
                migrate_items.push(g);
            }
            ov.structure = None;
            ov.eco_ticks = 0;
            changed = true;
        }
    }
    for g in migrate_items {
        let mut rem = g.count;
        while rem > 0 {
            if let Some(dst) = find_best_stockpile_slot_for(overlays, (-9, 1), g.item, None) {
                let entry = overlays.entry(dst).or_default();
                match &mut entry.ground_item {
                    None => entry.ground_item = Some(GroundStack { item: g.item, count: 1 }),
                    Some(st) => st.count = st.count.saturating_add(1),
                }
                rem -= 1;
            } else {
                break;
            }
        }
    }

    // 5. 西麓南坡 4 大 5m×2m 田畦（40 格）與水尾寮溪坪 4 大 4m×2m 田畦（32 格），中夾 1m 夯土田埂
    let mut seed_zone = |probe: (i32, i32), rx: std::ops::RangeInclusive<i32>, ry: std::ops::RangeInclusive<i32>, parcels: &[(i32, i32, i32, i32)], third_st: StructureKind| {
        if overlays.get(&probe).and_then(|o| o.structure).is_some_and(StructureKind::is_garden_plot) {
            return;
        }
        for x in rx {
            for y in ry.clone() {
                let (g, _) = derive_micro_pristine(rt, x, y);
                if !g.is_water() && g != MicroGround::SteepCliff {
                    overlays.entry((x, y)).or_default().cleared_natural = true;
                }
            }
        }
        for (idx, &(x0, x1, y0, y1)) in parcels.iter().enumerate() {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (g, _) = derive_micro_pristine(rt, x, y);
                    if g.is_water() || g == MicroGround::SteepCliff {
                        continue;
                    }
                    let st = match idx {
                        0 => if y == y1 { StructureKind::CropMature } else { StructureKind::CropSeedling },
                        1 => if y == y1 { StructureKind::HerbMature } else { StructureKind::HerbSeedling },
                        2 => third_st,
                        _ => StructureKind::TilledRidge,
                    };
                    changed |= ensure_cell_struct(overlays, (x, y), Some(st));
                }
            }
        }
    };
    seed_zone((-19, -15), -20..=-8, -20..=-14, &FARM_PARCELS[..4], StructureKind::CropSeedling);
    seed_zone((29, -12), 28..=38, -17..=-11, &FARM_PARCELS[4..], StructureKind::HerbSeedling);

    // 若新倉廩尚未備入初期青草藥，於室內貨架備好 2 份
    let has_herb = overlays.values().any(|o| o.ground_item.is_some_and(|g| g.item == RimItemKind::MountainHerb));
    if !has_herb
        && let Some(slot) = overlays.get_mut(&(-12, 4))
        && slot.structure == Some(StructureKind::Stockpile)
        && slot.ground_item.is_none()
    {
        slot.ground_item = Some(GroundStack {
            item: RimItemKind::MountainHerb,
            count: 2,
        });
        changed = true;
    }

    changed
}

/// 單次整畦排程：玩家或農夫對任一田壟下達翻土／播種／採收後，自動串連同一田畦內同態相連細格（不跨田埂）。
pub fn enqueue_contiguous_parcel(
    world: &mut RimWorldState,
    idx: usize,
    seed_x: i32,
    seed_y: i32,
    op: FarmOp,
) {
    let matches_op = |ov: Option<&MicroCellOverlay>| -> bool {
        let st = ov.and_then(|o| o.structure);
        match op {
            FarmOp::Till => st.is_none(),
            FarmOp::PlantHerb | FarmOp::PlantTuber => st == Some(StructureKind::TilledRidge),
            FarmOp::Harvest => matches!(st, Some(StructureKind::HerbMature | StructureKind::CropMature)),
        }
    };

    let (x0, x1, y0, y1) = FARM_PARCELS
        .iter()
        .copied()
        .find(|&(a, b, c, d)| (a..=b).contains(&seed_x) && (c..=d).contains(&seed_y))
        .unwrap_or((seed_x - 3, seed_x + 3, seed_y, seed_y));

    let mut q = VecDeque::new();
    for y in y0..=y1 {
        let row: Vec<i32> = if (y - y0) % 2 == 0 {
            (x0..=x1).collect()
        } else {
            (x0..=x1).rev().collect()
        };
        for x in row {
            if (x, y) == (seed_x, seed_y) {
                continue;
            }
            if matches_op(world.overlays.get(&(x, y))) {
                q.push_back((x, y, op));
                if q.len() >= 9 {
                    break;
                }
            }
        }
        if q.len() >= 9 {
            break;
        }
    }
    let aid = world.actors[idx].id;
    world.economy.parcel_queues.insert(aid, q);
}

/// 生態承載力野獸演替（隨外林空間與近期獵捕壓力動態刷新於聚落遠郊密林，不再駐點 30 秒空投）。
pub fn step_wildlife_carrying_capacity(world: &mut RimWorldState, _rt: Option<&SubstrateRuntime>) {
    let interval: u64 = 180 + u64::from(world.economy.hunting_pressure.min(180)) * 2;
    if !world.tick.is_multiple_of(interval) {
        return;
    }
    let target_beasts = if world.economy.hunting_pressure > 60 { 1 } else { 2 };
    let cur_beasts = world
        .actors
        .iter()
        .filter(|a| a.faction == ActorFaction::WildBeast && a.hp > 0)
        .count();
    if cur_beasts >= target_beasts {
        return;
    }
    let next_id = world.actors.iter().map(|a| a.id).max().unwrap_or(10) + 1;
    let b_key = format!("beast_{next_id}");
    world.actors.push(RimActor {
        id: next_id,
        code: b_key.clone(),
        actor_key: b_key,
        entity_id: None,
        glyph: if next_id.is_multiple_of(2) { "豚" } else { "雉" }.into(),
        name: if next_id.is_multiple_of(2) { "紅土野山豬" } else { "鐵羽竹雞" }.into(),
        gender: if next_id.is_multiple_of(2) { "M" } else { "F" }.into(),
        soul_seed: 70_000 + next_id as i64,
        role_tag: "山海野獸".into(),
        faction: ActorFaction::WildBeast,
        is_player: false,
        autopilot: true,
        running: false,
        px: if next_id.is_multiple_of(2) { 18.0 } else { -22.0 },
        py: if next_id.is_multiple_of(2) { 20.0 } else { 14.0 },
        facing_rad: 0.0,
        hp: if next_id.is_multiple_of(2) { 80 } else { 55 },
        max_hp: if next_id.is_multiple_of(2) { 80 } else { 55 },
        equipped_tool: None,
        carrying: None,
        inventory: Vec::new(),
        directive: String::new(),
        path: VecDeque::new(),
        task: PendingTask::None,
        activity_tag: "林間遊走".into(),
        pop_text: None,
        cooldown_ticks: 0,
        last_active_unix: 0,
        hunger: 85,
        thirst: 85,
        stamina: 90,
        magnesium: 0,
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
        civic: Default::default(),
    });
}

/// 推進田壟新苗抽長、聚落外林木再生與露天無保護生鮮之自然消解。
pub fn step_land_ecology(world: &mut RimWorldState, rt: Option<&SubstrateRuntime>) {
    let weather = super::rim_shelter::weather_at_unix(super::rim_sim::now_unix_sec());
    step_land_ecology_with_weather(world, rt, weather);
}

pub fn step_land_ecology_with_weather(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    weather: RimWeatherKind,
) {
    let step: u16 = if weather.is_raining() { 2 } else { 1 };

    if world.tick.is_multiple_of(20) {
        let keys: Vec<(i32, i32)> = world.overlays.keys().copied().collect();
        for pos in keys {
            if let Some(ov) = world.overlays.get_mut(&pos) {
                match ov.structure {
                    Some(StructureKind::HerbSeedling) => {
                        ov.eco_ticks = ov.eco_ticks.saturating_add(step);
                        if ov.eco_ticks >= 16 {
                            ov.structure = Some(StructureKind::HerbMature);
                            ov.eco_ticks = 0;
                            world.dirty_unsaved_ticks += 2;
                        }
                    }
                    Some(StructureKind::CropSeedling) => {
                        ov.eco_ticks = ov.eco_ticks.saturating_add(step);
                        if ov.eco_ticks >= 16 {
                            ov.structure = Some(StructureKind::CropMature);
                            ov.eco_ticks = 0;
                            world.dirty_unsaved_ticks += 2;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if world.tick.is_multiple_of(40) {
        let candidates: Vec<(i32, i32)> = world
            .overlays
            .iter()
            .filter_map(|(&pos, ov)| {
                (ov.cleared_natural
                    && ov.structure.is_none()
                    && ov.blueprint.is_none()
                    && ov.ground_item.is_none()
                    && !is_settlement_keep_clear(pos.0, pos.1))
                .then_some(pos)
            })
            .collect();

        for (mx, my) in candidates {
            let (_, nat) = derive_micro_pristine(rt, mx, my);
            if !matches!(
                nat,
                Some(NaturalProp::BambooClump | NaturalProp::BambooShoot | NaturalProp::WildHerb)
            ) {
                continue;
            }
            if world
                .actors
                .iter()
                .any(|a| a.hp > 0 && (a.px - mx as f32).hypot(a.py - my as f32) < 0.85)
            {
                continue;
            }
            let mut regrown = false;
            if let Some(ov) = world.overlays.get_mut(&(mx, my)) {
                ov.eco_ticks = ov.eco_ticks.saturating_add(step);
                if ov.eco_ticks >= 32 {
                    ov.cleared_natural = false;
                    ov.eco_ticks = 0;
                    regrown = ov.is_pristine();
                }
            }
            if regrown {
                world.overlays.remove(&(mx, my));
                world.dirty_unsaved_ticks += 2;
            }
        }
    }

    if world.tick.is_multiple_of(80) {
        let keys: Vec<(i32, i32)> = world
            .overlays
            .iter()
            .filter_map(|(&pos, ov)| {
                (ov.structure != Some(StructureKind::Stockpile)
                    && ov.ground_item.is_some_and(|g| {
                        g.count > 0
                            && matches!(
                                g.item,
                                RimItemKind::BeastMeat
                                    | RimItemKind::BambooShoot
                                    | RimItemKind::MountainHerb
                                    | RimItemKind::SweetTuber
                            )
                    }))
                .then_some(pos)
            })
            .collect();

        for pos in keys {
            let mut prune = false;
            if let Some(ov) = world.overlays.get_mut(&pos) {
                ov.eco_ticks = ov.eco_ticks.saturating_add(step);
                if ov.eco_ticks >= 24 {
                    ov.eco_ticks = 0;
                    if let Some(mut g) = ov.ground_item {
                        g.count = g.count.saturating_sub(1);
                        ov.ground_item = (g.count > 0).then_some(g);
                    }
                    prune = ov.is_pristine();
                    world.dirty_unsaved_ticks += 2;
                }
            }
            if prune {
                world.overlays.remove(&pos);
            }
        }
    }
}

/// 敷貼一劑`敷傷藥膏`，立即消解部位負傷減速並恢復氣血精力。
pub fn apply_wound_salve(actor: &mut RimActor) -> Result<&'static str, &'static str> {
    if actor.injury_ticks == 0 && actor.injury_part.is_none() && actor.hp >= actor.max_hp {
        return Err("氣血完好無傷，無需耗用藥膏");
    }
    if actor.take_inv(RimItemKind::WoundSalve, 1) == 0 {
        return Err("行囊中尚無敷傷藥膏");
    }
    actor.injury_ticks = 0;
    actor.injury_part = None;
    actor.hp = (actor.hp + 28).min(actor.max_hp);
    actor.stamina = actor.stamina.saturating_add(14).min(100);
    actor.activity_tag = "敷藥調息".into();
    actor.push_diary("敷貼敷傷藥膏，患處腫痛消散");
    Ok("清涼藥膏貼合患處，筋骨痛楚頓減")
}

/// 於工作台將一份`青草藥`搗製為`敷傷藥膏`。
pub fn complete_craft_salve(world: &mut RimWorldState, idx: usize, tx: i32, ty: i32) {
    let (ax, ay) = world.actors[idx].micro_pos();
    if (ax - tx).abs() > 1 || (ay - ty).abs() > 1 {
        return;
    }
    if world.overlays.get(&(tx, ty)).and_then(|o| o.structure) != Some(StructureKind::Workbench) {
        return;
    }
    let mut got = world.actors[idx].take_inv(RimItemKind::MountainHerb, 1);
    if got == 0
        && let Some((sx, sy)) = world.find_nearest_ground_item((tx, ty), RimItemKind::MountainHerb, false)
    {
        got = world.take_ground_item(sx, sy, RimItemKind::MountainHerb, 1);
    }
    if got > 0 {
        world.actors[idx].add_inv(RimItemKind::WoundSalve, 1);
        world.actors[idx].craft_forge = world.actors[idx].craft_forge.saturating_add(2);
        world.actors[idx].push_diary("於案台將青草藥搗製成敷傷藥膏");
        world.actors[idx].activity_tag = "搗製藥膏".into();
        world.actors[idx].pop_text = Some(("搗成敷傷藥膏".into(), world.tick + 12));
        let glyph = world.actors[idx].glyph.clone();
        world.push_event(format!("{glyph} 搗製好［敷傷藥膏］"));
    } else {
        world.actors[idx].pop_text = Some(("行囊或倉廩缺青草藥".into(), world.tick + 10));
    }
}

/// 結算抵達田壟格後的翻土、播種或採收，並自動串接同畦下一格（整畦單次決策不勞作）。
pub fn complete_farm_plot(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    tx: i32,
    ty: i32,
    op: FarmOp,
) {
    let aid = world.actors[idx].id;
    let (ax, ay) = world.actors[idx].micro_pos();
    if (ax - tx).abs() > 1 || (ay - ty).abs() > 1 {
        world.economy.parcel_queues.remove(&aid);
        world.actors[idx].task = PendingTask::None;
        return;
    }
    world.actors[idx].facing_rad = ((ty - ay) as f32).atan2((tx - ax) as f32);

    match op {
        FarmOp::Till => {
            if !world.actors[idx].equipped_tool.is_some_and(RimItemKind::is_pick) {
                let picked = [RimItemKind::AlloyPick, RimItemKind::StonePick]
                    .into_iter()
                    .find(|&p| world.actors[idx].take_inv(p, 1) > 0);
                if let Some(pick) = picked {
                    if let Some(prev) = world.actors[idx].equipped_tool.replace(pick) {
                        world.actors[idx].add_inv(prev, 1);
                    }
                } else {
                    world.actors[idx].pop_text = Some(("須持開山鎬方能翻土".into(), world.tick + 12));
                    world.economy.parcel_queues.remove(&aid);
                    world.actors[idx].task = PendingTask::None;
                    return;
                }
            }
            let (ground, _) = derive_micro_pristine(rt, tx, ty);
            let can_till = matches!(ground, MicroGround::LoamGrass | MicroGround::RedGravel)
                && world.overlays.get(&(tx, ty)).is_none_or(|o| o.structure.is_none() && o.blueprint.is_none());
            if can_till {
                let ov = world.overlays.entry((tx, ty)).or_default();
                ov.cleared_natural = true;
                ov.structure = Some(StructureKind::TilledRidge);
                ov.blueprint = None;
                ov.eco_ticks = 0;
                super::rim_economy::record_tool_wear(world, idx);
                world.actors[idx].craft_harvest = world.actors[idx].craft_harvest.saturating_add(1);
                world.actors[idx].push_diary("揮鎬翻鬆一畦旱田壟");
                world.actors[idx].pop_text = Some(("翻妥空田壟".into(), world.tick + 8));
                world.dirty_unsaved_ticks += 3;
            }
        }
        FarmOp::PlantHerb | FarmOp::PlantTuber => {
            if world.overlays.get(&(tx, ty)).and_then(|o| o.structure) == Some(StructureKind::TilledRidge) {
                let (seed_item, target_st, msg_ok) = if op == FarmOp::PlantHerb {
                    (RimItemKind::MountainHerb, StructureKind::HerbSeedling, "於田壟栽下青草藥苗")
                } else {
                    (RimItemKind::SweetTuber, StructureKind::CropSeedling, "於田壟扦插紅土甘薯藤")
                };
                if world.actors[idx].take_inv(seed_item, 1) == 0
                    && let Some((sx, sy)) = world.find_nearest_ground_item((tx, ty), seed_item, false)
                {
                    let _ = world.take_ground_item(sx, sy, seed_item, 1);
                }
                if let Some(ov) = world.overlays.get_mut(&(tx, ty)) {
                    ov.structure = Some(target_st);
                    ov.eco_ticks = 0;
                }
                world.actors[idx].craft_harvest = world.actors[idx].craft_harvest.saturating_add(1);
                world.actors[idx].push_diary(msg_ok);
                world.actors[idx].pop_text = Some(("播下新苗".into(), world.tick + 8));
                world.dirty_unsaved_ticks += 3;
            }
        }
        FarmOp::Harvest => {
            let st = world.overlays.get(&(tx, ty)).and_then(|o| o.structure);
            let yield_item = match st {
                Some(StructureKind::HerbMature) => Some(RimItemKind::MountainHerb),
                Some(StructureKind::CropMature) => Some(RimItemKind::SweetTuber),
                _ => None,
            };
            if let Some(item) = yield_item {
                if let Some(ov) = world.overlays.get_mut(&(tx, ty)) {
                    ov.structure = Some(StructureKind::TilledRidge);
                    ov.eco_ticks = 0;
                }
                // 收成直接入行囊（背簍），滿整畦後回倉廩一次結算歸倉，不在田上丟滿地
                world.actors[idx].add_inv(item, 2);
                world.actors[idx].craft_harvest = world.actors[idx].craft_harvest.saturating_add(2);
                world.actors[idx].push_diary(format!("在田壟收成{}", item.label()));
                world.actors[idx].pop_text = Some((format!("收得{}×2", item.short_char()), world.tick + 8));
                let glyph = world.actors[idx].glyph.clone();
                world.push_event(format!("{glyph} 收成［{}］", item.label()));
                world.dirty_unsaved_ticks += 3;
            }
        }
    }

    // 接續同畦佇列下一格（單次整畦施作）
    while let Some((nx, ny, next_op)) = world
        .economy
        .parcel_queues
        .get_mut(&aid)
        .and_then(VecDeque::pop_front)
    {
        if let Some(p) = find_micro_path(rt, &world.overlays, (ax, ay), (nx, ny), true, false) {
            world.actors[idx].path = p.into();
            world.actors[idx].task = PendingTask::FarmPlot {
                tx: nx,
                ty: ny,
                op: next_op,
            };
            world.actors[idx].activity_tag = "整畦接續".into();
            return;
        }
    }
    world.actors[idx].task = PendingTask::None;
    world.actors[idx].activity_tag = "整畦完畢".into();
}

/// 負傷時優先自行敷藥或赴案台搗製`敷傷藥膏`。
pub fn try_urgent_salve_heal(world: &mut RimWorldState, rt: Option<&SubstrateRuntime>, idx: usize) -> bool {
    let me = &world.actors[idx];
    if me.faction != ActorFaction::Settler || (me.injury_ticks == 0 && me.injury_part.is_none()) {
        return false;
    }
    let start = me.micro_pos();
    if apply_wound_salve(&mut world.actors[idx]).is_ok() {
        world.actors[idx].pop_text = Some(("敷膏治傷".into(), world.tick + 10));
        return true;
    }
    if let Some((sx, sy)) = world.find_nearest_ground_item(start, RimItemKind::WoundSalve, false)
        && (sx - start.0).abs() + (sy - start.1).abs() <= 24
        && let Some(p) = find_micro_path(rt, &world.overlays, start, (sx, sy), true, false)
    {
        world.actors[idx].path = p.into();
        world.actors[idx].task = PendingTask::PickupEquip { tx: sx, ty: sy };
        world.actors[idx].activity_tag = "取藥治傷".into();
        return true;
    }
    let has_herb = world.actors[idx].inventory.iter().any(|s| s.item == RimItemKind::MountainHerb && s.count > 0)
        || world
            .find_nearest_ground_item(start, RimItemKind::MountainHerb, false)
            .is_some_and(|pos| (pos.0 - start.0).abs() + (pos.1 - start.1).abs() <= 24);
    if has_herb
        && let Some(&(wx, wy)) = world
            .overlays
            .iter()
            .filter(|(_, ov)| ov.structure == Some(StructureKind::Workbench))
            .min_by_key(|&(&(cx, cy), _)| (cx - start.0).abs() + (cy - start.1).abs())
            .map(|(pos, _)| pos)
        && (wx - start.0).abs() + (wy - start.1).abs() <= 24
        && let Some(p) = find_micro_path(rt, &world.overlays, start, (wx, wy), true, false)
    {
        world.actors[idx].path = p.into();
        world.actors[idx].task = PendingTask::CraftAt {
            tx: wx,
            ty: wy,
            item: RimItemKind::WoundSalve,
        };
        world.actors[idx].activity_tag = "赴台搗藥".into();
        return true;
    }
    false
}

/// 填充農田／藥圃結構於右側詳情卡之自然白話標籤與單步整畦插座動作。
pub fn describe_ecology_structure(
    st: StructureKind,
    raining: bool,
    tags: &mut Vec<String>,
    sockets: &mut Vec<Value>,
) {
    match st {
        StructureKind::TilledRidge => {
            tags.extend(["鬆軟田壟".into(), "單步整畦播栽".into()]);
            sockets.push(json!({ "label": "整畦栽種青草藥", "action": "plant_herb" }));
            sockets.push(json!({ "label": "整畦扦插紅土甘薯", "action": "plant_tuber" }));
        }
        StructureKind::HerbSeedling => {
            tags.push(if raining { "雨露滋養・抽芽中" } else { "新苗抽長中" }.into());
            tags.push("成片藥圃畦".into());
            sockets.push(json!({ "label": "走近察看", "action": "move" }));
        }
        StructureKind::HerbMature => {
            tags.extend(["藥葉繁茂・可採收".into(), "採後留壟".into()]);
            sockets.push(json!({ "label": "整畦順手採收", "action": "harvest_plot" }));
        }
        StructureKind::CropSeedling => {
            tags.push(if raining { "雨露滋養・展藤中" } else { "蔓藤生長中" }.into());
            tags.push("成片甘薯畦".into());
            sockets.push(json!({ "label": "走近察看", "action": "move" }));
        }
        StructureKind::CropMature => {
            tags.extend(["塊根飽滿・可採收".into(), "採後留壟".into()]);
            sockets.push(json!({ "label": "整畦採挖甘薯", "action": "harvest_plot" }));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn eco_actor(id: usize, tool: Option<RimItemKind>, inv: Vec<GroundStack>) -> RimActor {
        RimActor {
            id,
            code: format!("e_{id}"),
            actor_key: format!("e_{id}"),
            entity_id: Some("qa".into()),
            glyph: "農".into(),
            name: "驗農".into(),
            gender: "M".into(),
            soul_seed: 11,
            role_tag: "測試".into(),
            faction: ActorFaction::Settler,
            is_player: true,
            autopilot: false,
            running: false,
            px: 1.0,
            py: -4.0,
            facing_rad: 0.0,
            hp: 65,
            max_hp: 100,
            equipped_tool: tool,
            carrying: None,
            inventory: inv,
            directive: "forage".into(),
            path: VecDeque::new(),
            task: PendingTask::None,
            activity_tag: String::new(),
            pop_text: None,
            cooldown_ticks: 0,
            last_active_unix: 0,
            hunger: 80,
            thirst: 80,
            stamina: 60,
            magnesium: 18,
            equipped_hat: true,
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
            civic: Default::default(),
        }
    }

    fn eco_world(overlays: HashMap<(i32, i32), MicroCellOverlay>, actors: Vec<RimActor>) -> RimWorldState {
        RimWorldState {
            tick: 0,
            overlays,
            actors,
            events: VecDeque::new(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
            economy: Default::default(),
        }
    }

    #[test]
    fn rain_accelerates_seedling_growth_and_harvest_preserves_tilled_ridge() {
        let mut ov = HashMap::new();
        ov.insert((2, -4), MicroCellOverlay::cleared(Some(StructureKind::HerbSeedling)));
        ov.insert((3, -4), MicroCellOverlay::cleared(Some(StructureKind::CropSeedling)));
        let actor = eco_actor(1, None, Vec::new());
        let mut w_rain = eco_world(ov.clone(), vec![actor.clone()]);
        let mut w_clear = eco_world(ov, vec![actor]);

        for t in 1..=160 {
            w_rain.tick = t * 20;
            w_clear.tick = t * 20;
            if t <= 8 {
                step_land_ecology_with_weather(&mut w_rain, None, RimWeatherKind::SummerSquall);
                step_land_ecology_with_weather(&mut w_clear, None, RimWeatherKind::Clear);
            }
        }
        assert_eq!(w_rain.overlays[&(2, -4)].structure, Some(StructureKind::HerbMature));
        assert_eq!(w_rain.overlays[&(3, -4)].structure, Some(StructureKind::CropMature));
        assert_eq!(w_clear.overlays[&(2, -4)].structure, Some(StructureKind::HerbSeedling));

        complete_farm_plot(&mut w_rain, None, 0, 2, -4, FarmOp::Harvest);
        assert_eq!(w_rain.overlays[&(2, -4)].structure, Some(StructureKind::TilledRidge));
        assert_eq!(w_rain.actors[0].take_inv(RimItemKind::MountainHerb, 10), 2);
    }

    #[test]
    fn chopped_wild_bamboo_and_herb_naturally_sprout_back_and_prune_overlay() {
        let mut target = None;
        for mx in -32..=32 {
            for my in -32..=32 {
                if is_settlement_keep_clear(mx, my) {
                    continue;
                }
                if matches!(
                    derive_micro_pristine(None, mx, my).1,
                    Some(NaturalProp::BambooClump | NaturalProp::WildHerb)
                ) {
                    target = Some((mx, my));
                    break;
                }
            }
            if target.is_some() {
                break;
            }
        }
        let pos = target.expect("wild bamboo or herb outside keep-clear");
        let mut ov = HashMap::new();
        ov.insert(pos, MicroCellOverlay::cleared(None));
        let mut w = eco_world(ov, Vec::new());

        for t in 1..=16 {
            w.tick = t * 40;
            step_land_ecology_with_weather(&mut w, None, RimWeatherKind::SummerSquall);
        }
        assert!(!w.overlays.contains_key(&pos), "regrown cell must revert pristine and prune");
    }

    #[test]
    fn wound_salve_crafting_and_application_clears_anatomical_injury() {
        let mut ov = HashMap::new();
        ov.insert((0, -4), MicroCellOverlay::cleared(Some(StructureKind::Workbench)));
        let mut a = eco_actor(
            1,
            None,
            vec![GroundStack {
                item: RimItemKind::MountainHerb,
                count: 1,
            }],
        );
        a.injury_part = Some("右臂筋絡震傷".into());
        a.injury_ticks = 90;
        let mut w = eco_world(ov, vec![a]);

        complete_craft_salve(&mut w, 0, 0, -4);
        assert!(apply_wound_salve(&mut w.actors[0]).is_ok());
        assert_eq!(w.actors[0].injury_ticks, 0);
        assert!(w.actors[0].injury_part.is_none());
        assert_eq!(w.actors[0].hp, 93);
    }

    #[test]
    fn till_ridge_requires_pick_and_auto_equips_from_bag() {
        let mut a_no_pick = eco_actor(1, Some(RimItemKind::BambooBlade), Vec::new());
        a_no_pick.px = 2.0;
        a_no_pick.py = -4.0;
        let mut w1 = eco_world(HashMap::new(), vec![a_no_pick]);
        complete_farm_plot(&mut w1, None, 0, 2, -5, FarmOp::Till);
        assert!(!w1.overlays.contains_key(&(2, -5)));

        let mut a_with_pick = eco_actor(
            2,
            Some(RimItemKind::BambooBlade),
            vec![GroundStack {
                item: RimItemKind::StonePick,
                count: 1,
            }],
        );
        a_with_pick.px = 2.0;
        a_with_pick.py = -4.0;
        let mut w2 = eco_world(HashMap::new(), vec![a_with_pick]);
        complete_farm_plot(&mut w2, None, 0, 2, -5, FarmOp::Till);
        assert_eq!(
            w2.overlays.get(&(2, -5)).and_then(|o| o.structure),
            Some(StructureKind::TilledRidge)
        );
        assert_eq!(w2.actors[0].equipped_tool, Some(RimItemKind::StonePick));
        assert_eq!(w2.actors[0].take_inv(RimItemKind::BambooBlade, 2), 1);
    }

    fn meat_in(ov: &HashMap<(i32, i32), MicroCellOverlay>, in_stock: bool) -> u32 {
        ov.values()
            .filter(|o| (o.structure == Some(StructureKind::Stockpile)) == in_stock)
            .filter_map(|o| o.ground_item)
            .filter(|g| g.item == RimItemKind::BeastMeat)
            .map(|g| u32::from(g.count))
            .sum()
    }

    fn stack_cell(st: Option<StructureKind>, item: RimItemKind, count: u16) -> MicroCellOverlay {
        let mut cell = MicroCellOverlay::cleared(st);
        cell.ground_item = Some(GroundStack { item, count });
        cell
    }

    #[test]
    fn real_scale_relocation_preserves_goods_and_parcel_enqueues_contiguous_bed() {
        let mut ov = HashMap::new();
        ov.insert((-4, -3), stack_cell(Some(StructureKind::Stockpile), RimItemKind::BeastMeat, 5));
        assert!(reconcile_real_scale_layout_and_clutter(None, &mut ov));
        assert!(ov.get(&(-4, -3)).and_then(|o| o.structure).is_none());
        assert_eq!(
            ov.get(&(-12, 4)).and_then(|o| o.structure),
            Some(StructureKind::Stockpile)
        );
        assert_eq!(meat_in(&ov, true), 5);
        assert_eq!(meat_in(&ov, false), 0);

        let a = eco_actor(1, None, Vec::new());
        let mut w = eco_world(ov, vec![a]);
        enqueue_contiguous_parcel(&mut w, 0, -19, -15, FarmOp::Harvest);
        assert_eq!(w.economy.parcel_queues.get(&1).map_or(0, VecDeque::len), 4);
    }
}

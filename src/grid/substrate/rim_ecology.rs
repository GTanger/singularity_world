//! 土地動態演變（持鎬翻土開壟、青草藥圃／紅土甘薯旱田栽收、敷傷藥膏治傷與山林雨露再生）模組。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::rim_shelter::RimWeatherKind;
use super::rim_sim::{ActorFaction, PendingTask, RimActor, RimWorldState};
use super::rim_world::{
    GroundStack, MicroCellOverlay, MicroGround, NaturalProp, RimItemKind, StructureKind,
    derive_micro_pristine, find_micro_path, find_micro_path_crowded,
};
use super::runtime::SubstrateRuntime;

/// 田壟農事單一步驟（翻土、播種藥苗、扦插甘薯、採收留壟）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmOp {
    Till,
    PlantHerb,
    PlantTuber,
    Harvest,
}

/// 聚落廣場與雙聚落步道淨空帶（林木再生時避開此範圍以免阻路）。
pub fn is_settlement_keep_clear(mx: i32, my: i32) -> bool {
    ((-6..=12).contains(&mx) && (-6..=6).contains(&my))
        || (mx == 0 && (3..=18).contains(&my))
        || ((6..=28).contains(&mx) && (-8..=-2).contains(&my))
        || ((25..=37).contains(&mx) && (-11..=-3).contains(&my))
}

/// 冪等於既有西麓湧泉與水尾寮向陽地鋪設示範藥圃／甘薯旱田，並於空儲位補入初期青草藥。
pub fn upgrade_ecology_plots(
    rt: Option<&SubstrateRuntime>,
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
) -> bool {
    if overlays
        .values()
        .any(|o| o.structure.is_some_and(StructureKind::is_garden_plot))
    {
        return false;
    }
    let mut changed = false;
    let plots = [
        ((2, -4), StructureKind::HerbMature),
        ((3, -4), StructureKind::HerbSeedling),
        ((2, -5), StructureKind::CropMature),
        ((3, -5), StructureKind::TilledRidge),
        ((26, -6), StructureKind::HerbMature),
        ((26, -5), StructureKind::CropMature),
    ];
    for (pos, st) in plots {
        let (g, _) = derive_micro_pristine(rt, pos.0, pos.1);
        if g.is_water() || g == MicroGround::SteepCliff {
            continue;
        }
        let e = overlays.entry(pos).or_default();
        if e.structure.is_none() && e.blueprint.is_none() && e.ground_item.is_none() {
            e.cleared_natural = true;
            e.structure = Some(st);
            e.eco_ticks = 0;
            changed = true;
        }
    }
    if let Some(slot) = overlays.get_mut(&(-3, -2))
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

/// 推進田壟新苗抽長與聚落外已砍伐林木／藥草之雨露自然再生（實時依天氣調速）。
pub fn step_land_ecology(world: &mut RimWorldState, rt: Option<&SubstrateRuntime>) {
    let weather = super::rim_shelter::weather_at_unix(super::rim_sim::now_unix_sec());
    step_land_ecology_with_weather(world, rt, weather);
}

/// 可指定天氣之生態演替步進（山雨加倍催芽，滿額且無建築佔位時令覆寫格回歸原生並由資料庫自清）。
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
                Some(
                    NaturalProp::BambooClump | NaturalProp::BambooShoot | NaturalProp::WildHerb
                )
            ) {
                continue;
            }
            let occupied = world
                .actors
                .iter()
                .any(|a| a.hp > 0 && (a.px - mx as f32).hypot(a.py - my as f32) < 0.85);
            if occupied {
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

    // 露天未入儲物墊之生鮮肉果隨日曬雨淋／山鳥叼食自然消解（儲物墊內妥善存放者不損）
    if world.tick.is_multiple_of(80) {
        let perishable_keys: Vec<(i32, i32)> = world
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

        for pos in perishable_keys {
            let mut should_prune = false;
            if let Some(ov) = world.overlays.get_mut(&pos) {
                ov.eco_ticks = ov.eco_ticks.saturating_add(step);
                if ov.eco_ticks >= 24 {
                    ov.eco_ticks = 0;
                    if let Some(mut g) = ov.ground_item {
                        g.count = g.count.saturating_sub(1);
                        ov.ground_item = (g.count > 0).then_some(g);
                    }
                    should_prune = ov.is_pristine();
                    world.dirty_unsaved_ticks += 2;
                }
            }
            if should_prune {
                world.overlays.remove(&pos);
            }
        }
    }
}

/// 統計全聚落（地面、儲物墊與所有居民行囊手抱）某物資總量。
pub fn item_total_in_world(world: &RimWorldState, kind: RimItemKind) -> u32 {
    let ground: u32 = world
        .overlays
        .values()
        .filter_map(|ov| ov.ground_item)
        .filter(|g| g.item == kind)
        .map(|g| u32::from(g.count))
        .sum();
    let carried: u32 = world
        .actors
        .iter()
        .filter(|a| a.faction == ActorFaction::Settler)
        .map(|a| {
            let inv: u32 = a
                .inventory
                .iter()
                .filter(|s| s.item == kind)
                .map(|s| u32::from(s.count))
                .sum();
            let carry = a
                .carrying
                .filter(|c| c.item == kind)
                .map_or(0, |c| u32::from(c.count));
            inv + carry
        })
        .sum();
    ground + carried
}

/// 冪等擴充西麓 `2×4` 儲物席與水尾寮儲物格，並將既有滿地陳舊散落生鮮最多整齊歸倉 6 份後清除餘積。
pub fn reconcile_larder_and_loose_clutter(
    rt: Option<&SubstrateRuntime>,
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
) -> bool {
    let mut changed = false;
    let extra_stockpiles = [(-4, -3), (-3, -3), (-4, -4), (-3, -4), (33, -8)];
    for pos in extra_stockpiles {
        let (g, _) = derive_micro_pristine(rt, pos.0, pos.1);
        if g.is_water() || g == MicroGround::SteepCliff {
            continue;
        }
        let e = overlays.entry(pos).or_default();
        if e.structure.is_none() && e.blueprint.is_none() {
            e.cleared_natural = true;
            e.structure = Some(StructureKind::Stockpile);
            e.eco_ticks = 0;
            changed = true;
        }
    }

    let perishables = [
        RimItemKind::BeastMeat,
        RimItemKind::BambooShoot,
        RimItemKind::MountainHerb,
        RimItemKind::SweetTuber,
    ];
    let loose_positions: Vec<((i32, i32), RimItemKind)> = overlays
        .iter()
        .filter_map(|(&pos, ov)| {
            (ov.structure != Some(StructureKind::Stockpile))
                .then_some(ov.ground_item)
                .flatten()
                .filter(|g| g.count > 0 && perishables.contains(&g.item))
                .map(|g| (pos, g.item))
        })
        .collect();
    if loose_positions.is_empty() {
        return changed;
    }

    let mut stock_counts: HashMap<RimItemKind, u16> = HashMap::new();
    for ov in overlays.values() {
        if ov.structure == Some(StructureKind::Stockpile)
            && let Some(g) = ov.ground_item
        {
            *stock_counts.entry(g.item).or_insert(0) += g.count;
        }
    }

    for (pos, item) in loose_positions {
        let Some(ov) = overlays.get_mut(&pos) else {
            continue;
        };
        let taken_count = ov.ground_item.take().map_or(0, |g| g.count);
        ov.eco_ticks = 0;
        let prune = ov.is_pristine();
        if prune {
            overlays.remove(&pos);
        }
        changed = true;

        let cur = *stock_counts.get(&item).unwrap_or(&0);
        let allow = 6_u16.saturating_sub(cur).min(taken_count);
        if allow > 0 {
            let slot_pos = overlays
                .iter()
                .filter(|(_, o)| {
                    o.structure == Some(StructureKind::Stockpile)
                        && o.ground_item
                            .is_none_or(|g| g.item == item && g.count + allow <= 20)
                })
                .min_by_key(|&(&(sx, sy), ref o)| {
                    let match_bias = if o.ground_item.is_some_and(|g| g.item == item) { 0 } else { 100 };
                    match_bias + (sx + 4).abs() + (sy + 2).abs()
                })
                .map(|(&p, _)| p);
            if let Some(sp) = slot_pos
                && let Some(dst) = overlays.get_mut(&sp)
            {
                match &mut dst.ground_item {
                    None => {
                        dst.ground_item = Some(GroundStack { item, count: allow });
                    }
                    Some(g) if g.item == item => {
                        g.count = g.count.saturating_add(allow);
                    }
                    _ => {}
                }
                *stock_counts.entry(item).or_insert(0) += allow;
            }
        }
    }
    changed
}

/// 聚落迎擊護營與按需出獵（肉食充裕時只守聚落周遭，不無止境遠出宰殺）。
pub fn try_guard_or_hunt_beast(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    start: (i32, i32),
    home_pt: (i32, i32),
) -> bool {
    let dir_now = world.actors[idx].directive.as_str();
    let meat_stock = item_total_in_world(world, RimItemKind::BeastMeat)
        + item_total_in_world(world, RimItemKind::RoastedMeat);
    let beast_guard_radius: f32 = match dir_now {
        "guard" if meat_stock < 8 => 24.0,
        "guard" => 11.5,
        "rest" => 5.0,
        _ => 9.0,
    };
    if let Some(beast) = world.actors.iter().find(|b| {
        b.faction == ActorFaction::WildBeast
            && b.hp > 0
            && (b.px - home_pt.0 as f32).hypot(b.py - home_pt.1 as f32) <= beast_guard_radius
    }) {
        let b_id = beast.id;
        let b_pos = beast.micro_pos();
        if let Some(p) = find_micro_path(rt, &world.overlays, start, b_pos, true, false) {
            world.actors[idx].path = p.into();
            world.actors[idx].task = PendingTask::ChaseTarget { target_id: b_id };
            world.actors[idx].activity_tag = "迎擊護營".into();
            return true;
        }
    }
    false
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

/// 於工作台將一份`青草藥`搗製為`敷傷藥膏`（嚴格物質守恆）。
pub fn complete_craft_salve(world: &mut RimWorldState, idx: usize, tx: i32, ty: i32) {
    let (ax, ay) = world.actors[idx].micro_pos();
    if (ax - tx).abs() > 1 || (ay - ty).abs() > 1 {
        return;
    }
    let has_bench = world
        .overlays
        .get(&(tx, ty))
        .and_then(|o| o.structure)
        == Some(StructureKind::Workbench);
    if !has_bench {
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
        world.actors[idx].pop_text = Some(("行囊或儲物墊缺青草藥".into(), world.tick + 10));
    }
}

/// 結算抵達田壟／菜圃格後的翻土、播種或採收（採罷保留田壟以免瑣碎重複翻土）。
pub fn complete_farm_plot(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    tx: i32,
    ty: i32,
    op: FarmOp,
) {
    let (ax, ay) = world.actors[idx].micro_pos();
    if (ax - tx).abs() > 1 || (ay - ty).abs() > 1 {
        world.actors[idx].task = PendingTask::None;
        return;
    }
    world.actors[idx].facing_rad = ((ty - ay) as f32).atan2((tx - ax) as f32);

    match op {
        FarmOp::Till => {
            if world.actors[idx].equipped_tool != Some(RimItemKind::StonePick) {
                if world.actors[idx].take_inv(RimItemKind::StonePick, 1) > 0 {
                    if let Some(prev) = world.actors[idx].equipped_tool.replace(RimItemKind::StonePick) {
                        world.actors[idx].add_inv(prev, 1);
                    }
                } else {
                    world.actors[idx].pop_text =
                        Some(("須持石刃木鎬方能翻土".into(), world.tick + 12));
                    world.actors[idx].task = PendingTask::None;
                    return;
                }
            }
            let (ground, _) = derive_micro_pristine(rt, tx, ty);
            let can_till = matches!(ground, MicroGround::LoamGrass | MicroGround::RedGravel)
                && world
                    .overlays
                    .get(&(tx, ty))
                    .is_none_or(|o| o.structure.is_none() && o.blueprint.is_none());
            if can_till {
                let ov = world.overlays.entry((tx, ty)).or_default();
                ov.cleared_natural = true;
                ov.structure = Some(StructureKind::TilledRidge);
                ov.blueprint = None;
                ov.eco_ticks = 0;
                world.actors[idx].craft_harvest =
                    world.actors[idx].craft_harvest.saturating_add(1);
                world.actors[idx].push_diary("揮鎬翻鬆一畦旱田壟");
                world.actors[idx].activity_tag = "翻妥田壟".into();
                world.actors[idx].pop_text = Some(("翻妥空田壟".into(), world.tick + 10));
                world.dirty_unsaved_ticks += 4;
            }
        }
        FarmOp::PlantHerb | FarmOp::PlantTuber => {
            if world
                .overlays
                .get(&(tx, ty))
                .and_then(|o| o.structure)
                == Some(StructureKind::TilledRidge)
            {
                let (seed_item, target_st, msg_ok) = if op == FarmOp::PlantHerb {
                    (
                        RimItemKind::MountainHerb,
                        StructureKind::HerbSeedling,
                        "於田壟栽下青草藥苗",
                    )
                } else {
                    (
                        RimItemKind::SweetTuber,
                        StructureKind::CropSeedling,
                        "於田壟扦插紅土甘薯藤",
                    )
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
                world.actors[idx].craft_harvest =
                    world.actors[idx].craft_harvest.saturating_add(1);
                world.actors[idx].push_diary(msg_ok);
                world.actors[idx].activity_tag = "播栽完畢".into();
                world.actors[idx].pop_text = Some(("播下新苗".into(), world.tick + 10));
                world.dirty_unsaved_ticks += 4;
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
                let bag_cnt: u16 = world.actors[idx]
                    .inventory
                    .iter()
                    .filter(|s| s.item == item)
                    .map(|s| s.count)
                    .sum();
                if (world.actors[idx].entity_id.is_some() && !world.actors[idx].autopilot)
                    || bag_cnt < 4
                {
                    world.actors[idx].add_inv(item, 2);
                } else {
                    world.add_ground_item(tx, ty, item, 2);
                }
                world.actors[idx].craft_harvest =
                    world.actors[idx].craft_harvest.saturating_add(2);
                world.actors[idx].push_diary(format!("在田壟採收{}", item.label()));
                world.actors[idx].activity_tag = "採收完畢".into();
                world.actors[idx].pop_text =
                    Some((format!("收得{}×2", item.short_char()), world.tick + 10));
                let glyph = world.actors[idx].glyph.clone();
                world.push_event(format!("{glyph} 收成［{}］", item.label()));
                world.dirty_unsaved_ticks += 4;
            }
        }
    }
    world.actors[idx].task = PendingTask::None;
}

/// 自主居民／離線代理負傷時優先敷用或前往案台搗製`敷傷藥膏`。
pub fn try_urgent_salve_heal(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
) -> bool {
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
    let has_herb = world.actors[idx]
        .inventory
        .iter()
        .any(|s| s.item == RimItemKind::MountainHerb && s.count > 0)
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

/// 空閒同伴自主照料聚落附近之成熟圃畦與空置田壟（採收並補種）。
pub fn try_autonomous_farming(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    start: (i32, i32),
    home: (i32, i32),
    claimed: &[(i32, i32)],
    occ: &[(i32, i32)],
) -> bool {
    if !matches!(
        world.actors[idx].directive.as_str(),
        "" | "forage" | "craft" | "assist" | "自主巡採"
    ) || !world.tick.is_multiple_of(3)
    {
        return false;
    }
    let allow_herb = item_total_in_world(world, RimItemKind::MountainHerb) < 12;
    let allow_tuber = item_total_in_world(world, RimItemKind::SweetTuber)
        + item_total_in_world(world, RimItemKind::RoastedTuber)
        < 12;
    if let Some((&pos, _)) = world
        .overlays
        .iter()
        .filter(|&(&pos, ov)| {
            let want = match ov.structure {
                Some(StructureKind::HerbMature) => allow_herb,
                Some(StructureKind::CropMature) => allow_tuber,
                _ => false,
            };
            want && !claimed.contains(&pos)
                && (pos.0 - home.0).abs() + (pos.1 - home.1).abs() <= 24
        })
        .min_by_key(|&(&pos, _)| (pos.0 - start.0).abs() + (pos.1 - start.1).abs())
        && let Some(p) =
            find_micro_path_crowded(rt, &world.overlays, start, pos, true, false, occ)
    {
        world.actors[idx].path = p.into();
        world.actors[idx].task = PendingTask::FarmPlot {
            tx: pos.0,
            ty: pos.1,
            op: FarmOp::Harvest,
        };
        world.actors[idx].activity_tag = "採收圃畦".into();
        return true;
    }
    if let Some((&pos, _)) = world
        .overlays
        .iter()
        .filter(|&(&pos, ov)| {
            ov.structure == Some(StructureKind::TilledRidge)
                && !claimed.contains(&pos)
                && (pos.0 - home.0).abs() + (pos.1 - home.1).abs() <= 24
        })
        .min_by_key(|&(&pos, _)| (pos.0 - start.0).abs() + (pos.1 - start.1).abs())
        && let Some(p) =
            find_micro_path_crowded(rt, &world.overlays, start, pos, true, false, occ)
    {
        let op = if (pos.0 + pos.1).rem_euclid(2) == 0 {
            FarmOp::PlantHerb
        } else {
            FarmOp::PlantTuber
        };
        world.actors[idx].path = p.into();
        world.actors[idx].task = PendingTask::FarmPlot {
            tx: pos.0,
            ty: pos.1,
            op,
        };
        world.actors[idx].activity_tag = "補栽新苗".into();
        return true;
    }
    false
}

/// 填充農田／藥圃結構於右側詳情卡之自然白話標籤與單步插座動作。
pub fn describe_ecology_structure(
    st: StructureKind,
    raining: bool,
    tags: &mut Vec<String>,
    sockets: &mut Vec<Value>,
) {
    match st {
        StructureKind::TilledRidge => {
            tags.extend(["鬆軟田壟".into(), "可播種".into()]);
            sockets.push(json!({ "label": "栽種青草藥苗", "action": "plant_herb" }));
            sockets.push(json!({ "label": "扦插紅土甘薯", "action": "plant_tuber" }));
        }
        StructureKind::HerbSeedling => {
            tags.push(if raining { "雨露滋養・抽芽中" } else { "新苗抽長中" }.into());
            tags.push("藥材圃畦".into());
            sockets.push(json!({ "label": "走近察看", "action": "move" }));
        }
        StructureKind::HerbMature => {
            tags.extend(["藥葉繁茂・可採收".into(), "採後留壟".into()]);
            sockets.push(json!({ "label": "採收青草藥", "action": "harvest_plot" }));
        }
        StructureKind::CropSeedling => {
            tags.push(if raining { "雨露滋養・展藤中" } else { "蔓藤生長中" }.into());
            tags.push("旱田塊根".into());
            sockets.push(json!({ "label": "走近察看", "action": "move" }));
        }
        StructureKind::CropMature => {
            tags.extend(["塊根飽滿・可採收".into(), "採後留壟".into()]);
            sockets.push(json!({ "label": "採挖紅土甘薯", "action": "harvest_plot" }));
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
    fn larder_expansion_demand_cap_and_outdoor_scavenging_prevent_meat_flood() {
        let mut ov = HashMap::new();
        ov.insert((-4, -1), stack_cell(Some(StructureKind::Stockpile), RimItemKind::BambooPole, 5));
        for x in -10..=-6 {
            ov.insert((x, -4), stack_cell(None, RimItemKind::BeastMeat, 3));
        }
        assert!(reconcile_larder_and_loose_clutter(None, &mut ov));
        assert_eq!(ov.get(&(-4, -3)).and_then(|o| o.structure), Some(StructureKind::Stockpile));
        assert_eq!(meat_in(&ov, false), 0);
        assert_eq!(meat_in(&ov, true), 6);

        let mut guard = eco_actor(1, Some(RimItemKind::BambooBlade), Vec::new());
        guard.directive = "guard".into();
        guard.px = 0.0;
        guard.py = 0.0;
        let mut boar = eco_actor(2, None, Vec::new());
        boar.faction = ActorFaction::WildBeast;
        boar.is_player = false;
        boar.entity_id = None;
        boar.px = 18.0;
        boar.py = 0.0;
        ov.insert((-3, -3), stack_cell(Some(StructureKind::Stockpile), RimItemKind::BeastMeat, 4));
        let mut w = eco_world(ov, vec![guard, boar]);
        assert!(!try_guard_or_hunt_beast(&mut w, None, 0, (0, 0), (0, 0)));

        w.overlays.insert((5, -5), stack_cell(None, RimItemKind::BeastMeat, 1));
        for t in 1..=12 {
            w.tick = t * 80;
            step_land_ecology_with_weather(&mut w, None, RimWeatherKind::SummerSquall);
        }
        assert!(w.overlays.get(&(5, -5)).and_then(|o| o.ground_item).is_none());
        assert_eq!(meat_in(&w.overlays, true), 10);
    }
}

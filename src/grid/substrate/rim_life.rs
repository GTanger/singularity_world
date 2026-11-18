//! 活態生理需求、去數值化體感轉譯、炊食穿戴與「鎂」物質守恆交換模組。
//!
//! 嚴格落實設計核心：
//! - 後台由嚴格數值 (`hunger`/`thirst`/`stamina`/`magnesium`) 推進並保證物質守恆；
//! - 前台一律轉譯為自然台灣白話體感標籤，絕不外露原始數值與加減分數。

use std::collections::HashMap;

use super::rim_sim::{ActorFaction, PendingTask, RimActor, RimWorldState};
use super::rim_world::{
    MicroCellOverlay, MicroGround, NaturalProp, RimItemKind, StructureKind, derive_micro_pristine,
    find_micro_path,
};
use super::runtime::SubstrateRuntime;

/// 載入既有存檔時，清理早期遊走逸失至遠處野地的零星覆寫格並將走遠居民召回各自聚落。
pub fn recover_stranded_world_state(
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
    actors: &mut [RimActor],
) -> bool {
    let prev_ov_len = overlays.len();
    overlays.retain(|&(mx, my), ov| {
        ((-50..=60).contains(&mx) && (-45..=45).contains(&my))
            || ov.structure.is_some()
            || ov.blueprint.is_some()
    });
    let mut changed = overlays.len() != prev_ov_len;
    for a in actors {
        if a.faction == ActorFaction::Settler
            && (a.px < -50.0 || a.px > 60.0 || a.py < -45.0 || a.py > 45.0)
        {
            let (nx, ny) = match a.name.as_str() {
                "周阿茶" => (30.0, -7.0),
                "鄭石頭" => (33.0, -6.0),
                "林阿木" => (0.8, 0.8),
                _ => (1.5, -1.0),
            };
            a.px = nx;
            a.py = ny;
            a.path.clear();
            a.running = false;
            a.task = PendingTask::None;
            a.activity_tag = "歸營歇息".into();
            changed = true;
        }
    }
    changed
}

pub fn hunger_tag(hunger: u8) -> &'static str {
    match hunger {
        70..=255 => "腹中溫飽",
        40..=69 => "腹中微飢",
        18..=39 => "飢腸轆轆",
        _ => "飢疲交迫",
    }
}

pub fn thirst_tag(thirst: u8) -> &'static str {
    match thirst {
        70..=255 => "喉間潤澤",
        40..=69 => "口乾微渴",
        18..=39 => "唇乾舌燥",
        _ => "渴極嗓乾",
    }
}

pub fn stamina_tag(stamina: u8) -> &'static str {
    match stamina {
        70..=255 => "神采奕奕",
        40..=69 => "氣息微促",
        18..=39 => "略顯疲態",
        _ => "步履沉重",
    }
}

pub fn magnesium_tag(magnesium: u32) -> &'static str {
    match magnesium {
        0 => "囊中無鎂",
        1..=10 => "碎鎂少許",
        11..=35 => "鎂晶數枚",
        _ => "鎂晶充裕",
    }
}

/// 推算某格與周遭 1 格內是否臨近湧泉或溪流。
pub fn find_nearby_water(
    rt: Option<&SubstrateRuntime>,
    origin: (i32, i32),
    radius: i32,
) -> Option<(i32, i32)> {
    let mut best: Option<((i32, i32), i32)> = None;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let nx = origin.0 + dx;
            let ny = origin.1 + dy;
            let (g, _) = derive_micro_pristine(rt, nx, ny);
            if matches!(g, MicroGround::ShallowStream | MicroGround::SpringPool) {
                let d = dx.abs() + dy.abs();
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some(((nx, ny), d));
                }
            }
        }
    }
    best.map(|(p, _)| p)
}

/// 每 24 ticks（約 6 秒）推進場上人類角色的自然代謝、圍爐回氣與極端飢渴疲勞。
pub fn step_settler_metabolism(world: &mut RimWorldState) {
    if !world.tick.is_multiple_of(24) {
        return;
    }
    let weather = super::rim_shelter::weather_at_unix(super::rim_sim::now_unix_sec());
    let sheltered = super::rim_shelter::sheltered_cell_set(&world.overlays);
    for idx in 0..world.actors.len() {
        if world.actors[idx].faction != ActorFaction::Settler || world.actors[idx].hp <= 0 {
            continue;
        }
        let (ax, ay) = world.actors[idx].micro_pos();
        let near_fire = world.overlays.iter().any(|(&(fx, fy), ov)| {
            ov.structure == Some(StructureKind::Campfire)
                && (fx - ax).abs() <= 4
                && (fy - ay).abs() <= 4
        });
        let near_dry_fire = world.overlays.iter().any(|(&(fx, fy), ov)| {
            ov.structure == Some(StructureKind::Campfire)
                && (fx - ax).abs() <= 4
                && (fy - ay).abs() <= 4
                && (!weather.is_raining() || sheltered.contains(&(fx, fy)))
        });
        let moving = !world.actors[idx].path.is_empty();
        let running = world.actors[idx].running;
        let hat_shield = if world.actors[idx].equipped_hat {
            0_u8
        } else {
            1_u8
        };

        let a = &mut world.actors[idx];
        // 自然代謝緩降，極緩慢推進不製造煩躁勞作
        if world.tick.is_multiple_of(72) {
            a.hunger = a.hunger.saturating_sub(1);
            a.thirst = a.thirst.saturating_sub(1 + u8::from(running));
        } else if running {
            a.thirst = a.thirst.saturating_sub(1);
        }

        if near_fire && !moving {
            let gain = if near_dry_fire { 12 } else { 5 };
            a.stamina = a.stamina.saturating_add(gain).min(100);
        } else if running {
            a.stamina = a.stamina.saturating_sub(3 + hat_shield);
        } else if moving {
            a.stamina = a.stamina.saturating_sub(1);
        } else {
            a.stamina = a.stamina.saturating_add(3).min(100);
        }

        // 極度飢渴僅略抑體力回復，不憑空暴斃
        if (a.hunger < 12 || a.thirst < 12) && world.tick.is_multiple_of(96) && a.hp > 20 {
            a.hp = a.hp.saturating_sub(1);
        }
    }
    super::rim_shelter::step_weather_and_bed_recovery(world, weather);
}

/// 空閒同伴／離線自主角色優先照顧自身飲水、炊食與歇息。
pub fn try_satisfy_urgent_needs(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
) -> bool {
    let start = world.actors[idx].micro_pos();

    // 1. 口乾微渴時主動尋溪掬水
    if world.actors[idx].thirst < 42
        && let Some(water) = find_nearby_water(rt, start, 22)
    {
        for (ox, oy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
            let bank = (water.0 + ox, water.1 + oy);
            let (bg, _) = derive_micro_pristine(rt, bank.0, bank.1);
            if !bg.is_water()
                && bg != MicroGround::SteepCliff
                && let Some(p) = find_micro_path(rt, &world.overlays, start, bank, true, false)
            {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::DrinkAt {
                    tx: water.0,
                    ty: water.1,
                };
                world.actors[idx].activity_tag = "前往飲泉".into();
                return true;
            }
        }
    }

    // 2. 腹中微飢時進食、取近處熟食、就火煨烤、拾近處食材或就近採筍（錨定營火 32m 內）
    if world.actors[idx].hunger < 45 {
        for dish in [
            RimItemKind::RoastedMeat,
            RimItemKind::RoastedTuber,
            RimItemKind::RoastedShoot,
        ] {
            if world.actors[idx].take_inv(dish, 1) > 0 {
                let gain = match dish {
                    RimItemKind::RoastedMeat => 48,
                    RimItemKind::RoastedTuber => 44,
                    _ => 38,
                };
                world.actors[idx].hunger = world.actors[idx].hunger.saturating_add(gain).min(100);
                world.actors[idx].stamina = world.actors[idx].stamina.saturating_add(16).min(100);
                world.actors[idx].hp = (world.actors[idx].hp + 8).min(world.actors[idx].max_hp);
                world.actors[idx].pop_text =
                    Some((format!("吃罷{}", dish.label()), world.tick + 10));
                world.actors[idx].activity_tag = "就地進食".into();
                return true;
            }
        }
        let claimed: Vec<(i32, i32)> = world
            .actors
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != idx)
            .filter_map(|(_, a)| match a.task {
                PendingTask::ChopAt { tx, ty } | PendingTask::PickupEquip { tx, ty } => {
                    Some((tx, ty))
                }
                _ => None,
            })
            .collect();
        for dish in [
            RimItemKind::RoastedMeat,
            RimItemKind::RoastedTuber,
            RimItemKind::RoastedShoot,
        ] {
            if let Some((gx, gy)) = world.find_nearest_ground_item(start, dish, false)
                && !claimed.contains(&(gx, gy))
                && (gx - start.0).abs() + (gy - start.1).abs() <= 22
                && let Some(p) = find_micro_path(rt, &world.overlays, start, (gx, gy), true, false)
            {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::PickupEquip { tx: gx, ty: gy };
                world.actors[idx].activity_tag = "取食充飢".into();
                return true;
            }
        }
        let raw_opt = [
            RimItemKind::BeastMeat,
            RimItemKind::SweetTuber,
            RimItemKind::BambooShoot,
        ]
        .into_iter()
        .find(|raw| {
            world.actors[idx]
                .inventory
                .iter()
                .any(|s| s.item == *raw && s.count > 0)
        });
        if let Some(raw) = raw_opt {
            if let Some(&(fx, fy)) = world
                .overlays
                .iter()
                .filter(|(_, ov)| ov.structure == Some(StructureKind::Campfire))
                .min_by_key(|&(&(cx, cy), _)| (cx - start.0).abs() + (cy - start.1).abs())
                .map(|(pos, _)| pos)
                && (fx - start.0).abs() + (fy - start.1).abs() <= 36
                && let Some(p) = find_micro_path(rt, &world.overlays, start, (fx, fy), true, false)
            {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::CookAt {
                    tx: fx,
                    ty: fy,
                    raw,
                };
                world.actors[idx].activity_tag = "赴火炊食".into();
                return true;
            }
            if world.actors[idx].hunger < 25
                && matches!(raw, RimItemKind::BambooShoot | RimItemKind::SweetTuber)
            {
                let _ = eat_item(&mut world.actors[idx], raw);
                return true;
            }
        }
        for raw in [
            RimItemKind::BeastMeat,
            RimItemKind::SweetTuber,
            RimItemKind::BambooShoot,
        ] {
            if let Some((gx, gy)) = world.find_nearest_ground_item(start, raw, false)
                && !claimed.contains(&(gx, gy))
                && (gx - start.0).abs() + (gy - start.1).abs() <= 22
                && let Some(p) = find_micro_path(rt, &world.overlays, start, (gx, gy), true, false)
            {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::PickupEquip { tx: gx, ty: gy };
                world.actors[idx].activity_tag = "拾取食材".into();
                return true;
            }
        }
        // 林間採鮮筍備食（由最近候選依距離排序並鎖定營火 32m 內，防單向一路越採越遠）
        let anchor = world
            .overlays
            .iter()
            .filter(|(_, ov)| ov.structure == Some(StructureKind::Campfire))
            .min_by_key(|&(&(cx, cy), _)| (cx - start.0).abs() + (cy - start.1).abs())
            .map_or((0, 0), |(&pos, _)| pos);
        let mut shoot_cands: Vec<(i32, (i32, i32))> = Vec::new();
        for dy in -16..=16 {
            for dx in -16..=16 {
                let (cx, cy) = (start.0 + dx, start.1 + dy);
                if (cx - anchor.0).abs().max((cy - anchor.1).abs()) > 32
                    || claimed.contains(&(cx, cy))
                {
                    continue;
                }
                let ov_clear = world
                    .overlays
                    .get(&(cx, cy))
                    .is_some_and(|o| o.cleared_natural);
                if !ov_clear
                    && matches!(
                        derive_micro_pristine(rt, cx, cy).1,
                        Some(NaturalProp::BambooShoot)
                    )
                {
                    shoot_cands.push((dx.abs() + dy.abs(), (cx, cy)));
                }
            }
        }
        shoot_cands.sort_by_key(|&(d, _)| d);
        for (_, (cx, cy)) in shoot_cands {
            if let Some(p) = find_micro_path(rt, &world.overlays, start, (cx, cy), true, false) {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::ChopAt { tx: cx, ty: cy };
                world.actors[idx].activity_tag = "採摘鮮筍".into();
                return true;
            }
        }
    }

    // 3. 精力低落時回營火旁空席歇息
    if world.actors[idx].stamina < 28
        && let Some(&(fx, fy)) = world
            .overlays
            .iter()
            .filter(|(_, ov)| ov.structure == Some(StructureKind::Campfire))
            .min_by_key(|&(&(cx, cy), _)| (cx - start.0).abs() + (cy - start.1).abs())
            .map(|(pos, _)| pos)
    {
        let seat = world.pick_campfire_seat(rt, fx, fy, world.actors[idx].id);
        let occ = world.occupied_cells(Some(idx));
        if let Some(p) = super::rim_world::find_micro_path_crowded(
            rt,
            &world.overlays,
            start,
            seat,
            false,
            false,
            &occ,
        ) {
            world.actors[idx].path = p.into();
            world.actors[idx].task = PendingTask::RestAt { tx: fx, ty: fy };
            world.actors[idx].activity_tag = "圍爐養神".into();
            return true;
        }
    }

    false
}

fn nearest_campfire(world: &RimWorldState, from: (i32, i32)) -> Option<(i32, i32)> {
    world
        .overlays
        .iter()
        .filter(|(_, ov)| ov.structure == Some(StructureKind::Campfire))
        .min_by_key(|&(&(cx, cy), _)| (cx - from.0).abs() + (cy - from.1).abs())
        .map(|(&pos, _)| pos)
}

/// 發現周邊有因交鋒力竭倒地的聚落同伴時，健康成員主動趕赴扶起並護送歸營調息。
pub fn try_rescue_downed_ally(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
) -> bool {
    let me = &world.actors[idx];
    if me.faction != ActorFaction::Settler
        || me.downed_ticks > 0
        || me.injury_ticks > 0
        || me.hp <= me.max_hp / 3
    {
        return false;
    }
    let start = me.micro_pos();
    let my_id = me.id;
    let claimed_targets: Vec<usize> = world
        .actors
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != idx)
        .filter_map(|(_, a)| match a.task {
            PendingTask::RescueAlly { target_id } => Some(target_id),
            _ => None,
        })
        .collect();

    let Some((ally_id, ally_pos)) = world
        .actors
        .iter()
        .filter(|a| {
            a.id != my_id
                && a.faction == ActorFaction::Settler
                && a.downed_ticks > 0
                && !claimed_targets.contains(&a.id)
        })
        .map(|a| (a.id, a.micro_pos()))
        .filter(|(_, pos)| (pos.0 - start.0).abs() + (pos.1 - start.1).abs() <= 32)
        .min_by_key(|(_, pos)| (pos.0 - start.0).abs() + (pos.1 - start.1).abs())
    else {
        return false;
    };

    if let Some(p) = find_micro_path(rt, &world.overlays, start, ally_pos, true, false) {
        world.actors[idx].path = p.into();
        world.actors[idx].task = PendingTask::RescueAlly { target_id: ally_id };
        world.actors[idx].activity_tag = "趕赴扶助".into();
        return true;
    }
    false
}

/// 抵達力竭同伴身旁時將其扶起並引往就近營火靜養。
pub fn complete_rescue_ally(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    target_id: usize,
) {
    let Some(t_idx) = world
        .actors
        .iter()
        .position(|a| a.id == target_id && a.faction == ActorFaction::Settler && a.downed_ticks > 0)
    else {
        world.actors[idx].task = PendingTask::None;
        world.actors[idx].activity_tag = "駐足待命".into();
        return;
    };

    let dist = (world.actors[t_idx].px - world.actors[idx].px)
        .hypot(world.actors[t_idx].py - world.actors[idx].py);
    if dist > 1.8 {
        if world.actors[idx].path.is_empty() {
            let s = world.actors[idx].micro_pos();
            let g = world.actors[t_idx].micro_pos();
            if let Some(p) = find_micro_path(rt, &world.overlays, s, g, true, false) {
                world.actors[idx].path = p.into();
            } else {
                world.actors[idx].task = PendingTask::None;
            }
        }
        return;
    }

    let r_glyph = world.actors[idx].glyph.clone();
    let r_name = world.actors[idx].name.clone();
    let t_name = world.actors[t_idx].name.clone();
    let t_glyph = world.actors[t_idx].glyph.clone();
    let t_pos = world.actors[t_idx].micro_pos();

    world.actors[t_idx].downed_ticks = 0;
    world.actors[t_idx].hp = world.actors[t_idx]
        .hp
        .max((world.actors[t_idx].max_hp * 2 / 5).max(32));
    world.actors[t_idx].activity_tag = "獲救歸營".into();
    world.actors[t_idx].pop_text = Some((format!("獲{r_glyph}扶起"), world.tick + 10));
    world.actors[t_idx].push_diary(format!("力竭時獲{r_name}扶起護送歸營調息"));

    if let Some((fx, fy)) = nearest_campfire(world, t_pos) {
        let seat = world.pick_campfire_seat(rt, fx, fy, world.actors[t_idx].id);
        let occ = world.occupied_cells(Some(t_idx));
        if let Some(p) =
            super::rim_world::find_micro_path_crowded(rt, &world.overlays, t_pos, seat, false, false, &occ)
        {
            world.actors[t_idx].path = p.into();
            world.actors[t_idx].task = PendingTask::RestAt { tx: fx, ty: fy };
        }
    }

    world.actors[idx].task = PendingTask::None;
    world.actors[idx].activity_tag = "扶傷返營".into();
    world.actors[idx].pop_text = Some((format!("扶起{t_glyph}"), world.tick + 10));
    world.actors[idx].push_diary(format!("扶起負傷的{t_name}退回營火旁"));
    world.push_event(format!("{r_glyph} 扶起負傷的 {t_name} 歸營"));
}

/// 玩家或居民直接食用行囊內的食物。
pub fn eat_item(actor: &mut RimActor, item: RimItemKind) -> Result<&'static str, &'static str> {
    if !item.is_edible() {
        return Err("此物不可食用");
    }
    if actor.take_inv(item, 1) == 0 {
        return Err("行囊裡已無此食物");
    }
    let (hg, stam, hp_gain, msg) = match item {
        RimItemKind::RoastedMeat => (48_u8, 18_u8, 12_i32, "細嚼溫熱炙肉，腹內頓時踏實"),
        RimItemKind::RoastedTuber => (44_u8, 14_u8, 2_i32, "趁熱吃下鬆軟炭烤甘薯，腹中暖意融融"),
        RimItemKind::RoastedShoot => (38_u8, 14_u8, 8_i32, "撥開微焦籜葉吃罷煨筍，齒頰回甘"),
        RimItemKind::SweetTuber => (24_u8, 6_u8, 1_i32, "生嚼紅土甘薯，清甜解渴"),
        _ => (16_u8, 4_u8, 2_i32, "生嚼脆嫩鮮筍，略解腹飢"),
    };
    if item == RimItemKind::SweetTuber {
        actor.thirst = actor.thirst.saturating_add(4).min(100);
        actor.push_diary("生嚼甘甜紅土甘薯充飢");
    } else if item == RimItemKind::RoastedTuber {
        actor.push_diary("趁熱吃下鬆軟炭烤甘薯");
    }
    actor.hunger = actor.hunger.saturating_add(hg).min(100);
    actor.stamina = actor.stamina.saturating_add(stam).min(100);
    actor.hp = (actor.hp + hp_gain).min(actor.max_hp);
    actor.activity_tag = "進食補氣".into();
    Ok(msg)
}

/// 穿戴或卸下頭部竹笠／背部背簍，100% 維持行囊與身外裝備守恆。
pub fn toggle_wearable(
    actor: &mut RimActor,
    item: RimItemKind,
    wear: bool,
) -> Result<&'static str, &'static str> {
    match item {
        RimItemKind::BambooHat => {
            if wear {
                if actor.equipped_hat {
                    return Ok("已戴著細編竹笠");
                }
                if actor.take_inv(RimItemKind::BambooHat, 1) == 0 {
                    return Err("行囊內沒有細編竹笠");
                }
                actor.equipped_hat = true;
                Ok("戴上細編竹笠，遮陽擋雨")
            } else {
                if !actor.equipped_hat {
                    return Ok("頭上並未戴笠");
                }
                actor.equipped_hat = false;
                actor.add_inv(RimItemKind::BambooHat, 1);
                Ok("已除下細編竹笠收入行囊")
            }
        }
        RimItemKind::BambooPack => {
            if wear {
                if actor.equipped_pack {
                    return Ok("已背著竹編背簍");
                }
                if actor.take_inv(RimItemKind::BambooPack, 1) == 0 {
                    return Err("行囊內沒有竹編背簍");
                }
                actor.equipped_pack = true;
                Ok("肩上負起竹編背簍，行囊利落")
            } else {
                if !actor.equipped_pack {
                    return Ok("肩背原本輕裝無簍");
                }
                actor.equipped_pack = false;
                actor.add_inv(RimItemKind::BambooPack, 1);
                Ok("已卸下竹編背簍收入行囊")
            }
        }
        _ => Err("此物不可穿戴於頭背"),
    }
}

/// 玩家與現場居民當面以「鎂」互通有無（絕對物質與鎂總量雙守恆）。
pub fn execute_barter(
    player: &mut RimActor,
    npc: &mut RimActor,
    item: RimItemKind,
    player_buys: bool,
) -> Result<String, String> {
    let buy_price = item.base_magnesium_price();
    let sell_price = (buy_price * 4 / 5).max(1);
    if player_buys {
        if player.magnesium < buy_price {
            return Err("隨身攜帶的鎂晶不足".into());
        }
        if npc.take_inv(item, 1) == 0 {
            return Err("對方手邊已無此貨品".into());
        }
        player.magnesium -= buy_price;
        npc.magnesium = npc.magnesium.saturating_add(buy_price);
        player.add_inv(item, 1);
        Ok(format!("付予鎂晶，向{}換得「{}」", npc.name, item.label()))
    } else {
        if npc.magnesium < sell_price {
            return Err("對方囊中鎂晶不夠收購此物".into());
        }
        if player.take_inv(item, 1) == 0 {
            return Err("行囊裡已無此物品可讓出".into());
        }
        npc.magnesium -= sell_price;
        player.magnesium = player.magnesium.saturating_add(sell_price);
        npc.add_inv(item, 1);
        Ok(format!("將「{}」讓與{}，收下鎂晶", item.label(), npc.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::rim_world::GroundStack;
    use std::collections::VecDeque;

    fn dummy_actor(name: &str, mag: u32, inv: Vec<GroundStack>) -> RimActor {
        RimActor {
            id: 1,
            code: name.into(),
            actor_key: name.into(),
            entity_id: None,
            glyph: "測".into(),
            name: name.into(),
            gender: "M".into(),
            soul_seed: 1,
            role_tag: "測試".into(),
            faction: ActorFaction::Settler,
            is_player: false,
            autopilot: false,
            running: false,
            px: 0.0,
            py: 0.0,
            facing_rad: 0.0,
            hp: 90,
            max_hp: 100,
            equipped_tool: None,
            carrying: None,
            inventory: inv,
            directive: String::new(),
            path: VecDeque::new(),
            task: PendingTask::None,
            activity_tag: String::new(),
            pop_text: None,
            cooldown_ticks: 0,
            last_active_unix: 0,
            hunger: 40,
            thirst: 50,
            stamina: 60,
            magnesium: mag,
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
    fn wearables_and_barter_strictly_conserve_items_and_magnesium() {
        let mut p = dummy_actor(
            "玩家甲",
            20,
            vec![
                GroundStack {
                    item: RimItemKind::BambooHat,
                    count: 1,
                },
                GroundStack {
                    item: RimItemKind::BambooPole,
                    count: 2,
                },
            ],
        );
        let mut npc = dummy_actor(
            "周阿茶",
            35,
            vec![GroundStack {
                item: RimItemKind::RoastedShoot,
                count: 2,
            }],
        );

        assert!(toggle_wearable(&mut p, RimItemKind::BambooHat, true).is_ok());
        assert!(p.equipped_hat);
        assert_eq!(p.take_inv(RimItemKind::BambooHat, 1), 0);
        assert!(toggle_wearable(&mut p, RimItemKind::BambooHat, false).is_ok());
        assert!(!p.equipped_hat);
        assert!(
            p.inventory
                .iter()
                .any(|s| s.item == RimItemKind::BambooHat && s.count == 1)
        );

        let total_mag_before = p.magnesium + npc.magnesium;
        assert!(execute_barter(&mut p, &mut npc, RimItemKind::RoastedShoot, true).is_ok());
        assert_eq!(p.magnesium + npc.magnesium, total_mag_before);
        assert_eq!(p.magnesium, 16);
        assert_eq!(npc.magnesium, 39);

        assert!(eat_item(&mut p, RimItemKind::RoastedShoot).is_ok());
        assert_eq!(p.hunger, 78);

        assert!(execute_barter(&mut p, &mut npc, RimItemKind::BambooPole, false).is_ok());
        assert_eq!(p.magnesium + npc.magnesium, total_mag_before);
    }

    #[test]
    fn head_on_settlers_pass_shared_waypoint_without_deadlocking() {
        let mut overlays = HashMap::new();
        for x in -3..=3 {
            for y in -2..=2 {
                overlays.insert(
                    (x, y),
                    MicroCellOverlay {
                        cleared_natural: true,
                        structure: None,
                        ..Default::default()
                    },
                );
            }
        }
        let mut a = dummy_actor("甲", 10, Vec::new());
        a.id = 1;
        a.px = -2.0;
        a.py = 0.0;
        a.path = VecDeque::from([(-1, 0), (0, 0), (1, 0), (2, 0)]);

        let mut b = dummy_actor("乙", 10, Vec::new());
        b.id = 2;
        b.px = 2.0;
        b.py = 0.0;
        b.path = VecDeque::from([(1, 0), (0, 0), (-1, 0), (-2, 0)]);

        let mut world = RimWorldState {
            tick: 1,
            overlays,
            actors: vec![a, b],
            events: VecDeque::new(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };
        for _ in 0..90 {
            world.step_physics_50ms();
        }
        assert!(world.actors[0].path.is_empty());
        assert!(world.actors[1].path.is_empty());
        assert!(world.actors[0].px > 1.0, "a.px={}", world.actors[0].px);
        assert!(world.actors[1].px < -1.0, "b.px={}", world.actors[1].px);
    }

    #[test]
    fn autopilot_forager_picks_up_shoot_cooks_and_eats() {
        let mut overlays = HashMap::new();
        let mut shoot_cell = None;
        for y in -12..=12 {
            for x in -12..=12 {
                if (x, y) == (0, 0) || (x, y) == (1, 0) {
                    overlays.insert(
                        (x, y),
                        MicroCellOverlay {
                            cleared_natural: true,
                            structure: (x == 0 && y == 0).then_some(StructureKind::Campfire),
                            ..Default::default()
                        },
                    );
                    continue;
                }
                if shoot_cell.is_none()
                    && derive_micro_pristine(None, x, y).1 == Some(NaturalProp::BambooShoot)
                {
                    shoot_cell = Some((x, y));
                } else {
                    overlays.insert(
                        (x, y),
                        MicroCellOverlay {
                            cleared_natural: true,
                            ..Default::default()
                        },
                    );
                }
            }
        }
        assert!(shoot_cell.is_some(), "expected natural bamboo shoot cell");

        let mut p = dummy_actor("自走採筍", 18, Vec::new());
        p.id = 7;
        p.entity_id = Some("tester".into());
        p.is_player = true;
        p.autopilot = true;
        p.equipped_tool = Some(RimItemKind::BambooBlade);
        p.px = 1.0;
        p.py = 0.0;
        p.hunger = 35;

        let mut world = RimWorldState {
            tick: 1,
            overlays,
            actors: vec![p],
            events: VecDeque::new(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };
        for _ in 0..450 {
            world.step_physics_50ms();
            if world.actors[0].hunger >= 70 {
                break;
            }
        }
        assert!(
            world.actors[0].hunger >= 70,
            "hunger={}",
            world.actors[0].hunger
        );
        assert!(world.actors[0].px.abs() < 32.0 && world.actors[0].py.abs() < 32.0);
    }
}

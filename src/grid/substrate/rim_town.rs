//! 西麓湧泉街鎮規劃：商住一體／純住宅／旅宿／公倉、石板街道、騎樓亭仔腳、街燈與歸戶安歇系統。

use std::collections::HashMap;

use serde_json::{Value, json};

use super::rim_economy::find_best_stockpile_slot_for;
use super::rim_sim::{ActorFaction, PendingTask, RimWorldState};
use super::rim_world::{
    GroundStack, MicroCellOverlay, MicroGround, RimItemKind, StructureKind,
    derive_micro_pristine, find_micro_path_crowded,
};
use super::runtime::SubstrateRuntime;

/// 街鎮有名建物與家戶配置規格。
#[derive(Debug, Clone, Copy)]
pub struct TownBuildingSpec {
    pub code: &'static str,
    pub name: &'static str,
    pub kind_label: &'static str,
    pub household_note: &'static str,
    pub bbox: (i32, i32, i32, i32),
    pub shop_zone: Option<(i32, i32, i32, i32)>,
    pub home_zone: Option<(i32, i32, i32, i32)>,
    pub clinic_bed: Option<(i32, i32)>,
    pub guest_beds: &'static [(i32, i32)],
    pub owner_beds: &'static [(&'static str, (i32, i32))],
}

/// 街鎮 9 棟建物正典清單（8 處有主街屋住宅 + 1 處公倉，零空屋；15 名常駐家戶 1 對 1 專屬床位）。
pub const TOWN_BUILDINGS: [TownBuildingSpec; 9] = [
    TownBuildingSpec {
        code: "chunshui_diner",
        name: "春水小吃",
        kind_label: "商住一體",
        household_note: "母子 2 人一戶",
        bbox: (-20, -15, -3, 5),
        shop_zone: Some((-20, -15, -3, 1)),
        home_zone: Some((-20, -15, 1, 5)),
        clinic_bed: None,
        guest_beds: &[],
        owner_beds: &[
            ("res:liu_chunshui", (-19, 4)),
            ("res:liu_abao", (-16, 4)),
        ],
    },
    TownBuildingSpec {
        code: "sanshui_shop",
        name: "三水商行",
        kind_label: "商住一體",
        household_note: "掌櫃夥計 2 人一戶",
        bbox: (-13, -6, -1, 5),
        shop_zone: Some((-13, -6, -1, 1)),
        home_zone: Some((-13, -6, 1, 5)),
        clinic_bed: None,
        guest_beds: &[],
        owner_beds: &[
            ("res:lin_sanshui", (-7, 4)),
            ("res:gao_afa", (-7, 2)),
        ],
    },
    TownBuildingSpec {
        code: "amu_smithy",
        name: "阿木鐵店",
        kind_label: "商住一體",
        household_note: "師徒 2 人一戶",
        bbox: (7, 11, -3, 5),
        shop_zone: Some((7, 11, -3, 1)),
        home_zone: Some((7, 11, 1, 5)),
        clinic_bed: None,
        guest_beds: &[],
        owner_beds: &[
            ("res:lin_amu", (8, 4)),
            ("res:zhang_xiaoan", (8, 2)),
        ],
    },
    TownBuildingSpec {
        code: "jian_clinic",
        name: "濟安診所",
        kind_label: "商住一體",
        household_note: "父女 2 人一戶",
        bbox: (14, 19, -3, 5),
        shop_zone: Some((14, 19, -3, 1)),
        home_zone: Some((14, 19, 1, 5)),
        clinic_bed: Some((18, -1)),
        guest_beds: &[],
        owner_beds: &[
            ("res:zhang_jian", (15, 4)),
            ("res:zhang_suxin", (18, 4)),
        ],
    },
    TownBuildingSpec {
        code: "yingfeng_inn",
        name: "迎風宿屋",
        kind_label: "旅宿",
        household_note: "1 人駐守・附雙客榻",
        bbox: (-20, -15, -12, -7),
        shop_zone: Some((-20, -15, -9, -7)),
        home_zone: Some((-20, -15, -12, -9)),
        clinic_bed: None,
        guest_beds: &[(-16, -11), (-16, -10)],
        owner_beds: &[("res:xie_chengfeng", (-19, -11))],
    },
    TownBuildingSpec {
        code: "xu_farmhouse",
        name: "許家農宅",
        kind_label: "純住宅",
        household_note: "一家 3 口",
        bbox: (-13, -6, -12, -7),
        shop_zone: None,
        home_zone: Some((-13, -6, -12, -7)),
        clinic_bed: None,
        guest_beds: &[],
        owner_beds: &[
            ("res:xu_qiuniang", (-12, -11)),
            ("res:xu_dabo", (-10, -11)),
            ("res:xu_xiaoman", (-8, -11)),
        ],
    },
    TownBuildingSpec {
        code: "chen_cottage",
        name: "陳青宅",
        kind_label: "純住宅",
        household_note: "單身 1 人一戶",
        bbox: (10, 14, -12, -7),
        shop_zone: None,
        home_zone: Some((10, 14, -12, -7)),
        clinic_bed: None,
        guest_beds: &[],
        owner_beds: &[("res:chen_qing", (11, -11))],
    },
    TownBuildingSpec {
        code: "shuiwei_granary",
        name: "水尾寮倉房",
        kind_label: "山貨公倉",
        household_note: "街鎮山貨公倉",
        bbox: (21, 26, -12, -8),
        shop_zone: Some((21, 26, -12, -8)),
        home_zone: None,
        clinic_bed: None,
        guest_beds: &[],
        owner_beds: &[],
    },
    TownBuildingSpec {
        code: "acha_teahouse",
        name: "阿茶茶寮",
        kind_label: "商住一體",
        household_note: "夫妻 2 人一戶",
        bbox: (28, 36, -10, -4),
        shop_zone: Some((28, 33, -10, -4)),
        home_zone: Some((33, 36, -10, -4)),
        clinic_bed: None,
        guest_beds: &[],
        owner_beds: &[
            ("res:zhou_acha", (35, -5)),
            ("res:he_dashu", (35, -9)),
        ],
    },
];

fn set_town_cell(
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
    pos: (i32, i32),
    st: Option<StructureKind>,
) {
    let e = overlays.entry(pos).or_default();
    e.cleared_natural = true;
    e.blueprint = None;
    e.structure = st;
}

fn vacate_and_set(
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
    pos: (i32, i32),
    st: Option<StructureKind>,
    fallback: (i32, i32),
) {
    let displaced = overlays.get_mut(&pos).and_then(|o| o.ground_item.take());
    set_town_cell(overlays, pos, st);
    if let Some(stack) = displaced {
        let mut rem = stack.count;
        while rem > 0 {
            if let Some(dst) = find_best_stockpile_slot_for(overlays, fallback, stack.item, None) {
                let entry = overlays.entry(dst).or_default();
                match &mut entry.ground_item {
                    None => entry.ground_item = Some(GroundStack { item: stack.item, count: 1 }),
                    Some(g) => g.count = g.count.saturating_add(1),
                }
                rem -= 1;
            } else {
                break;
            }
        }
    }
}

fn build_rect_room(
    rt: Option<&SubstrateRuntime>,
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
    bounds: (i32, i32, i32, i32),
    doors: &[(i32, i32)],
) {
    let (x0, x1, y0, y1) = bounds;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (g, _) = derive_micro_pristine(rt, x, y);
            if g.is_water() || g == MicroGround::SteepCliff {
                continue;
            }
            let is_border = x == x0 || x == x1 || y == y0 || y == y1;
            if is_border {
                let st = if doors.contains(&(x, y)) {
                    StructureKind::BambooDoor
                } else {
                    StructureKind::BambooWall
                };
                vacate_and_set(overlays, (x, y), Some(st), (-9, 0));
            } else {
                overlays.entry((x, y)).or_default().cleared_natural = true;
            }
        }
    }
}

fn lay_if_clear(
    rt: Option<&SubstrateRuntime>,
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
    pos: (i32, i32),
    st: StructureKind,
) {
    let (g, _) = derive_micro_pristine(rt, pos.0, pos.1);
    if g.is_water() || g == MicroGround::SteepCliff {
        return;
    }
    let e = overlays.entry(pos).or_default();
    if e.structure.is_none() && e.blueprint.is_none() && e.ground_item.is_none() {
        e.cleared_natural = true;
        e.structure = Some(st);
    }
}

/// 冪等升級西麓湧泉街景：清理露天地攤營火、隔間商住街屋、鋪石板道、亭仔腳與街燈。
pub fn apply_town_architecture(
    rt: Option<&SubstrateRuntime>,
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
) {
    // 1. 移除舊野外露宿式廣場營火 (0,0)/(6,1) 與戶外工作檯 (4,-1)/(10,4)，無損保物並保留 (7..=11,1..=5)
    for (pos, replacement) in [
        ((0, 0), Some(StructureKind::StonePaving)),
        ((4, -1), Some(StructureKind::StonePaving)),
        ((6, 1), Some(StructureKind::StonePaving)),
    ] {
        if matches!(
            overlays.get(&pos).and_then(|o| o.structure),
            Some(StructureKind::Campfire | StructureKind::Workbench)
        ) {
            vacate_and_set(overlays, pos, replacement, (-8, 0));
        }
    }
    if overlays.get(&(10, 4)).and_then(|o| o.structure) == Some(StructureKind::Workbench) {
        set_town_cell(overlays, (10, 4), None);
    }

    // 2. 三水商行 (-13..=-6, -1..=5)：加設中隔牆 y=1 與北側後門 (-10,5) 赴料場、安兩張專屬睡榻
    for x in -13..=-6 {
        let st = if x == -9 {
            StructureKind::BambooDoor
        } else {
            StructureKind::BambooWall
        };
        vacate_and_set(overlays, (x, 1), Some(st), (-12, 4));
    }
    vacate_and_set(overlays, (-10, 5), Some(StructureKind::BambooDoor), (-12, 4));
    for bed in [(-7, 4), (-7, 2)] {
        if overlays.get(&bed).and_then(|o| o.structure) != Some(StructureKind::BambooBed) {
            vacate_and_set(overlays, bed, Some(StructureKind::BambooBed), (-11, 4));
        }
    }

    // 3. 阿木鐵店前店 (7..=11, -3..=1)：與後寢臥 (7..=11, 1..=5) 切齊，x=12..=13 留 2m 石板巷
    for y in -4..=5 {
        vacate_and_set(overlays, (12, y), Some(StructureKind::StonePaving), (8, -1));
    }
    if overlays.get(&(11, -1)).and_then(|o| o.structure) == Some(StructureKind::Workbench) {
        vacate_and_set(overlays, (11, -1), Some(StructureKind::BambooWall), (8, -1));
    }
    build_rect_room(rt, overlays, (7, 11, -3, 1), &[(9, -3), (9, 1)]);
    vacate_and_set(overlays, (10, -1), Some(StructureKind::Workbench), (8, -1));
    lay_if_clear(rt, overlays, (8, -1), StructureKind::Stockpile);

    // 4. 春水小吃 (-20..=-15, -3..=5)：前灶食後母子臥房
    build_rect_room(rt, overlays, (-20, -15, -3, 1), &[(-18, -3), (-18, 1)]);
    build_rect_room(rt, overlays, (-20, -15, 1, 5), &[(-18, 1)]);
    vacate_and_set(overlays, (-16, -1), Some(StructureKind::Campfire), (-19, -1));
    lay_if_clear(rt, overlays, (-19, -1), StructureKind::Stockpile);
    for bed in [(-19, 4), (-16, 4)] {
        vacate_and_set(overlays, bed, Some(StructureKind::BambooBed), (-19, -1));
    }
    if let Some(ov) = overlays.get_mut(&(-19, -1))
        && ov.structure == Some(StructureKind::Stockpile)
        && ov.ground_item.is_none()
    {
        ov.ground_item = Some(GroundStack {
            item: RimItemKind::RoastedShoot,
            count: 2,
        });
    }

    // 5. 濟安診所 (14..=19, -3..=5)：前診間藥檯＋病榻 (18,-1)，後父女臥房
    build_rect_room(rt, overlays, (14, 19, -3, 1), &[(16, -3), (16, 1)]);
    build_rect_room(rt, overlays, (14, 19, 1, 5), &[(16, 1)]);
    vacate_and_set(overlays, (15, -1), Some(StructureKind::Workbench), (17, -1));
    lay_if_clear(rt, overlays, (17, -1), StructureKind::Stockpile);
    for bed in [(18, -1), (15, 4), (18, 4)] {
        vacate_and_set(overlays, bed, Some(StructureKind::BambooBed), (17, -1));
    }
    if let Some(ov) = overlays.get_mut(&(17, -1))
        && ov.structure == Some(StructureKind::Stockpile)
        && ov.ground_item.is_none()
    {
        ov.ground_item = Some(GroundStack {
            item: RimItemKind::WoundSalve,
            count: 2,
        });
    }

    // 6. 迎風宿屋 (-20..=-15, -12..=-7)：中牆 (-18,-9) 隔開前櫃檯廳與後客寢區，1 掌櫃榻＋2 客榻
    build_rect_room(rt, overlays, (-20, -15, -9, -7), &[(-18, -7), (-18, -9)]);
    build_rect_room(rt, overlays, (-20, -15, -12, -9), &[(-18, -9)]);
    vacate_and_set(overlays, (-19, -8), Some(StructureKind::Stockpile), (-18, -8));
    for bed in [(-19, -11), (-16, -11), (-16, -10)] {
        vacate_and_set(overlays, bed, Some(StructureKind::BambooBed), (-19, -8));
    }

    // 7. 許家農宅 (-13..=-6, -12..=-7)：一家 3 口純住宅，南門 (-9,-12) 直通南坡田畦
    build_rect_room(rt, overlays, (-13, -6, -12, -7), &[(-9, -7), (-6, -9), (-9, -12)]);
    vacate_and_set(overlays, (-11, -8), Some(StructureKind::Campfire), (-7, -8));
    lay_if_clear(rt, overlays, (-7, -8), StructureKind::Stockpile);
    for bed in [(-12, -11), (-10, -11), (-8, -11)] {
        vacate_and_set(overlays, bed, Some(StructureKind::BambooBed), (-7, -8));
    }

    // 8. 陳青宅 (10..=14, -12..=-7)：單身獵戶 1 人小宅
    build_rect_room(rt, overlays, (10, 14, -12, -7), &[(12, -7)]);
    vacate_and_set(overlays, (11, -8), Some(StructureKind::Campfire), (13, -10));
    lay_if_clear(rt, overlays, (13, -10), StructureKind::Stockpile);
    vacate_and_set(overlays, (11, -11), Some(StructureKind::BambooBed), (13, -10));

    // 9. 阿茶茶寮前檯茶席：保有 (28..=36, -10..=-4) 完整外框不變
    lay_if_clear(rt, overlays, (30, -5), StructureKind::Stockpile);

    // 10. 連續亭仔腳（騎樓迴廊，嚴格避開 (2,0) 與 (5,3) 避雨檢定露天地）
    for x in -20..=-15 {
        lay_if_clear(rt, overlays, (x, -4), StructureKind::CoveredArcade);
        lay_if_clear(rt, overlays, (x, -6), StructureKind::CoveredArcade);
    }
    for x in -13..=-6 {
        lay_if_clear(rt, overlays, (x, -2), StructureKind::CoveredArcade);
        lay_if_clear(rt, overlays, (x, -6), StructureKind::CoveredArcade);
    }
    for x in 7..=11 {
        lay_if_clear(rt, overlays, (x, -4), StructureKind::CoveredArcade);
    }
    for x in 14..=19 {
        lay_if_clear(rt, overlays, (x, -4), StructureKind::CoveredArcade);
    }
    for x in 10..=14 {
        lay_if_clear(rt, overlays, (x, -6), StructureKind::CoveredArcade);
    }
    for x in 21..=26 {
        lay_if_clear(rt, overlays, (x, -7), StructureKind::CoveredArcade);
    }
    for x in 29..=35 {
        lay_if_clear(rt, overlays, (x, -3), StructureKind::CoveredArcade);
    }
    for y in -9..=-5 {
        lay_if_clear(rt, overlays, (27, y), StructureKind::CoveredArcade);
    }

    // 11. 湧泉本街、廣場與巷道石板鋪面（不遮雨、不擋路）
    for x in -21..=27 {
        lay_if_clear(rt, overlays, (x, -5), StructureKind::StonePaving);
    }
    for y in -4..=4 {
        for x in -5..=5 {
            lay_if_clear(rt, overlays, (x, y), StructureKind::StonePaving);
        }
    }
    for y in -12..=5 {
        lay_if_clear(rt, overlays, (-14, y), StructureKind::StonePaving);
        lay_if_clear(rt, overlays, (-21, y), StructureKind::StonePaving);
        lay_if_clear(rt, overlays, (9, y.min(-5)), StructureKind::StonePaving);
    }
    for y in -5..=5 {
        lay_if_clear(rt, overlays, (13, y), StructureKind::StonePaving);
    }
    for y in -12..=-5 {
        for x in 15..=20 {
            lay_if_clear(rt, overlays, (x, y), StructureKind::StonePaving);
        }
    }
    for x in -1..=1 {
        for y in 5..=8 {
            lay_if_clear(rt, overlays, (x, y), StructureKind::StonePaving);
        }
    }
    for x in -20..=-9 {
        lay_if_clear(rt, overlays, (x, -13), StructureKind::StonePaving);
    }
    for y in -19..=-13 {
        lay_if_clear(rt, overlays, (-14, y), StructureKind::StonePaving);
    }
    for x in 20..=27 {
        for y in -6..=-3 {
            lay_if_clear(rt, overlays, (x, y), StructureKind::StonePaving);
        }
    }
    lay_if_clear(rt, overlays, (6, 3), StructureKind::StonePaving);

    // 12. 導能恆光街燈（安放於路側與廣場轉角，防風雨且不擋門道與通道）
    for lamp in [
        (-21, -5),
        (-5, -4),
        (5, -4),
        (-2, 6),
        (16, -6),
        (20, -4),
        (27, -3),
    ] {
        vacate_and_set(overlays, lamp, Some(StructureKind::StreetLamp), (-9, 0));
    }
}

/// 查詢 15 名常駐家戶成員所對應之 1 對 1 專屬床位。
pub fn assigned_bed_for(actor_key: &str) -> Option<(i32, i32)> {
    TOWN_BUILDINGS
        .iter()
        .flat_map(|b| b.owner_beds.iter())
        .find_map(|&(key, pos)| (key == actor_key).then_some(pos))
}

/// 查詢角色夜間睡眠或休息最優先的床位（含貨郎 `鄭石頭` 投宿迎風宿屋一號客榻）。
pub fn preferred_bed_for(actor_key: &str) -> Option<(i32, i32)> {
    assigned_bed_for(actor_key).or_else(|| (actor_key == "res:zheng_shitou").then_some((-16, -11)))
}

/// 該格是否為他人專屬家戶私床（非己床亦非診間／客棧公共床）。
pub fn is_other_household_bed(actor_key: &str, pos: (i32, i32)) -> bool {
    TOWN_BUILDINGS
        .iter()
        .flat_map(|b| b.owner_beds.iter())
        .any(|&(key, bed_pos)| bed_pos == pos && key != actor_key)
}

/// 尋找優先收治傷患的濟安診所病榻，若遭拆除則退回旅宿客榻或任意空床。
pub fn clinic_bed_near(overlays: &HashMap<(i32, i32), MicroCellOverlay>) -> Option<(i32, i32)> {
    for &pos in &[
        (18, -1),
        (15, 4),
        (-16, -10),
        (-16, -11),
        (8, 4),
        (-7, 4),
    ] {
        if overlays.get(&pos).and_then(|o| o.structure) == Some(StructureKind::BambooBed) {
            return Some(pos);
        }
    }
    overlays
        .iter()
        .find_map(|(&pos, o)| (o.structure == Some(StructureKind::BambooBed)).then_some(pos))
}

/// 夜間時段（`21:00 ～ 04:59`）引導街坊各自歸戶回到自己家門與專屬睡榻安歇。
pub fn try_town_night_sleep(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    game_hour: i32,
) -> bool {
    if !(game_hour >= 22 || game_hour < 4) {
        return false;
    }
    let actor = &world.actors[idx];
    if actor.faction != ActorFaction::Settler
        || actor.is_player
        || actor.hp <= 0
        || actor.downed_ticks > 0
        || actor.hunger < 25
        || actor.thirst < 25
    {
        return false;
    }
    let Some(bed) = preferred_bed_for(&actor.actor_key) else {
        return false;
    };
    if world.overlays.get(&bed).and_then(|o| o.structure) != Some(StructureKind::BambooBed) {
        return false;
    }
    let cur = actor.micro_pos();
    if cur == bed {
        world.actors[idx].task = PendingTask::None;
        world.actors[idx].activity_tag = "歸戶安歇".into();
        return true;
    }
    let occupied = world.occupied_cells(Some(idx));
    let Some(path) =
        find_micro_path_crowded(rt, &world.overlays, cur, bed, false, false, &occupied)
    else {
        return false;
    };
    world.actors[idx].path = path.into();
    world.actors[idx].task = PendingTask::SleepBed {
        tx: bed.0,
        ty: bed.1,
    };
    world.actors[idx].activity_tag = "夜歸安歇".into();
    true
}

pub fn ground_code_str(g: MicroGround) -> &'static str {
    match g {
        MicroGround::LoamGrass => "grass",
        MicroGround::RedGravel => "red",
        MicroGround::PebbleBank => "pebble",
        MicroGround::CoastalSand => "sand",
        MicroGround::TidalMudflat => "mudflat",
        MicroGround::BareRock => "rock",
        MicroGround::ShallowStream => "stream",
        MicroGround::SpringPool => "spring",
        MicroGround::DeepWater => "deep",
        MicroGround::SteepCliff => "cliff",
    }
}

pub fn structure_code_str(s: StructureKind) -> &'static str {
    match s {
        StructureKind::BambooWall => "wall",
        StructureKind::BambooDoor => "door",
        StructureKind::WoodBridge => "bridge",
        StructureKind::Campfire => "campfire",
        StructureKind::Workbench => "workbench",
        StructureKind::Stockpile => "stockpile",
        StructureKind::BambooBed => "bed",
        StructureKind::TilledRidge => "ridge",
        StructureKind::HerbSeedling => "herb_seed",
        StructureKind::HerbMature => "herb_plot",
        StructureKind::CropSeedling => "crop_seed",
        StructureKind::CropMature => "crop_plot",
        StructureKind::StonePaving => "paving",
        StructureKind::CoveredArcade => "arcade",
        StructureKind::StreetLamp => "lamp",
    }
}

pub fn parse_item_code_str(s: &str) -> Option<RimItemKind> {
    Some(match s {
        "bamboo" => RimItemKind::BambooPole,
        "stone" => RimItemKind::RiverStone,
        "shoot" => RimItemKind::BambooShoot,
        "meat" => RimItemKind::BeastMeat,
        "roast_shoot" => RimItemKind::RoastedShoot,
        "roast_meat" => RimItemKind::RoastedMeat,
        "blade" => RimItemKind::BambooBlade,
        "pick" => RimItemKind::StonePick,
        "hat" => RimItemKind::BambooHat,
        "pack" => RimItemKind::BambooPack,
        "herb" => RimItemKind::MountainHerb,
        "salve" => RimItemKind::WoundSalve,
        "tuber" => RimItemKind::SweetTuber,
        "roast_tuber" => RimItemKind::RoastedTuber,
        "alloy_blade" => RimItemKind::AlloyBlade,
        "alloy_pick" => RimItemKind::AlloyPick,
        _ => return None,
    })
}

pub fn is_protected_town_structure(mx: i32, my: i32, st: StructureKind) -> bool {
    building_at(mx, my).is_some()
        || matches!(
            st,
            StructureKind::StonePaving | StructureKind::CoveredArcade | StructureKind::StreetLamp
        )
        || st.is_garden_plot()
        || (mx == 0 && (9..=15).contains(&my) && st == StructureKind::WoodBridge)
        || ((-13..=-10).contains(&mx) && (6..=7).contains(&my))
        || ((21..=23).contains(&mx) && my == -14)
}

/// 白晝引導店主駐店看店、其餘街坊於宅前街巷閒步（取代舊版全員聚集原點「營畔歇息」）。
pub fn try_assign_town_duty_or_stroll(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    game_hour: i32,
    duty_only: bool,
) -> bool {
    const TOWN_POSTS: [(&str, (i32, i32), &str); 10] = [
        ("res:liu_chunshui", (-17, -1), "灶畔掌杓"),
        ("res:lin_sanshui", (-10, 0), "掌櫃看店"),
        ("res:lin_amu", (10, -2), "爐前司鍛"),
        ("res:zhang_jian", (15, -2), "診間候診"),
        ("res:xie_chengfeng", (-17, -8), "櫃檯迎客"),
        ("res:zhou_acha", (31, -6), "茶灶看火"),
        ("res:liu_abao", (-18, 0), "招呼堂食"),
        ("res:gao_afa", (-8, 0), "盤點山貨"),
        ("res:zhang_xiaoan", (8, -2), "整理鐵器"),
        ("res:zhang_suxin", (17, -2), "分揀百草"),
    ];
    if game_hour >= 22 || game_hour < 4 {
        return false;
    }
    let actor = &world.actors[idx];
    if actor.faction != ActorFaction::Settler || actor.is_player || actor.hp <= 0 {
        return false;
    }
    let start = actor.micro_pos();
    let occ = world.occupied_cells(Some(idx));
    let posts = if duty_only { &TOWN_POSTS[..6] } else { &TOWN_POSTS[..] };
    if let Some(&(_, post, tag)) = posts.iter().find(|&&(k, _, _)| k == actor.actor_key)
        && actor.hunger >= 35
        && actor.thirst >= 35
    {
        if (start.0 - post.0).abs() + (start.1 - post.1).abs() <= 1 {
            world.actors[idx].activity_tag = tag.into();
            return true;
        }
        if !matches!(actor.task, PendingTask::CookAt { .. } | PendingTask::CraftAt { .. })
            && let Some(p) = find_micro_path_crowded(rt, &world.overlays, start, post, false, false, &occ)
        {
            world.actors[idx].path = p.into();
            world.actors[idx].activity_tag = tag.into();
            return true;
        }
    }
    if !duty_only && world.tick % 19 == (world.actors[idx].id as u64 % 19) {
        let base = preferred_bed_for(&world.actors[idx].actor_key).unwrap_or(start);
        let angle = (world.tick as f32 * 0.7) + world.actors[idx].id as f32;
        let gx = base.0 + (angle.cos() * 3.0).round() as i32;
        let gy = base.1 + (angle.sin() * 3.0).round() as i32;
        if let Some(p) = find_micro_path_crowded(rt, &world.overlays, start, (gx, gy), false, false, &occ) {
            world.actors[idx].path = p.into();
            world.actors[idx].activity_tag = "宅前歇息".into();
            return true;
        }
    }
    false
}

pub fn building_at(mx: i32, my: i32) -> Option<&'static TownBuildingSpec> {
    TOWN_BUILDINGS.iter().find(|b| {
        mx >= b.bbox.0 && mx <= b.bbox.1 && my >= b.bbox.2 && my <= b.bbox.3
    })
}

pub fn enrich_card_with_building(
    mx: i32,
    my: i32,
    title: &mut String,
    tags: &mut Vec<String>,
) {
    let Some(b) = building_at(mx, my) else {
        return;
    };
    let part = if b
        .shop_zone
        .is_some_and(|(x0, x1, y0, y1)| mx >= x0 && mx <= x1 && my >= y0 && my <= y1)
    {
        "前店營業間"
    } else if b
        .home_zone
        .is_some_and(|(x0, x1, y0, y1)| mx >= x0 && mx <= x1 && my >= y0 && my <= y1)
    {
        "後宅私室"
    } else {
        b.kind_label
    };
    *title = format!("{}・{}", b.name, title);
    tags.insert(0, format!("{}｜{}", b.kind_label, b.household_note));
    tags.insert(1, part.to_string());
}

pub fn town_buildings_dto() -> Vec<Value> {
    TOWN_BUILDINGS
        .iter()
        .map(|b| {
            let (x0, x1, y0, y1) = b.bbox;
            json!({
                "code": b.code,
                "name": b.name,
                "kind": b.kind_label,
                "household_note": b.household_note,
                "x0": x0,
                "x1": x1,
                "y0": y0,
                "y1": y1,
                "label_x": (x0 + x1) as f32 * 0.5,
                "label_y": (y0 + y1) as f32 * 0.5,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::{HashSet, VecDeque};

    use super::*;
    use crate::grid::substrate::coords::SubstrateRegion;
    use crate::grid::substrate::l0_cell::{L0Cell, SubstrateMaterial, SurfaceWaterKind, landmass};
    use crate::grid::substrate::rim_seed::seed_outpost;
    use crate::grid::substrate::rim_shelter::{
        RimWeatherKind, detect_enclosed_rooms, sheltered_cell_set, step_weather_and_bed_recovery,
    };
    use crate::grid::substrate::rim_world::{NaturalProp, find_micro_path};

    #[test]
    fn six_substrate_zonation_eliminates_beach_bamboo_and_preserves_foothill_groves() {
        let fill = L0Cell::new(
            Some(30.0),
            landmass::DAJIA_WU_TABLELAND,
            SubstrateMaterial::AlluvialSandyLoam,
            SurfaceWaterKind::None,
            80.0,
            90,
        );
        let mut region = SubstrateRegion::new_filled(-10, -2, 20, 5, fill);
        region.set(
            -8,
            0,
            L0Cell::new(
                Some(0.5),
                landmass::DAJIA_WU_TABLELAND,
                SubstrateMaterial::IntertidalMudflat,
                SurfaceWaterKind::None,
                50.0,
                70,
            ),
        );
        for gx in [-7, -6] {
            region.set(
                gx,
                0,
                L0Cell::new(
                    Some(4.0),
                    landmass::DAJIA_WU_TABLELAND,
                    SubstrateMaterial::CoastalWindSand,
                    SurfaceWaterKind::None,
                    50.0,
                    75,
                ),
            );
        }
        region.set(
            -4,
            0,
            L0Cell::new(
                Some(8.0),
                landmass::DAJIA_WU_TABLELAND,
                SubstrateMaterial::AlluvialSandyLoam,
                SurfaceWaterKind::None,
                50.0,
                80,
            ),
        );
        region.set(
            0,
            0,
            L0Cell::new(
                Some(29.0),
                landmass::DAJIA_WU_TABLELAND,
                SubstrateMaterial::RiverbedQuartzCobble,
                SurfaceWaterKind::None,
                50.0,
                85,
            ),
        );
        region.set(
            4,
            0,
            L0Cell::new(
                Some(95.0),
                landmass::DAJIA_WU_TABLELAND,
                SubstrateMaterial::LateriteGravel,
                SurfaceWaterKind::None,
                50.0,
                85,
            ),
        );
        region.set(
            6,
            0,
            L0Cell::new(
                Some(120.0),
                landmass::DAJIA_WU_TABLELAND,
                SubstrateMaterial::FaultBareRock,
                SurfaceWaterKind::None,
                50.0,
                85,
            ),
        );
        let rt = SubstrateRuntime::from_region(region);

        assert_eq!(derive_micro_pristine(Some(&rt), -150, 5).0, MicroGround::TidalMudflat);
        assert_eq!(derive_micro_pristine(Some(&rt), -110, 5).0, MicroGround::CoastalSand);
        assert_eq!(derive_micro_pristine(Some(&rt), -70, 5).0, MicroGround::LoamGrass);
        assert_eq!(derive_micro_pristine(Some(&rt), 15, 2).0, MicroGround::PebbleBank);
        assert_eq!(derive_micro_pristine(Some(&rt), 90, 5).0, MicroGround::RedGravel);
        assert_eq!(derive_micro_pristine(Some(&rt), 130, 5).0, MicroGround::BareRock);

        // 海岸、泥灘與低海拔沖積帶（< 18m）及裸岩皆不生莿竹與春筍
        for mx in -160..=-101 {
            for my in 0..=18 {
                let (_, prop) = derive_micro_pristine(Some(&rt), mx, my);
                assert!(
                    !matches!(prop, Some(NaturalProp::BambooClump | NaturalProp::BambooShoot)),
                    "beach/mudflat ({mx},{my}) must not have bamboo or shoot"
                );
            }
        }
        // 局部湧泉溪僅限 -46..=38，遠離溪段處不該出現偽山澗
        assert_ne!(derive_micro_pristine(Some(&rt), -80, 11).0, MicroGround::ShallowStream);

        // 紅土丘陵台地仍保有多簇莿竹林
        let rt_ref = &rt;
        let foothill_bamboos = (80..=99)
            .flat_map(|x| (0..=19).map(move |y| derive_micro_pristine(Some(rt_ref), x, y).1))
            .filter(|p| matches!(p, Some(NaturalProp::BambooClump)))
            .count();
        assert!(foothill_bamboos > 0);
    }

    #[test]
    fn shophouses_and_residences_have_front_back_rooms_and_bijective_household_beds() {
        let (overlays, actors, _) = seed_outpost(None);
        let rooms = detect_enclosed_rooms(&overlays);
        assert!(rooms.iter().any(|r| r.x0 == 7 && r.x1 == 11 && r.y0 == 1 && r.y1 == 5));
        assert!(rooms.iter().any(|r| r.x0 == 7 && r.x1 == 11 && r.y0 == -3 && r.y1 == 1));
        assert!(rooms.iter().any(|r| r.x0 == 14 && r.x1 == 19 && r.y0 == -3 && r.y1 == 1));
        assert!(rooms.iter().any(|r| r.x0 == 14 && r.x1 == 19 && r.y0 == 1 && r.y1 == 5));
        assert!(rooms.iter().any(|r| r.x0 == -20 && r.x1 == -15 && r.y0 == -3 && r.y1 == 1));
        assert!(rooms.iter().any(|r| r.x0 == -20 && r.x1 == -15 && r.y0 == 1 && r.y1 == 5));
        assert!(rooms.iter().any(|r| r.x0 == -20 && r.x1 == -15 && r.y0 == -9 && r.y1 == -7));
        assert!(rooms.iter().any(|r| r.x0 == -20 && r.x1 == -15 && r.y0 == -12 && r.y1 == -9));
        assert!(rooms.iter().any(|r| r.x0 == 10 && r.x1 == 14 && r.y0 == -12 && r.y1 == -7));

        let mut seen_keys = HashSet::new();
        let mut seen_beds = HashSet::new();
        for b in &TOWN_BUILDINGS {
            for &(key, pos) in b.owner_beds {
                assert!(seen_keys.insert(key), "duplicate resident key {key}");
                assert!(seen_beds.insert(pos), "duplicate household bed {pos:?}");
                assert_eq!(
                    overlays.get(&pos).and_then(|o| o.structure),
                    Some(StructureKind::BambooBed),
                    "missing bed for {key} at {pos:?}"
                );
                assert!(
                    actors.iter().any(|a| a.actor_key == key),
                    "missing resident {key} in seeded actors"
                );
                assert!(
                    find_micro_path(None, &overlays, (0, -2), pos, true, false).is_some(),
                    "cannot reach resident bed {key} at {pos:?}"
                );
            }
        }
        assert_eq!(seen_keys.len(), 15);
        assert_eq!(clinic_bed_near(&overlays), Some((18, -1)));
        assert!(
            overlays.get(&(-16, -10)).and_then(|o| o.structure) == Some(StructureKind::BambooBed)
                || overlays.get(&(-16, -11)).and_then(|o| o.structure) == Some(StructureKind::BambooBed)
        );
    }

    #[test]
    fn covered_arcade_shields_from_rain() {
        let (overlays, mut actors, _) = seed_outpost(None);
        assert_eq!(
            overlays.get(&(9, -4)).and_then(|o| o.structure),
            Some(StructureKind::CoveredArcade)
        );
        let sheltered = sheltered_cell_set(&overlays);
        assert!(sheltered.contains(&(9, -4)));
        assert!(!sheltered.contains(&(2, 0)));

        let mut under_arcade = actors.remove(0);
        under_arcade.px = 9.0;
        under_arcade.py = -4.0;
        under_arcade.equipped_hat = false;
        under_arcade.wet_ticks = 20;

        let mut open_plaza = actors.remove(0);
        open_plaza.px = 2.0;
        open_plaza.py = 0.0;
        open_plaza.equipped_hat = false;
        open_plaza.wet_ticks = 0;

        let mut world = RimWorldState {
            tick: 24,
            overlays,
            actors: vec![under_arcade, open_plaza],
            events: VecDeque::new(),
            economy: Default::default(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
        };
        step_weather_and_bed_recovery(&mut world, RimWeatherKind::SummerSquall);
        assert!(world.actors[0].wet_ticks < 20);
        assert!(world.actors[1].wet_ticks > 0);
    }
}

//! 西麓湧泉聚落前哨（West-Foot Pioneer Outpost）種子與升級補植。

use std::collections::{HashMap, VecDeque};

use super::rim_sim::{ActorFaction, PendingTask, RimActor, ShortEvent};
use super::rim_world::{
    GroundStack, MicroCellOverlay, MicroGround, RimItemKind, StructureKind, derive_micro_pristine,
};
use super::runtime::SubstrateRuntime;

/// 冪等鋪設西麓湧泉廣場、東側竹編草寮 `(7..=11, 1..=5)` 與北澗渡溪窄橋。
pub fn apply_outpost_architecture(
    rt: Option<&SubstrateRuntime>,
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
) {
    // 1. 湧泉廣場核心：(0,0) 營火、(4,-1) 工作台；確保 (-5..=5, -3..=3) 其餘暢通
    overlays
        .entry((0, 0))
        .or_insert_with(|| MicroCellOverlay::cleared(Some(StructureKind::Campfire)));
    overlays
        .entry((4, -1))
        .or_insert_with(|| MicroCellOverlay::cleared(Some(StructureKind::Workbench)));

    // 2. 東側竹編草寮 (7..=11, 1..=5)，西側 (7,3) 設竹編院門
    for x in 7..=11 {
        for y in 1..=5 {
            let is_border = x == 7 || x == 11 || y == 1 || y == 5;
            let st = is_border.then(|| {
                if x == 7 && y == 3 {
                    StructureKind::BambooDoor
                } else {
                    StructureKind::BambooWall
                }
            });
            overlays
                .entry((x, y))
                .or_insert_with(|| MicroCellOverlay::cleared(st));
        }
    }
    // 門前 (6,3) 清道 & 門外南側 (6,1) 夜巡暖爐營火
    overlays
        .entry((6, 3))
        .or_insert_with(|| MicroCellOverlay::cleared(None));
    overlays
        .entry((6, 1))
        .or_insert_with(|| MicroCellOverlay::cleared(Some(StructureKind::Campfire)));
    // 室內案台與物資儲位
    overlays
        .entry((10, 4))
        .or_insert_with(|| MicroCellOverlay::cleared(Some(StructureKind::Workbench)));
    overlays.entry((10, 2)).or_insert_with(|| MicroCellOverlay {
        cleared_natural: true,
        structure: Some(StructureKind::Stockpile),
        blueprint: None,
        ground_item: Some(GroundStack {
            item: RimItemKind::BambooShoot,
            count: 3,
        }),
        eco_ticks: 0,
    });
    overlays.entry((9, 2)).or_insert_with(|| MicroCellOverlay {
        cleared_natural: true,
        structure: Some(StructureKind::Stockpile),
        blueprint: None,
        ground_item: Some(GroundStack {
            item: RimItemKind::BambooPole,
            count: 4,
        }),
        eco_ticks: 0,
    });
    for bed_pos in [(8, 4), (8, 2)] {
        let e = overlays.entry(bed_pos).or_default();
        if e.structure.is_none() && e.ground_item.is_none() {
            e.cleared_natural = true;
            e.structure = Some(StructureKind::BambooBed);
        }
    }

    // 3. 北澗渡溪木橋：沿 mx=0 偵測 ShallowStream 自動鋪設 WoodBridge，並打通南北兩岸林木
    for my in 3..=18 {
        let (g, _) = derive_micro_pristine(rt, 0, my);
        if g == MicroGround::ShallowStream {
            overlays
                .entry((0, my))
                .or_insert_with(|| MicroCellOverlay::cleared(Some(StructureKind::WoodBridge)));
        } else if my >= 7 && !g.is_water() && g != MicroGround::SteepCliff {
            overlays
                .entry((0, my))
                .or_insert_with(|| MicroCellOverlay::cleared(None));
        }
    }

    apply_shuoweiliao_outpost(rt, overlays);
    let _ = super::rim_ecology::reconcile_real_scale_layout_and_clutter(rt, overlays);
    for (pos, item, count) in [
        ((-13, -3), RimItemKind::BambooPole, 9),
        ((-12, -3), RimItemKind::RiverStone, 2),
    ] {
        if let Some(ov) = overlays.get_mut(&pos)
            && ov.structure == Some(StructureKind::Stockpile)
            && ov.ground_item.is_none()
        {
            ov.ground_item = Some(GroundStack { item, count });
        }
    }
    super::rim_town::apply_town_architecture(rt, overlays);
}

/// 冪等鋪設東南向小徑與「水尾寮・溪畔茶寮」(mx: 28..=36, my: -10..=-4)。
pub fn apply_shuoweiliao_outpost(
    rt: Option<&SubstrateRuntime>,
    overlays: &mut HashMap<(i32, i32), MicroCellOverlay>,
) {
    for mx in 6..=28 {
        for my in -8..=-2 {
            let on_trail = (mx <= 18 && my == -3)
                || (mx == 18 && (-7..=-3).contains(&my))
                || (mx >= 18 && my == -7);
            if !on_trail {
                continue;
            }
            let (g, _) = derive_micro_pristine(rt, mx, my);
            if g == MicroGround::ShallowStream {
                overlays
                    .entry((mx, my))
                    .or_insert_with(|| MicroCellOverlay::cleared(Some(StructureKind::WoodBridge)));
            } else if !g.is_water() && g != MicroGround::SteepCliff {
                overlays
                    .entry((mx, my))
                    .or_insert_with(|| MicroCellOverlay::cleared(None));
            }
        }
    }
    for x in 28..=36 {
        for y in -10..=-4 {
            let (g, _) = derive_micro_pristine(rt, x, y);
            if g == MicroGround::DeepWater || g == MicroGround::SteepCliff {
                continue;
            }
            if g == MicroGround::ShallowStream || g == MicroGround::SpringPool {
                overlays
                    .entry((x, y))
                    .or_insert_with(|| MicroCellOverlay::cleared(Some(StructureKind::WoodBridge)));
                continue;
            }
            let is_border = x == 28 || x == 36 || y == -10 || y == -4;
            let st = if is_border {
                if (x == 32 && y == -4) || (x == 28 && y == -7) {
                    Some(StructureKind::BambooDoor)
                } else {
                    Some(StructureKind::BambooWall)
                }
            } else if x == 31 && y == -7 {
                Some(StructureKind::Campfire)
            } else if x == 34 && y == -6 {
                Some(StructureKind::Workbench)
            } else if x == 34 && y == -8 {
                Some(StructureKind::Stockpile)
            } else if (x == 35 && y == -5) || (x == 35 && y == -9) {
                Some(StructureKind::BambooBed)
            } else {
                None
            };
            let ground_item = if x == 34 && y == -8 {
                Some(GroundStack {
                    item: RimItemKind::RoastedShoot,
                    count: 3,
                })
            } else {
                None
            };
            overlays.entry((x, y)).or_insert_with(|| MicroCellOverlay {
                cleared_natural: true,
                structure: st,
                blueprint: None,
                ground_item,
                eco_ticks: 0,
            });
            if ((x == 28 && y == -7) || (x == 35 && (y == -5 || y == -9)) || (x == 31 && y == -7))
                && let Some(ov) = overlays.get_mut(&(x, y))
                && ov.structure.is_none()
            {
                ov.cleared_natural = true;
                ov.structure = st;
            }
        }
    }
    let _ = super::rim_ecology::reconcile_real_scale_layout_and_clutter(rt, overlays);
}

struct ResidentSpec<'a> {
    key: &'a str,
    glyph: &'a str,
    name: &'a str,
    gender: &'a str,
    seed: i64,
    role: &'a str,
    faction: ActorFaction,
    tool: Option<RimItemKind>,
    pos: (f32, f32),
    facing: f32,
    hp: i32,
    dir: &'a str,
    act: &'a str,
    magnesium: u32,
    hat: bool,
    pack: bool,
    inv: &'a [(RimItemKind, u16)],
}

/// 確保聚落常駐居民與生態異獸齊備（不寫入 `entities` 表，不污染主世契約）。
pub fn ensure_outpost_residents(actors: &mut Vec<RimActor>) {
    // 清理早期無 entity_id 的佔位主角 ("player")
    actors.retain(|a| !(a.entity_id.is_none() && (a.code == "player" || a.is_player)));

    let mut next_id = actors.iter().map(|a| a.id).max().unwrap_or(1) + 1;
    let specs: [ResidentSpec<'_>; 18] = [
        ResidentSpec {
            key: "res:lin_amu",
            glyph: "鍛",
            name: "林阿木",
            gender: "M",
            seed: 108_421,
            role: "鐵店師傅",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::AlloyPick),
            pos: (9.0, -1.0),
            facing: 0.0,
            hp: 105,
            dir: "assist",
            act: "打鐵校刃",
            magnesium: 36,
            hat: false,
            pack: true,
            inv: &[(RimItemKind::BambooPole, 2), (RimItemKind::RiverStone, 2)],
        },
        ResidentSpec {
            key: "res:zhang_xiaoan",
            glyph: "安",
            name: "張小安",
            gender: "M",
            seed: 108_907,
            role: "鐵店學徒",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::StonePick),
            pos: (10.0, 0.0),
            facing: 1.57,
            hp: 90,
            dir: "assist",
            act: "拉風鼓火",
            magnesium: 18,
            hat: false,
            pack: true,
            inv: &[(RimItemKind::RiverStone, 2), (RimItemKind::BambooPole, 1)],
        },
        ResidentSpec {
            key: "res:lin_sanshui",
            glyph: "商",
            name: "林三水",
            gender: "M",
            seed: 311_209,
            role: "商行掌櫃",
            faction: ActorFaction::Settler,
            tool: None,
            pos: (-10.0, 0.0),
            facing: -1.57,
            hp: 92,
            dir: "patrol",
            act: "檢核貨架",
            magnesium: 64,
            hat: false,
            pack: true,
            inv: &[(RimItemKind::RoastedShoot, 2), (RimItemKind::BambooPole, 2)],
        },
        ResidentSpec {
            key: "res:gao_afa",
            glyph: "夥",
            name: "高阿發",
            gender: "M",
            seed: 311_743,
            role: "商行夥計",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::StonePick),
            pos: (-8.0, 0.0),
            facing: 0.0,
            hp: 94,
            dir: "assist",
            act: "盤點山貨",
            magnesium: 24,
            hat: true,
            pack: true,
            inv: &[(RimItemKind::BambooShoot, 2), (RimItemKind::BambooPole, 1)],
        },
        ResidentSpec {
            key: "res:liu_chunshui",
            glyph: "廚",
            name: "劉春水",
            gender: "F",
            seed: 412_517,
            role: "小吃掌杓",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::AlloyBlade),
            pos: (-17.0, -1.0),
            facing: -1.57,
            hp: 95,
            dir: "assist",
            act: "灶畔煮湯",
            magnesium: 38,
            hat: false,
            pack: false,
            inv: &[
                (RimItemKind::RoastedShoot, 3),
                (RimItemKind::RoastedTuber, 2),
            ],
        },
        ResidentSpec {
            key: "res:liu_abao",
            glyph: "寶",
            name: "劉阿寶",
            gender: "M",
            seed: 412_991,
            role: "小吃幫廚",
            faction: ActorFaction::Settler,
            tool: None,
            pos: (-18.0, 0.0),
            facing: 1.2,
            hp: 85,
            dir: "assist",
            act: "招呼茶食",
            magnesium: 14,
            hat: false,
            pack: true,
            inv: &[(RimItemKind::BambooShoot, 2), (RimItemKind::SweetTuber, 2)],
        },
        ResidentSpec {
            key: "res:zhang_jian",
            glyph: "醫",
            name: "張濟安",
            gender: "M",
            seed: 523_103,
            role: "診所醫師",
            faction: ActorFaction::Settler,
            tool: None,
            pos: (16.0, -1.0),
            facing: -1.57,
            hp: 96,
            dir: "patrol",
            act: "診間候診",
            magnesium: 45,
            hat: false,
            pack: false,
            inv: &[(RimItemKind::WoundSalve, 3), (RimItemKind::MountainHerb, 2)],
        },
        ResidentSpec {
            key: "res:zhang_suxin",
            glyph: "藥",
            name: "張素心",
            gender: "F",
            seed: 523_679,
            role: "採藥醫助",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::AlloyBlade),
            pos: (17.0, 0.0),
            facing: 0.4,
            hp: 92,
            dir: "assist",
            act: "分揀藥草",
            magnesium: 26,
            hat: true,
            pack: true,
            inv: &[(RimItemKind::MountainHerb, 3), (RimItemKind::WoundSalve, 1)],
        },
        ResidentSpec {
            key: "res:xie_chengfeng",
            glyph: "宿",
            name: "謝承風",
            gender: "M",
            seed: 604_127,
            role: "宿屋主人",
            faction: ActorFaction::Settler,
            tool: None,
            pos: (-18.0, -9.0),
            facing: 1.57,
            hp: 95,
            dir: "patrol",
            act: "整理客榻",
            magnesium: 34,
            hat: false,
            pack: false,
            inv: &[(RimItemKind::RoastedShoot, 2), (RimItemKind::BambooPole, 1)],
        },
        ResidentSpec {
            key: "res:xu_qiuniang",
            glyph: "農",
            name: "許秋娘",
            gender: "F",
            seed: 517_309,
            role: "種薯農婦",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::AlloyPick),
            pos: (-8.0, -9.0),
            facing: 0.0,
            hp: 92,
            dir: "assist",
            act: "照看薯田",
            magnesium: 24,
            hat: true,
            pack: true,
            inv: &[(RimItemKind::SweetTuber, 3), (RimItemKind::RoastedTuber, 1)],
        },
        ResidentSpec {
            key: "res:xu_dabo",
            glyph: "伯",
            name: "許大伯",
            gender: "M",
            seed: 517_883,
            role: "老農",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::StonePick),
            pos: (-9.0, -9.0),
            facing: 0.3,
            hp: 88,
            dir: "assist",
            act: "疏通田溝",
            magnesium: 20,
            hat: true,
            pack: false,
            inv: &[
                (RimItemKind::BambooShoot, 2),
                (RimItemKind::MountainHerb, 1),
            ],
        },
        ResidentSpec {
            key: "res:xu_xiaoman",
            glyph: "滿",
            name: "許小滿",
            gender: "F",
            seed: 518_219,
            role: "農戶少女",
            faction: ActorFaction::Settler,
            tool: None,
            pos: (-10.0, -9.0),
            facing: 1.57,
            hp: 86,
            dir: "assist",
            act: "曬草收薯",
            magnesium: 15,
            hat: false,
            pack: true,
            inv: &[(RimItemKind::SweetTuber, 2), (RimItemKind::RoastedShoot, 1)],
        },
        ResidentSpec {
            key: "res:chen_qing",
            glyph: "獵",
            name: "陳青",
            gender: "F",
            seed: 294_715,
            role: "巡山獵戶",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::AlloyBlade),
            pos: (12.0, -9.0),
            facing: 1.57,
            hp: 108,
            dir: "patrol",
            act: "巡哨山徑",
            magnesium: 28,
            hat: true,
            pack: true,
            inv: &[(RimItemKind::RoastedMeat, 2), (RimItemKind::BeastMeat, 1)],
        },
        ResidentSpec {
            key: "res:zhou_acha",
            glyph: "茶",
            name: "周阿茶",
            gender: "F",
            seed: 642_817,
            role: "茶寮掌櫃",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::BambooBlade),
            pos: (30.0, -7.0),
            facing: 1.57,
            hp: 95,
            dir: "patrol",
            act: "照看茶爐",
            magnesium: 36,
            hat: false,
            pack: false,
            inv: &[
                (RimItemKind::RoastedShoot, 3),
                (RimItemKind::RoastedMeat, 2),
                (RimItemKind::BambooShoot, 2),
            ],
        },
        ResidentSpec {
            key: "res:he_dashu",
            glyph: "焙",
            name: "何大樹",
            gender: "M",
            seed: 643_111,
            role: "茶寮焙師",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::StonePick),
            pos: (32.0, -6.0),
            facing: 3.14,
            hp: 98,
            dir: "assist",
            act: "翻曬山茶",
            magnesium: 24,
            hat: true,
            pack: true,
            inv: &[(RimItemKind::BambooPole, 2), (RimItemKind::MountainHerb, 2)],
        },
        ResidentSpec {
            key: "res:zheng_shitou",
            glyph: "商",
            name: "鄭石頭",
            gender: "M",
            seed: 781_903,
            role: "行腳貨郎",
            faction: ActorFaction::Settler,
            tool: Some(RimItemKind::StonePick),
            pos: (33.0, -6.0),
            facing: 3.14,
            hp: 100,
            dir: "patrol",
            act: "檢點行貨",
            magnesium: 48,
            hat: true,
            pack: true,
            inv: &[
                (RimItemKind::BambooHat, 2),
                (RimItemKind::BambooPack, 2),
                (RimItemKind::StonePick, 1),
                (RimItemKind::RoastedShoot, 2),
            ],
        },
        ResidentSpec {
            key: "beast:boar_init",
            glyph: "豚",
            name: "紅土野山豬",
            gender: "M",
            seed: 70_001,
            role: "山海野獸",
            faction: ActorFaction::WildBeast,
            tool: None,
            pos: (5.0, 21.0),
            facing: -1.57,
            hp: 80,
            dir: "",
            act: "拱土覓食",
            magnesium: 0,
            hat: false,
            pack: false,
            inv: &[],
        },
        ResidentSpec {
            key: "beast:pheasant_init",
            glyph: "雉",
            name: "鐵羽竹雞",
            gender: "F",
            seed: 70_002,
            role: "竹林山禽",
            faction: ActorFaction::WildBeast,
            tool: None,
            pos: (-24.0, 9.0),
            facing: 0.4,
            hp: 55,
            dir: "",
            act: "林緣啄食",
            magnesium: 0,
            hat: false,
            pack: false,
            inv: &[],
        },
    ];

    for spec in specs {
        if let Some(existing) = actors
            .iter_mut()
            .find(|a| a.actor_key == spec.key || (a.entity_id.is_none() && a.name == spec.name))
        {
            if existing.soul_seed == 0 {
                existing.actor_key = spec.key.into();
                existing.code = spec.key.into();
                existing.gender = spec.gender.into();
                existing.soul_seed = spec.seed;
                existing.role_tag = spec.role.into();
            } else if existing.role_tag == "百工木匠"
                || existing.role_tag == "採筍山民"
                || existing.role_tag == "茶寮庖廚"
            {
                existing.role_tag = spec.role.into();
            }
            if existing.glyph == "匠" {
                existing.glyph = spec.glyph.into();
            }
            if existing.hunger == 0 && existing.thirst == 0 {
                existing.hunger = 86;
                existing.thirst = 88;
                existing.stamina = 90;
            }
            if existing.faction == ActorFaction::Settler && existing.inventory.is_empty() {
                existing.inventory = spec
                    .inv
                    .iter()
                    .map(|&(item, count)| GroundStack { item, count })
                    .collect();
                existing.magnesium = existing.magnesium.max(spec.magnesium);
                existing.equipped_hat = spec.hat;
                existing.equipped_pack = spec.pack;
            } else if existing.faction == ActorFaction::WildBeast {
                existing.equipped_hat = false;
                existing.equipped_pack = false;
                existing.magnesium = 0;
                if super::rim_ecology::is_settlement_keep_clear(
                    existing.px.round() as i32,
                    existing.py.round() as i32,
                ) {
                    existing.px = spec.pos.0;
                    existing.py = spec.pos.1;
                }
            }
            if existing.civic.tool_maker.is_none() && existing.equipped_tool.is_some() {
                existing.civic.tool_maker = Some(if spec.key == "res:lin_amu" {
                    "林阿木自鍛".into()
                } else {
                    "阿木鐵店・林阿木鍛造".into()
                });
            }
            continue;
        }
        let tool_maker = spec.tool.map(|_| {
            if spec.key == "res:lin_amu" {
                "林阿木自鍛".to_string()
            } else {
                "阿木鐵店・林阿木鍛造".to_string()
            }
        });
        actors.push(RimActor {
            id: next_id,
            code: spec.key.into(),
            actor_key: spec.key.into(),
            entity_id: None,
            glyph: spec.glyph.into(),
            name: spec.name.into(),
            gender: spec.gender.into(),
            soul_seed: spec.seed,
            role_tag: spec.role.into(),
            faction: spec.faction,
            is_player: false,
            autopilot: true,
            running: false,
            px: spec.pos.0,
            py: spec.pos.1,
            facing_rad: spec.facing,
            hp: spec.hp,
            max_hp: spec.hp,
            equipped_tool: spec.tool,
            carrying: None,
            inventory: spec
                .inv
                .iter()
                .map(|&(item, count)| GroundStack { item, count })
                .collect(),
            directive: spec.dir.into(),
            path: VecDeque::new(),
            task: PendingTask::None,
            activity_tag: spec.act.into(),
            pop_text: None,
            cooldown_ticks: 0,
            last_active_unix: 0,
            hunger: 88,
            thirst: 90,
            stamina: 92,
            magnesium: spec.magnesium,
            equipped_hat: spec.hat,
            equipped_pack: spec.pack,
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
            caravan_leg: if spec.role.contains("貨郎") { 1 } else { 0 },
            wet_ticks: 0,
            recent_diary: VecDeque::new(),
            civic: super::rim_shops::ActorCivicState {
                tool_maker,
                ..Default::default()
            },
        });
        next_id += 1;
    }
}

pub fn seed_outpost(
    rt: Option<&SubstrateRuntime>,
) -> (
    HashMap<(i32, i32), MicroCellOverlay>,
    Vec<RimActor>,
    VecDeque<ShortEvent>,
) {
    let mut overlays = HashMap::new();
    apply_outpost_architecture(rt, &mut overlays);
    let mut actors = Vec::new();
    ensure_outpost_residents(&mut actors);
    let mut events = VecDeque::new();
    events.push_back(ShortEvent {
        tick: 0,
        text: "山風拂過竹梢，湧泉水聲清淺".into(),
    });
    (overlays, actors, events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::rim_world::find_micro_path;

    #[test]
    fn outpost_bridge_crosses_north_stream_and_door_blocks_beasts() {
        let (overlays, actors, _) = seed_outpost(None);
        // 廣場核心動線點暢通
        assert!(find_micro_path(None, &overlays, (0, -2), (4, -2), false, false).is_some());
        assert!(find_micro_path(None, &overlays, (0, -2), (-4, 2), false, false).is_some());
        // 北澗木橋已連通，人物可直接渡至北岸 (0, 18)
        assert!(find_micro_path(None, &overlays, (0, -2), (0, 18), false, false).is_some());
        // 東側竹編草寮：人類可經竹編門 (7, 3) 進入室內 (9, 3)，野獸不可入內
        assert!(find_micro_path(None, &overlays, (0, -2), (9, 3), false, false).is_some());
        assert!(find_micro_path(None, &overlays, (0, -2), (9, 3), false, true).is_none());
        // 東南向小徑直通水尾寮・溪畔茶寮 (32, -7)
        assert!(find_micro_path(None, &overlays, (0, -2), (32, -7), false, false).is_some());
        // 常駐聚落居民具備性別與多樣靈魂種子
        assert!(actors.iter().any(|a| a.name == "許秋娘" && a.gender == "F"));
        assert!(actors.iter().any(|a| a.name == "林阿木" && a.soul_seed > 0));
        assert!(actors.iter().any(|a| a.name == "周阿茶" && a.gender == "F"));
        assert!(actors.iter().any(|a| a.name == "鄭石頭" && a.equipped_hat));
    }
}

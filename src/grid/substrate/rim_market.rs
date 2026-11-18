//! 聚落生計效用排序、自動赴倉交割與跨聚落雙向空間套利排程模組。

use super::rim_ecology::{FarmOp, enqueue_contiguous_parcel};
use super::rim_economy::{
    ALL_ITEMS, SettlementId, buy_from_storehouse, find_best_stockpile_slot_for, quote_item,
    sell_to_storehouse, settlement_of_pos,
};
use super::rim_sim::{ActorFaction, PendingTask, RimWorldState};
use super::rim_world::{
    NaturalProp, RimItemKind, StructureKind, derive_micro_pristine, find_micro_path,
    find_micro_path_crowded,
};
use super::runtime::SubstrateRuntime;

/// 聚落近身護衛（防禦入營野獸）。
pub fn try_guard_or_market_livelihood(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    start: (i32, i32),
    home_pt: (i32, i32),
) -> bool {
    let dir_now = world.actors[idx].directive.as_str();
    let close_r: f32 = if dir_now == "rest" { 5.0 } else { 10.5 };
    if let Some(beast) = world.actors.iter().find(|b| {
        b.faction == ActorFaction::WildBeast
            && b.hp > 0
            && (b.px - home_pt.0 as f32).hypot(b.py - home_pt.1 as f32) <= close_r
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

/// 以個人邊際時薪效用比較打獵、成片農耕、庖廚炙烤、百工造具與採集，選出最合時宜之生計。
pub fn try_assign_market_livelihood(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    start: (i32, i32),
    home: (i32, i32),
    claimed: &[(i32, i32)],
    occ: &[(i32, i32)],
) -> bool {
    if world.actors[idx].directive == "patrol" || world.actors[idx].directive == "rest" {
        return false;
    }
    let s = settlement_of_pos(start.0, start.1);

    // 1. 優先向倉廩交割隨身餘貨，回流貨款並釋放行囊空間
    let sellable_inv: u16 = world.actors[idx]
        .inventory
        .iter()
        .filter(|st| {
            st.count > u16::from(st.item.is_edible())
                && quote_item(world, s, st.item).bid_mg > 0
                && !st.item.is_hand_tool()
        })
        .map(|st| st.count)
        .sum();
    if sellable_inv >= 3
        && let Some((sx, sy)) =
            find_best_stockpile_slot_for(&world.overlays, start, RimItemKind::BeastMeat, Some(s))
        && let Some(p) = find_micro_path_crowded(rt, &world.overlays, start, (sx, sy), true, false, occ)
    {
        world.actors[idx].path = p.into();
        world.actors[idx].task = PendingTask::StorehouseTrade { tx: sx, ty: sy };
        world.actors[idx].activity_tag = "赴倉交貨".into();
        return true;
    }

    // 工具磨損遺失時若有鎂優先向倉廩購換
    if world.actors[idx].equipped_tool.is_none() {
        for tool in [RimItemKind::StonePick, RimItemKind::BambooBlade] {
            if world.actors[idx].magnesium >= quote_item(world, s, tool).ask_mg
                && let Some((&kpos, _)) = world
                    .overlays
                    .iter()
                    .filter(|((mx, my), ov)| {
                        settlement_of_pos(*mx, *my) == s
                            && ov.structure == Some(StructureKind::Stockpile)
                            && ov.ground_item.is_some_and(|g| g.item == tool && g.count > 0)
                    })
                    .min_by_key(|(pos, _)| (pos.0 - start.0).abs() + (pos.1 - start.1).abs())
                && let Some(p) = find_micro_path_crowded(rt, &world.overlays, start, kpos, true, false, occ)
            {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::StorehouseTrade {
                    tx: kpos.0,
                    ty: kpos.1,
                };
                world.actors[idx].activity_tag = "赴倉購具".into();
                return true;
            }
        }
    }

    // 2. 評估各項生計邊際每刻收益 U_j(a)
    #[derive(Clone, Copy)]
    enum Choice {
        Hunt(usize, (i32, i32)),
        Farm((i32, i32), FarmOp),
        Cook((i32, i32), RimItemKind),
        Craft((i32, i32), RimItemKind),
        Forage((i32, i32)),
    }
    let mut best: Option<(f32, Choice)> = None;
    let mut consider = |score: f32, ch: Choice| {
        if score > 0.05 && best.as_ref().is_none_or(|(bs, _)| score > *bs) {
            best = Some((score, ch));
        }
    };

    let hunt_apt = 1.0
        + f32::from(world.actors[idx].directive == "guard") * 0.45
        + (world.actors[idx].practice_blade as f32 * 0.04).min(0.4);
    let farm_apt = 1.0
        + (world.actors[idx].craft_harvest as f32 * 0.04).min(0.45)
        + f32::from(world.actors[idx].directive == "forage") * 0.25;
    let cook_apt = 1.0 + (world.actors[idx].craft_culinary as f32 * 0.05).min(0.5);
    let forge_apt = 1.0
        + f32::from(world.actors[idx].directive == "craft") * 0.40
        + (world.actors[idx].craft_forge as f32 * 0.05).min(0.5);

    // A) 按真實肉價出獵（不設任何寫死配額，過剩時 bid=0 自然退出）
    let meat_bid = quote_item(world, s, RimItemKind::BeastMeat).bid_mg as f32;
    if meat_bid > 0.0
        && let Some(b) = world
            .actors
            .iter()
            .filter(|b| {
                b.faction == ActorFaction::WildBeast
                    && b.hp > 0
                    && (b.px - home.0 as f32).hypot(b.py - home.1 as f32) <= 28.0
            })
            .min_by_key(|b| ((b.px - start.0 as f32).hypot(b.py - start.1 as f32) * 10.0) as i32)
    {
        let dist = (b.px - start.0 as f32).hypot(b.py - start.1 as f32).max(2.0);
        let sc = ((meat_bid * 2.0) / (8.0 + dist)) * hunt_apt;
        consider(sc, Choice::Hunt(b.id, b.micro_pos()));
    }

    // B) 成片田畦收成與補種
    let herb_bid = quote_item(world, s, RimItemKind::MountainHerb).bid_mg as f32;
    let tuber_bid = quote_item(world, s, RimItemKind::SweetTuber).bid_mg as f32;
    if let Some((&pos, ov)) = world
        .overlays
        .iter()
        .filter(|&(&pos, ov)| {
            matches!(
                ov.structure,
                Some(StructureKind::HerbMature | StructureKind::CropMature | StructureKind::TilledRidge)
            ) && !claimed.contains(&pos)
                && (pos.0 - home.0).abs() + (pos.1 - home.1).abs() <= 26
        })
        .max_by_key(|&(&pos, ov)| {
            let p_val = match ov.structure {
                Some(StructureKind::HerbMature) => (herb_bid * 20.0) as i32,
                Some(StructureKind::CropMature) => (tuber_bid * 20.0) as i32,
                Some(StructureKind::TilledRidge) => (herb_bid.max(tuber_bid) * 10.0) as i32,
                _ => 0,
            };
            p_val - ((pos.0 - start.0).abs() + (pos.1 - start.1).abs())
        })
    {
        let (op, val) = match ov.structure {
            Some(StructureKind::HerbMature) => (FarmOp::Harvest, herb_bid * 2.0),
            Some(StructureKind::CropMature) => (FarmOp::Harvest, tuber_bid * 2.0),
            _ if herb_bid >= tuber_bid => (FarmOp::PlantHerb, herb_bid * 1.15),
            _ => (FarmOp::PlantTuber, tuber_bid * 1.15),
        };
        let d = ((pos.0 - start.0).abs() + (pos.1 - start.1).abs()) as f32;
        consider((val / (5.0 + d)) * farm_apt, Choice::Farm(pos, op));
    }

    // C) 庖廚加工毛利
    if let Some((&fpos, _)) = world
        .overlays
        .iter()
        .filter(|((mx, my), ov)| {
            settlement_of_pos(*mx, *my) == s && ov.structure == Some(StructureKind::Campfire)
        })
        .min_by_key(|&(&p, _)| (p.0 - start.0).abs() + (p.1 - start.1).abs())
    {
        let d = ((fpos.0 - start.0).abs() + (fpos.1 - start.1).abs()) as f32;
        for (raw, cooked) in [
            (RimItemKind::BeastMeat, RimItemKind::RoastedMeat),
            (RimItemKind::SweetTuber, RimItemKind::RoastedTuber),
            (RimItemKind::BambooShoot, RimItemKind::RoastedShoot),
        ] {
            let have_raw = world.actors[idx].inventory.iter().any(|x| x.item == raw && x.count > 0)
                || world.find_nearest_ground_item(fpos, raw, false).is_some();
            if have_raw {
                let c_bid = quote_item(world, s, cooked).bid_mg as f32;
                let r_ref = quote_item(world, s, raw).bid_mg as f32;
                let margin = (c_bid - r_ref * 0.7).max(0.0);
                consider((margin / (4.0 + d)) * cook_apt, Choice::Cook(fpos, raw));
            }
        }
    }

    // D) 百工工作台造具／搗藥毛利
    if let Some((&wpos, _)) = world
        .overlays
        .iter()
        .filter(|((mx, my), ov)| {
            settlement_of_pos(*mx, *my) == s && ov.structure == Some(StructureKind::Workbench)
        })
        .min_by_key(|&(&p, _)| (p.0 - start.0).abs() + (p.1 - start.1).abs())
    {
        let d = ((wpos.0 - start.0).abs() + (wpos.1 - start.1).abs()) as f32;
        for (prod, mat) in [
            (RimItemKind::WoundSalve, RimItemKind::MountainHerb),
            (RimItemKind::BambooBlade, RimItemKind::BambooPole),
            (RimItemKind::StonePick, RimItemKind::BambooPole),
            (RimItemKind::BambooHat, RimItemKind::BambooPole),
        ] {
            let have_mat = world.actors[idx].inventory.iter().any(|x| x.item == mat && x.count > 0)
                || world.find_nearest_ground_item(wpos, mat, false).is_some();
            if have_mat {
                let p_bid = quote_item(world, s, prod).bid_mg as f32;
                let m_ref = quote_item(world, s, mat).bid_mg as f32;
                let margin = (p_bid - m_ref * 0.6).max(0.0);
                consider((margin / (5.0 + d)) * forge_apt, Choice::Craft(wpos, prod));
            }
        }
    }

    // E) 山林採筍／採野生青草藥
    if let Some(pos) = (-18..=18)
        .flat_map(|dy| (-18..=18).map(move |dx| (home.0 + dx, home.1 + dy)))
        .filter(|p| !claimed.contains(p))
        .find(|&(cx, cy)| {
            let cleared = world
                .overlays
                .get(&(cx, cy))
                .is_some_and(|o| o.cleared_natural || o.structure.is_some());
            !cleared
                && matches!(
                    derive_micro_pristine(rt, cx, cy).1,
                    Some(NaturalProp::BambooShoot | NaturalProp::WildHerb)
                )
        })
    {
        let yield_item = match derive_micro_pristine(rt, pos.0, pos.1).1 {
            Some(NaturalProp::WildHerb) => RimItemKind::MountainHerb,
            _ => RimItemKind::BambooShoot,
        };
        let val = quote_item(world, s, yield_item).bid_mg as f32 * 1.5;
        let d = ((pos.0 - start.0).abs() + (pos.1 - start.1).abs()) as f32;
        consider((val / (5.0 + d)) * farm_apt, Choice::Forage(pos));
    }

    let Some((_, choice)) = best else {
        return false;
    };
    match choice {
        Choice::Hunt(b_id, b_pos) => {
            if let Some(p) = find_micro_path(rt, &world.overlays, start, b_pos, true, false) {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::ChaseTarget { target_id: b_id };
                world.actors[idx].activity_tag = "按價出獵".into();
                return true;
            }
        }
        Choice::Farm(pos, op) => {
            if let Some(p) = find_micro_path_crowded(rt, &world.overlays, start, pos, true, false, occ) {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::FarmPlot {
                    tx: pos.0,
                    ty: pos.1,
                    op,
                };
                enqueue_contiguous_parcel(world, idx, pos.0, pos.1, op);
                world.actors[idx].activity_tag = match op {
                    FarmOp::Harvest => "整畦採收",
                    _ => "整畦播栽",
                }
                .into();
                return true;
            }
        }
        Choice::Cook(fpos, raw) => {
            if let Some(p) = find_micro_path_crowded(rt, &world.overlays, start, fpos, true, false, occ) {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::CookAt {
                    tx: fpos.0,
                    ty: fpos.1,
                    raw,
                };
                world.actors[idx].activity_tag = "赴火烹製".into();
                return true;
            }
        }
        Choice::Craft(wpos, item) => {
            if let Some(p) = find_micro_path_crowded(rt, &world.overlays, start, wpos, true, false, occ) {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::CraftAt {
                    tx: wpos.0,
                    ty: wpos.1,
                    item,
                };
                world.actors[idx].activity_tag = format!("造製{}", item.label());
                return true;
            }
        }
        Choice::Forage(pos) => {
            if let Some(p) = find_micro_path_crowded(rt, &world.overlays, start, pos, true, false, occ) {
                world.actors[idx].path = p.into();
                world.actors[idx].task = PendingTask::ChopAt { tx: pos.0, ty: pos.1 };
                world.actors[idx].activity_tag = "林緣採收".into();
                return true;
            }
        }
    }
    false
}

/// 跨聚落真實價差驅動之行商貨運（掃描全 14 物資選出淨利最高者購貨挑赴對岸）。
pub fn prepare_caravan_arbitrage_cargo(
    world: &mut RimWorldState,
    idx: usize,
    from: SettlementId,
) {
    let to = from.other();
    let carried: Vec<(RimItemKind, u16)> = world.actors[idx]
        .inventory
        .iter()
        .filter_map(|s| (s.count > 0 && !s.item.is_hand_tool()).then_some((s.item, s.count)))
        .collect();
    for (item, cnt) in carried {
        for _ in 0..cnt {
            if sell_to_storehouse(world, idx, item, None).is_err() {
                break;
            }
        }
    }

    let mut ranked: Vec<(i32, RimItemKind, u32)> = ALL_ITEMS
        .iter()
        .filter_map(|&item| {
            let has_stock = world.overlays.iter().any(|(&(mx, my), ov)| {
                settlement_of_pos(mx, my) == from
                    && ov.structure == Some(StructureKind::Stockpile)
                    && ov.ground_item.is_some_and(|g| g.item == item && g.count >= 2)
            });
            if !has_stock {
                return None;
            }
            let q_from = quote_item(world, from, item);
            let q_to = quote_item(world, to, item);
            let spread = (q_to.bid_mg as i32) - (q_from.ask_mg as i32);
            (spread > 0).then_some((spread, item, q_from.ask_mg))
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));

    let mut loaded = 0_usize;
    for (_, item, _) in ranked {
        while loaded < 4 && buy_from_storehouse(world, idx, item, None).is_ok() {
            loaded += 1;
        }
        if loaded >= 4 {
            break;
        }
    }
}

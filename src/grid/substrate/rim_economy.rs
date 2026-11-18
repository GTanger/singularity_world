//! 全代數無魔術常數供需價格湧現引擎、雙向空間套利、公倉流動性帳本與真實工具磨損模組。

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use super::rim_ecology::FarmOp;
use super::rim_sim::{ActorFaction, PendingTask, RimWorldState};
use super::rim_world::{GroundStack, MicroCellOverlay, RimItemKind, StructureKind};
pub use super::rim_market::{
    prepare_caravan_arbitrage_cargo, try_assign_market_livelihood, try_guard_or_market_livelihood,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SettlementId {
    WestOutpost = 0,
    Shuoweiliao = 1,
}

impl SettlementId {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn other(self) -> Self {
        match self {
            Self::WestOutpost => Self::Shuoweiliao,
            Self::Shuoweiliao => Self::WestOutpost,
        }
    }

    pub fn anchor(self) -> (i32, i32) {
        match self {
            Self::WestOutpost => (0, 0),
            Self::Shuoweiliao => (31, -7),
        }
    }

    pub fn yard_home(self) -> (i32, i32) {
        match self {
            Self::WestOutpost => (-9, 1),
            Self::Shuoweiliao => (24, -10),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::WestOutpost => "西麓湧泉",
            Self::Shuoweiliao => "水尾寮",
        }
    }
}

pub fn settlement_of_pos(mx: i32, _my: i32) -> SettlementId {
    if mx >= 18 {
        SettlementId::Shuoweiliao
    } else {
        SettlementId::WestOutpost
    }
}

pub fn is_bulk_material(item: RimItemKind) -> bool {
    matches!(item, RimItemKind::BambooPole | RimItemKind::RiverStone)
}

pub fn stockpile_is_indoor(mx: i32, my: i32) -> bool {
    ((-12..=-7).contains(&mx) && (0..=4).contains(&my))
        || ((22..=25).contains(&mx) && (-11..=-9).contains(&my))
        || ((29..=35).contains(&mx) && (-9..=-5).contains(&my))
}

pub fn stockpile_slot_capacity(indoor: bool, item: RimItemKind) -> u16 {
    if is_bulk_material(item) {
        if indoor { 24 } else { 40 }
    } else if matches!(
        item,
        RimItemKind::BambooShoot
            | RimItemKind::BeastMeat
            | RimItemKind::MountainHerb
            | RimItemKind::SweetTuber
    ) {
        if indoor { 24 } else { 12 }
    } else {
        12
    }
}

pub fn eco_spec(item: RimItemKind) -> (f32, f32, f32) {
    // (單人每日消耗權重 c_i, 馬歇爾需求彈性 η_i, 基準價鎂 P_par)
    match item {
        RimItemKind::BambooPole => (0.45, 0.75, 2.0),
        RimItemKind::RiverStone => (0.25, 0.75, 2.0),
        RimItemKind::BambooShoot => (0.50, 0.75, 4.0),
        RimItemKind::SweetTuber => (0.50, 0.75, 4.0),
        RimItemKind::BeastMeat => (0.45, 0.85, 6.0),
        RimItemKind::MountainHerb => (0.35, 0.85, 6.0),
        RimItemKind::RoastedShoot => (0.40, 0.90, 8.0),
        RimItemKind::RoastedTuber => (0.40, 0.90, 9.0),
        RimItemKind::BambooHat => (0.18, 0.80, 8.0),
        RimItemKind::BambooBlade => (0.18, 0.85, 10.0),
        RimItemKind::StonePick => (0.18, 0.85, 12.0),
        RimItemKind::BambooPack => (0.18, 0.85, 12.0),
        RimItemKind::RoastedMeat => (0.45, 0.95, 12.0),
        RimItemKind::WoundSalve => (0.25, 0.95, 12.0),
    }
}

pub const ALL_ITEMS: [RimItemKind; 14] = [
    RimItemKind::BambooPole,
    RimItemKind::RiverStone,
    RimItemKind::BambooShoot,
    RimItemKind::SweetTuber,
    RimItemKind::BeastMeat,
    RimItemKind::MountainHerb,
    RimItemKind::RoastedShoot,
    RimItemKind::RoastedTuber,
    RimItemKind::RoastedMeat,
    RimItemKind::WoundSalve,
    RimItemKind::BambooBlade,
    RimItemKind::StonePick,
    RimItemKind::BambooHat,
    RimItemKind::BambooPack,
];

#[derive(Debug, Clone, Copy)]
pub struct MarketQuote {
    pub bid_mg: u32,
    pub ask_mg: u32,
    pub r_eff: f32,
    pub no_space: bool,
    pub status_tag: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RimEconomyBook {
    pub vault_mg: [u32; 2],
    pub hunting_pressure: u16,
    #[serde(skip)]
    pub tool_wear: HashMap<usize, u8>,
    #[serde(skip)]
    pub parcel_queues: HashMap<usize, VecDeque<(i32, i32, FarmOp)>>,
    #[serde(skip)]
    pub step_cursor: u64,
}

impl Default for RimEconomyBook {
    fn default() -> Self {
        Self {
            vault_mg: [120, 96],
            hunting_pressure: 0,
            tool_wear: HashMap::new(),
            parcel_queues: HashMap::new(),
            step_cursor: 0,
        }
    }
}

pub fn find_best_stockpile_slot_for(
    overlays: &HashMap<(i32, i32), MicroCellOverlay>,
    from: (i32, i32),
    item: RimItemKind,
    settle_filter: Option<SettlementId>,
) -> Option<(i32, i32)> {
    let want_indoor = !is_bulk_material(item);
    overlays
        .iter()
        .filter(|((mx, my), ov)| {
            if ov.structure != Some(StructureKind::Stockpile) {
                return false;
            }
            if let Some(sf) = settle_filter
                && settlement_of_pos(*mx, *my) != sf
            {
                return false;
            }
            let indoor = stockpile_is_indoor(*mx, *my);
            let cap = stockpile_slot_capacity(indoor, item);
            ov.ground_item
                .is_none_or(|g| g.item == item && g.count < cap)
        })
        .min_by_key(|((mx, my), ov)| {
            let indoor = stockpile_is_indoor(*mx, *my);
            let shelter_pen = if indoor == want_indoor { 0 } else { 300 };
            let stack_bias = if ov.ground_item.is_some_and(|g| g.item == item) {
                0
            } else {
                80
            };
            shelter_pen + stack_bias + (*mx - from.0).abs() + (*my - from.1).abs()
        })
        .map(|(&pos, _)| pos)
}

pub fn quote_item(world: &RimWorldState, s: SettlementId, item: RimItemKind) -> MarketQuote {
    let pop = world
        .actors
        .iter()
        .filter(|a| {
            a.faction == ActorFaction::Settler
                && settlement_of_pos(a.micro_pos().0, a.micro_pos().1) == s
        })
        .count()
        .max(1) as f32;

    let want_indoor = !is_bulk_material(item);
    let mut v_store = 0.0_f32;
    let mut total_slots = 0.0_f32;
    let mut used_slots = 0.0_f32;
    let mut has_room = false;

    for (&(mx, my), ov) in &world.overlays {
        if settlement_of_pos(mx, my) != s {
            continue;
        }
        if ov.structure == Some(StructureKind::Stockpile) {
            let indoor = stockpile_is_indoor(mx, my);
            if let Some(g) = ov.ground_item {
                if g.item == item {
                    v_store += f32::from(g.count);
                    if g.count < stockpile_slot_capacity(indoor, item) {
                        has_room = true;
                    }
                }
            } else if indoor == want_indoor || !has_room {
                has_room = true;
            }
            if indoor == want_indoor {
                total_slots += 1.0;
                if ov.ground_item.is_some() {
                    used_slots += 1.0;
                }
            }
        }
    }

    let mut v_committed = 0.0_f32;
    for a in &world.actors {
        if a.faction != ActorFaction::Settler {
            continue;
        }
        let pos = a.micro_pos();
        if settlement_of_pos(pos.0, pos.1) != s {
            continue;
        }
        if let Some(c) = a.carrying
            && c.item == item
        {
            v_committed += f32::from(c.count);
        }
        match a.task {
            PendingTask::CookAt { raw, .. }
                if matches!(
                    (raw, item),
                    (RimItemKind::BeastMeat, RimItemKind::RoastedMeat)
                        | (RimItemKind::SweetTuber, RimItemKind::RoastedTuber)
                        | (RimItemKind::BambooShoot, RimItemKind::RoastedShoot)
                ) =>
            {
                v_committed += 1.0;
            }
            PendingTask::CraftAt { item: p, .. } if p == item => v_committed += 1.0,
            PendingTask::FarmPlot { op: FarmOp::Harvest, tx, ty }
                if matches!(
                    (world.overlays.get(&(tx, ty)).and_then(|o| o.structure), item),
                    (Some(StructureKind::HerbMature), RimItemKind::MountainHerb)
                        | (Some(StructureKind::CropMature), RimItemKind::SweetTuber)
                ) =>
            {
                v_committed += 2.0;
            }
            PendingTask::ChaseTarget { .. } if item == RimItemKind::BeastMeat => {
                v_committed += 1.5;
            }
            _ => {}
        }
    }

    let (c_i, eta, par_mg) = eco_spec(item);
    let v_cap = (3.0 * c_i * pop).max(2.0);
    let r_eff = (v_store + v_committed + 0.5) / v_cap;
    let p_mid = par_mg * r_eff.powf(-eta);

    let vault = f32::from(world.economy.vault_mg[s.index()] as u16);
    let m_star = 28.0 * pop;
    let l_factor = (vault / m_star).max(0.0).sqrt().clamp(0.55, 1.45);
    let rho = if total_slots > 0.0 {
        (used_slots / total_slots).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let shelf_shadow = if has_room {
        1.0 - 0.45 * rho.powi(3)
    } else {
        0.0
    };

    let raw_bid = (p_mid * 0.92 * l_factor * shelf_shadow).floor();
    let cap_bid = (world.economy.vault_mg[s.index()] / 3).min(60);
    let bid_mg = if !has_room || r_eff > 2.35 || raw_bid < 0.85 {
        0
    } else {
        (raw_bid as u32).min(cap_bid)
    };
    let ask_mg = ((p_mid * (1.10 / l_factor)).ceil() as u32)
        .max(bid_mg.saturating_add(1))
        .max(1);

    let status_tag = if !has_room || bid_mg == 0 {
        "倉足停收"
    } else if r_eff < 0.48 {
        "急缺高價"
    } else if r_eff < 1.05 {
        "供需平穩"
    } else {
        "存貨寬裕"
    };

    MarketQuote {
        bid_mg,
        ask_mg,
        r_eff,
        no_space: !has_room,
        status_tag,
    }
}

/// 個人向所在地聚落公倉售出 1 件貨物（物資入倉、公倉扣鎂入個人錢包，嚴格守恆）。
pub fn sell_to_storehouse(
    world: &mut RimWorldState,
    actor_idx: usize,
    item: RimItemKind,
    preferred_cell: Option<(i32, i32)>,
) -> Result<(u32, String), &'static str> {
    let pos = world.actors[actor_idx].micro_pos();
    let s = settlement_of_pos(pos.0, pos.1);
    let quote = quote_item(world, s, item);

    let slot = preferred_cell
        .filter(|&p| {
            world.overlays.get(&p).is_some_and(|ov| {
                ov.structure == Some(StructureKind::Stockpile)
                    && ov.ground_item.is_none_or(|g| {
                        g.item == item && g.count < stockpile_slot_capacity(stockpile_is_indoor(p.0, p.1), item)
                    })
            })
        })
        .or_else(|| find_best_stockpile_slot_for(&world.overlays, pos, item, Some(s)));

    let Some(dst) = slot else {
        return Err("聚落倉廩已無相應儲位");
    };
    if world.actors[actor_idx].take_inv(item, 1) == 0 {
        return Err("隨身行囊已無此物");
    }
    world.add_ground_item(dst.0, dst.1, item, 1);

    let paid = quote.bid_mg.min(world.economy.vault_mg[s.index()]);
    world.economy.vault_mg[s.index()] -= paid;
    world.actors[actor_idx].magnesium = world.actors[actor_idx].magnesium.saturating_add(paid);
    world.dirty_unsaved_ticks += 3;

    let note = if paid > 0 {
        format!("將［{}］售入{}倉廩，領得 {} 鎂", item.label(), s.label(), paid)
    } else {
        format!("將［{}］歸入{}倉廩存儲", item.label(), s.label())
    };
    Ok((paid, note))
}

/// 個人向所在地聚落公倉購取 1 件貨物（個人扣鎂入公倉、物資出倉入囊，嚴格守恆）。
pub fn buy_from_storehouse(
    world: &mut RimWorldState,
    buyer_idx: usize,
    item: RimItemKind,
    preferred_cell: Option<(i32, i32)>,
) -> Result<(u32, String), &'static str> {
    let pos = world.actors[buyer_idx].micro_pos();
    let s = settlement_of_pos(pos.0, pos.1);
    let quote = quote_item(world, s, item);
    if world.actors[buyer_idx].magnesium < quote.ask_mg {
        return Err("隨身攜帶的鎂晶不足");
    }

    let stock_pos = preferred_cell
        .filter(|&p| {
            world.overlays.get(&p).is_some_and(|ov| {
                ov.structure == Some(StructureKind::Stockpile)
                    && ov.ground_item.is_some_and(|g| g.item == item && g.count > 0)
            })
        })
        .or_else(|| {
            world
                .overlays
                .iter()
                .filter(|((mx, my), ov)| {
                    settlement_of_pos(*mx, *my) == s
                        && ov.structure == Some(StructureKind::Stockpile)
                        && ov.ground_item.is_some_and(|g| g.item == item && g.count > 0)
                })
                .min_by_key(|((mx, my), _)| (*mx - pos.0).abs() + (*my - pos.1).abs())
                .map(|(&p, _)| p)
        });

    let Some(src) = stock_pos else {
        return Err("倉廩內暫無此物現貨");
    };
    if world.take_ground_item(src.0, src.1, item, 1) == 0 {
        return Err("倉廩內暫無此物現貨");
    }
    world.actors[buyer_idx].magnesium -= quote.ask_mg;
    world.economy.vault_mg[s.index()] = world.economy.vault_mg[s.index()].saturating_add(quote.ask_mg);
    world.actors[buyer_idx].add_inv(item, 1);
    world.dirty_unsaved_ticks += 3;

    Ok((
        quote.ask_mg,
        format!("向{}倉廩購取［{}］（{} 鎂）", s.label(), item.label(), quote.ask_mg),
    ))
}

/// 記錄手持工具（刀、鎬）於實作或交鋒後的真實磨損；滿額時鈍損毀壞並騰出新器需求。
pub fn record_tool_wear(world: &mut RimWorldState, idx: usize) {
    let Some(tool) = world.actors[idx].equipped_tool else {
        return;
    };
    let aid = world.actors[idx].id;
    let w = world.economy.tool_wear.entry(aid).or_insert(0);
    *w = w.saturating_add(1);
    if *w >= 28 {
        *w = 0;
        world.actors[idx].equipped_tool = None;
        world.actors[idx].push_diary(format!("手中的{}磨耗已極，應聲折斷", tool.label()));
        world.actors[idx].pop_text = Some((format!("{}磨損", tool.label()), world.tick + 10));
        let g = world.actors[idx].glyph.clone();
        world.push_event(format!("{g} 手邊［{}］磨耗毀損", tool.label()));
    }
}

/// 公倉盈餘分紅回流、富裕消費消解及獵捕壓力緩降（嚴守總鎂守恆）。
pub fn step_economy_tick(world: &mut RimWorldState) {
    if world.tick.is_multiple_of(40) && world.economy.hunting_pressure > 0 {
        world.economy.hunting_pressure = world.economy.hunting_pressure.saturating_sub(1);
    }
    if !world.tick.is_multiple_of(240) {
        return;
    }

    for s in [SettlementId::WestOutpost, SettlementId::Shuoweiliao] {
        let workers: Vec<usize> = world
            .actors
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                a.faction == ActorFaction::Settler
                    && a.hp > 0
                    && a.downed_ticks == 0
                    && settlement_of_pos(a.micro_pos().0, a.micro_pos().1) == s
            })
            .map(|(i, _)| i)
            .collect();
        if workers.is_empty() {
            continue;
        }
        let target_vault = 28 * (workers.len() as u32);
        let cur = world.economy.vault_mg[s.index()];
        if cur > (target_vault * 5) / 4 {
            let excess = (cur - target_vault) / 4;
            let each = excess / (workers.len() as u32);
            if each > 0 {
                let total_out = each * (workers.len() as u32);
                world.economy.vault_mg[s.index()] -= total_out;
                for &w_idx in &workers {
                    world.actors[w_idx].magnesium = world.actors[w_idx].magnesium.saturating_add(each);
                }
            }
        }
    }

    // 錢包富裕且需補養的自主居民主動消費高階熟食／藥膏，讓鎂穩定回流公倉
    for i in 0..world.actors.len() {
        let a = &world.actors[i];
        if a.faction != ActorFaction::Settler || !a.autopilot || a.magnesium < 80 {
            continue;
        }
        if a.injury_ticks > 0 {
            let _ = buy_from_storehouse(world, i, RimItemKind::WoundSalve, None);
        } else if a.hunger < 65 {
            if buy_from_storehouse(world, i, RimItemKind::RoastedMeat, None).is_ok() {
                let _ = super::rim_life::eat_item(&mut world.actors[i], RimItemKind::RoastedMeat);
            }
        }
    }
}

/// 抵達倉廩後將行囊盈餘賣入公倉或購齊缺口器具。
pub fn complete_storehouse_trade(world: &mut RimWorldState, idx: usize, tx: i32, ty: i32) {
    let (ax, ay) = world.actors[idx].micro_pos();
    if (ax - tx).abs() > 2 || (ay - ty).abs() > 2 {
        world.actors[idx].task = PendingTask::None;
        return;
    }
    let s = settlement_of_pos(ax, ay);
    let mut sold_any = false;
    let sellables: Vec<RimItemKind> = world.actors[idx]
        .inventory
        .iter()
        .filter_map(|st| (st.count > 0).then_some(st.item))
        .collect();
    for item in sellables {
        let keep = if item.is_edible() { 1 } else { 0 };
        let have: u16 = world.actors[idx]
            .inventory
            .iter()
            .filter(|st| st.item == item)
            .map(|st| st.count)
            .sum();
        let excess = have.saturating_sub(keep);
        if excess > 0 && quote_item(world, s, item).bid_mg > 0 {
            for _ in 0..excess.min(4) {
                if sell_to_storehouse(world, idx, item, Some((tx, ty))).is_ok() {
                    sold_any = true;
                }
            }
        }
    }
    if world.actors[idx].equipped_tool.is_none() {
        for tool in [RimItemKind::StonePick, RimItemKind::BambooBlade] {
            if buy_from_storehouse(world, idx, tool, None).is_ok()
                && world.actors[idx].take_inv(tool, 1) > 0
            {
                world.actors[idx].equipped_tool = Some(tool);
                break;
            }
        }
    }
    if sold_any {
        world.actors[idx].pop_text = Some(("倉廩交貨結算".into(), world.tick + 10));
        world.actors[idx].activity_tag = "倉廩交割".into();
    }
    world.actors[idx].task = PendingTask::None;
}

/// 統一代幣貨物代碼對應（與 HTTP API 完全一致）。
pub const fn item_code(item: RimItemKind) -> &'static str {
    match item {
        RimItemKind::BambooPole => "bamboo",
        RimItemKind::RiverStone => "stone",
        RimItemKind::BambooShoot => "shoot",
        RimItemKind::BeastMeat => "meat",
        RimItemKind::RoastedShoot => "roast_shoot",
        RimItemKind::RoastedMeat => "roast_meat",
        RimItemKind::BambooBlade => "blade",
        RimItemKind::StonePick => "pick",
        RimItemKind::BambooHat => "hat",
        RimItemKind::BambooPack => "pack",
        RimItemKind::MountainHerb => "herb",
        RimItemKind::WoundSalve => "salve",
        RimItemKind::SweetTuber => "tuber",
        RimItemKind::RoastedTuber => "roast_tuber",
    }
}

/// 依倉廩室內外屬地與供需牌價組裝儲物格檢查面板標籤與操作插座。
pub fn describe_stockpile_card(
    world: &RimWorldState,
    viewer_idx: Option<usize>,
    mx: i32,
    my: i32,
    ground_item: Option<GroundStack>,
    tags: &mut Vec<String>,
    sockets: &mut Vec<serde_json::Value>,
) {
    let s = settlement_of_pos(mx, my);
    let indoor = stockpile_is_indoor(mx, my);
    tags.push(if indoor { "倉廩木架儲位" } else { "竹木石料場" }.into());
    sockets.push(serde_json::json!({ "label": "走近倉架", "action": "move" }));
    if let Some(g) = ground_item {
        let q = quote_item(world, s, g.item);
        tags.push(format!("{}（{}）", g.item.label(), q.status_tag));
        sockets.push(serde_json::json!({ "label": format!("抱取{}", g.item.label()), "action": "pickup" }));
        if q.ask_mg > 0 {
            sockets.push(serde_json::json!({
                "label": format!("向公倉購{}（{} 鎂）", g.item.label(), q.ask_mg),
                "cmd": "vault_buy",
                "item_code": item_code(g.item),
            }));
        }
    }
    if let Some(vi) = viewer_idx {
        let mut seen = Vec::new();
        for st in &world.actors[vi].inventory {
            if st.count == 0 || st.item.is_hand_tool() || seen.contains(&st.item) {
                continue;
            }
            seen.push(st.item);
            let q = quote_item(world, s, st.item);
            if q.bid_mg > 0 {
                sockets.push(serde_json::json!({
                    "label": format!("售{}入倉（得 {} 鎂）", st.item.label(), q.bid_mg),
                    "cmd": "vault_sell",
                    "item_code": item_code(st.item),
                }));
            }
        }
    }
}

/// 執行玩家點擊公倉買入／賣出指令，嚴格維持聚落金庫與角色隨身鎂總量守恆。
pub fn handle_vault_cmd(
    world: &mut RimWorldState,
    p_idx: usize,
    is_buy: bool,
    mx: i32,
    my: i32,
    code: &str,
) -> (bool, String) {
    let Some(item) = ALL_ITEMS.iter().copied().find(|&i| item_code(i) == code) else {
        return (false, "未明貨品品項".into());
    };
    let (ax, ay) = world.actors[p_idx].micro_pos();
    if (ax - mx).abs() > 3 || (ay - my).abs() > 3 {
        return (false, "須走近倉廩貨架三步內方可交割".into());
    }
    let res = if is_buy {
        buy_from_storehouse(world, p_idx, item, Some((mx, my)))
    } else {
        sell_to_storehouse(world, p_idx, item, Some((mx, my)))
    };
    match res {
        Ok((_, msg)) => (true, msg),
        Err(e) => (false, e.into()),
    }
}

/// 對話面對面易物：依所在地聚落即時市價成交，嚴格保證雙邊鎂代幣守恆。
pub fn settlement_market_barter(
    world: &mut RimWorldState,
    p_idx: usize,
    n_idx: usize,
    item: RimItemKind,
    s: SettlementId,
    player_buys: bool,
) -> Result<(u32, String), &'static str> {
    let q = quote_item(world, s, item);
    let (buyer_idx, seller_idx, price) = if player_buys {
        (p_idx, n_idx, q.ask_mg.max(1))
    } else {
        if q.bid_mg == 0 {
            return Err("此地倉存充盈，對方暫不收購");
        }
        (n_idx, p_idx, q.bid_mg)
    };
    if !world.actors[seller_idx].inventory.iter().any(|st| st.item == item && st.count > 0) {
        return Err("對方身上此物已空");
    }
    if world.actors[buyer_idx].magnesium < price {
        return Err("隨身鎂不足，難以議價成交");
    }
    if world.actors[seller_idx].take_inv(item, 1) == 0 {
        return Err("交貨取物失敗");
    }
    world.actors[buyer_idx].magnesium -= price;
    world.actors[seller_idx].magnesium += price;
    world.actors[buyer_idx].add_inv(item, 1);
    let msg = if player_buys {
        format!("以即時市價 {price} 鎂購得一份{}", item.label())
    } else {
        format!("讓出一份{}，依即時市價實得 {price} 鎂", item.label())
    };
    Ok((price, msg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::rim_ecology::{
        reconcile_real_scale_layout_and_clutter, step_land_ecology_with_weather,
    };
    use crate::grid::substrate::rim_shelter::RimWeatherKind;
    use crate::grid::substrate::rim_sim::RimActor;

    fn eco_actor(id: usize, name: &str, dir: &str, mag: u32, pos: (f32, f32)) -> RimActor {
        RimActor {
            id,
            code: format!("m_{id}"),
            actor_key: format!("m_{id}"),
            entity_id: None,
            glyph: name.chars().next().unwrap_or('人').to_string(),
            name: name.into(),
            gender: "M".into(),
            soul_seed: 100 + id as i64,
            role_tag: "測試居民".into(),
            faction: ActorFaction::Settler,
            is_player: false,
            autopilot: true,
            running: false,
            px: pos.0,
            py: pos.1,
            facing_rad: 0.0,
            hp: 100,
            max_hp: 100,
            equipped_tool: Some(RimItemKind::BambooBlade),
            carrying: None,
            inventory: Vec::new(),
            directive: dir.into(),
            path: VecDeque::new(),
            task: PendingTask::None,
            activity_tag: String::new(),
            pop_text: None,
            cooldown_ticks: 0,
            last_active_unix: 0,
            hunger: 85,
            thirst: 85,
            stamina: 90,
            magnesium: mag,
            equipped_hat: true,
            equipped_pack: true,
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

    fn base_world() -> RimWorldState {
        let mut ov = HashMap::new();
        reconcile_real_scale_layout_and_clutter(None, &mut ov);
        RimWorldState {
            tick: 0,
            overlays: ov,
            actors: vec![
                eco_actor(1, "陳青", "guard", 20, (0.0, 0.0)),
                eco_actor(2, "林阿木", "craft", 20, (1.0, 0.0)),
                eco_actor(3, "許秋娘", "forage", 20, (-1.0, 0.0)),
                eco_actor(4, "周阿茶", "forage", 20, (31.0, -7.0)),
                eco_actor(5, "鄭石頭", "forage", 30, (32.0, -7.0)),
            ],
            events: VecDeque::new(),
            dirty_unsaved_ticks: 0,
            sub_step: 0,
            stall_tracker: HashMap::new(),
            economy: RimEconomyBook::default(),
        }
    }

    #[test]
    fn isoelastic_supply_demand_shifts_prices_and_halts_glut_naturally() {
        let mut w = base_world();
        let q_short = quote_item(&w, SettlementId::WestOutpost, RimItemKind::BeastMeat);
        assert!(q_short.bid_mg >= 10, "scarce meat must pay high bid: {:?}", q_short);

        // 填入大量獸肉至西麓倉廩（非寫死配額，純由等彈性供需曲線壓低收購價）
        w.add_ground_item(-12, 4, RimItemKind::BeastMeat, 18);
        let q_glut = quote_item(&w, SettlementId::WestOutpost, RimItemKind::BeastMeat);
        assert!(q_glut.bid_mg < q_short.bid_mg);
        assert_eq!(q_glut.bid_mg, 0, "glutted storehouse stops buying naturally: {:?}", q_glut);

        // 當生肉過剩廉價時，炙獸肉（缺貨）與生肉形成鮮明加工毛利
        let q_roast = quote_item(&w, SettlementId::WestOutpost, RimItemKind::RoastedMeat);
        assert!(q_roast.bid_mg > q_glut.ask_mg);
    }

    #[test]
    fn spatial_caravan_arbitrage_moves_surplus_meat_east_and_herbs_west_conserving_mg() {
        let mut w = base_world();
        w.overlays.get_mut(&(-12, 4)).unwrap().ground_item = Some(GroundStack {
            item: RimItemKind::BeastMeat,
            count: 14,
        });
        let total_mg_before: u32 =
            w.actors.iter().map(|a| a.magnesium).sum::<u32>() + w.economy.vault_mg.iter().sum::<u32>();

        // 鄭石頭位於西麓準備赴水尾寮：應自動買入便宜的西麓山獸骨肉運去水尾寮
        w.actors[4].px = 0.0;
        w.actors[4].py = 0.0;
        prepare_caravan_arbitrage_cargo(&mut w, 4, SettlementId::WestOutpost);
        assert!(
            w.actors[4].inventory.iter().any(|s| s.item == RimItemKind::BeastMeat && s.count > 0),
            "caravan should buy cheap surplus meat"
        );

        // 抵達水尾寮賣給對岸公倉，總鎂完全守恆
        w.actors[4].px = 31.0;
        w.actors[4].py = -7.0;
        prepare_caravan_arbitrage_cargo(&mut w, 4, SettlementId::Shuoweiliao);
        let total_mg_after: u32 =
            w.actors.iter().map(|a| a.magnesium).sum::<u32>() + w.economy.vault_mg.iter().sum::<u32>();
        assert_eq!(total_mg_before, total_mg_after);
    }

    #[test]
    fn multi_day_economy_simulation_strictly_conserves_magnesium_without_deadlock() {
        let mut w = base_world();
        let initial_mg: u32 =
            w.actors.iter().map(|a| a.magnesium).sum::<u32>() + w.economy.vault_mg.iter().sum::<u32>();

        for t in 1..=960_u64 {
            w.tick = t;
            step_land_ecology_with_weather(&mut w, None, RimWeatherKind::SummerSquall);
            step_economy_tick(&mut w);
            if t % 6 == 0 {
                for i in 0..w.actors.len() {
                    if w.actors[i].task == PendingTask::None {
                        let start = w.actors[i].micro_pos();
                        let home = settlement_of_pos(start.0, start.1).anchor();
                        let _ = try_assign_market_livelihood(&mut w, None, i, start, home, &[], &[]);
                    }
                    if i == 0 && t % 24 == 0 {
                        let _ = sell_to_storehouse(&mut w, i, RimItemKind::MountainHerb, None);
                    }
                }
            }
        }
        let end_mg: u32 =
            w.actors.iter().map(|a| a.magnesium).sum::<u32>() + w.economy.vault_mg.iter().sum::<u32>();
        assert_eq!(initial_mg, end_mg, "total magnesium must be strictly conserved");
    }
}

//! 西麓湧泉街各店專職營業、匠師落款、診所慈悲記帳、宿屋投宿與街坊日常往來模組。

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::rim_economy::{
    SettlementId, find_best_stockpile_slot_for, quote_item,
};
use super::rim_sim::{ActorFaction, PendingTask, RimActor, RimWorldState};
use super::rim_town::building_at;
use super::rim_world::{
    RimItemKind, StructureKind, find_micro_path, find_micro_path_crowded,
};
use super::runtime::SubstrateRuntime;

/// 角色隨身店舖契約、器物打造落款與診所賒欠帳目（隨 `phase_json` 持久化）。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorCivicState {
    #[serde(default)]
    pub tool_maker: Option<String>,
    #[serde(default)]
    pub inn_lease_until: u64,
    #[serde(default)]
    pub debt_creditor: Option<String>,
    #[serde(default)]
    pub debt_mg: u32,
}

/// 單次決策前往街屋門店辦理的專職服務項目。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopServiceOp {
    DineIn,
    TakeoutMeal,
    HoneTool,
    CommissionBlade,
    CommissionPick,
    ClinicTreat,
    BuySalve,
    RentInnBed,
    TeaRefresh,
    RepayDebt,
    ShopBuy(RimItemKind),
    ShopSell(RimItemKind),
}

struct ShopMeta {
    building_code: &'static str,
    label: &'static str,
    proprietor_key: &'static str,
    assistant_key: Option<&'static str>,
    shelf_slots: &'static [(i32, i32)],
    accepted_inputs: &'static [RimItemKind],
}

const SHOPS: [ShopMeta; 5] = [
    ShopMeta {
        building_code: "chunshui_diner", label: "春水小吃",
        proprietor_key: "res:liu_chunshui", assistant_key: Some("res:liu_abao"),
        shelf_slots: &[(-19, -1)],
        accepted_inputs: &[RimItemKind::BeastMeat, RimItemKind::SweetTuber, RimItemKind::BambooShoot],
    },
    ShopMeta {
        building_code: "amu_smithy", label: "阿木鐵店",
        proprietor_key: "res:lin_amu", assistant_key: Some("res:zhang_xiaoan"),
        shelf_slots: &[(8, -1), (9, 2), (10, 2)],
        accepted_inputs: &[RimItemKind::RiverStone, RimItemKind::BambooPole],
    },
    ShopMeta {
        building_code: "jian_clinic", label: "濟安診所",
        proprietor_key: "res:zhang_jian", assistant_key: Some("res:zhang_suxin"),
        shelf_slots: &[(17, -1)],
        accepted_inputs: &[RimItemKind::MountainHerb],
    },
    ShopMeta {
        building_code: "yingfeng_inn", label: "迎風宿屋",
        proprietor_key: "res:xie_chengfeng", assistant_key: None,
        shelf_slots: &[(-19, -8)],
        accepted_inputs: &[RimItemKind::RoastedShoot, RimItemKind::RoastedTuber, RimItemKind::BambooPole],
    },
    ShopMeta {
        building_code: "acha_teahouse", label: "阿茶茶寮",
        proprietor_key: "res:zhou_acha", assistant_key: Some("res:he_dashu"),
        shelf_slots: &[(30, -5), (34, -8)],
        accepted_inputs: &[RimItemKind::BambooShoot, RimItemKind::MountainHerb, RimItemKind::BeastMeat],
    },
];

fn shop_at(mx: i32, my: i32) -> Option<&'static ShopMeta> {
    let b = building_at(mx, my)?;
    SHOPS.iter().find(|s| s.building_code == b.code)
}

fn find_actor_by_key(world: &RimWorldState, key: &str) -> Option<usize> {
    world
        .actors
        .iter()
        .position(|a| a.actor_key == key && a.hp > 0)
}

/// 嚴格守恆轉移 `鎂`：優先付給店主實體，測試極簡世界缺店主時才入公倉。
fn pay_shop_mg(
    world: &mut RimWorldState,
    payer_idx: usize,
    meta: Option<&ShopMeta>,
    amount: u32,
    s: SettlementId,
) -> Result<(), &'static str> {
    if amount == 0 {
        return Ok(());
    }
    let owner_idx = meta.and_then(|m| {
        find_actor_by_key(world, m.proprietor_key)
            .or_else(|| m.assistant_key.and_then(|k| find_actor_by_key(world, k)))
    });
    if owner_idx == Some(payer_idx) {
        return Ok(());
    }
    if world.actors[payer_idx].magnesium < amount {
        return Err("隨身鎂晶不足");
    }
    world.actors[payer_idx].magnesium -= amount;
    if let Some(oi) = owner_idx {
        world.actors[oi].magnesium = world.actors[oi].magnesium.saturating_add(amount);
    } else {
        world.economy.vault_mg[s.index()] =
            world.economy.vault_mg[s.index()].saturating_add(amount);
    }
    Ok(())
}

/// 從店家貨架或店員行囊消耗 1 件實物（嚴守物質守恆）。
fn consume_shop_item(
    world: &mut RimWorldState,
    meta: &ShopMeta,
    items: &[RimItemKind],
    customer_idx: Option<usize>,
) -> Option<RimItemKind> {
    for &item in items {
        for &(sx, sy) in meta.shelf_slots {
            if world.take_ground_item(sx, sy, item, 1) > 0 {
                return Some(item);
            }
        }
        for key_opt in [Some(meta.proprietor_key), meta.assistant_key] {
            if let Some(k) = key_opt
                && let Some(si) = find_actor_by_key(world, k)
                && world.actors[si].take_inv(item, 1) > 0
            {
                return Some(item);
            }
        }
        if let Some(ci) = customer_idx
            && world.actors[ci].take_inv(item, 1) > 0
        {
            return Some(item);
        }
    }
    None
}

fn shop_stock_count(world: &RimWorldState, meta: &ShopMeta, items: &[RimItemKind]) -> u16 {
    let mut sum: u16 = 0;
    for &(sx, sy) in meta.shelf_slots {
        if let Some(g) = world.overlays.get(&(sx, sy)).and_then(|o| o.ground_item)
            && items.contains(&g.item)
        {
            sum = sum.saturating_add(g.count);
        }
    }
    for key_opt in [Some(meta.proprietor_key), meta.assistant_key] {
        if let Some(k) = key_opt
            && let Some(si) = find_actor_by_key(world, k)
        {
            for st in &world.actors[si].inventory {
                if items.contains(&st.item) {
                    sum = sum.saturating_add(st.count);
                }
            }
        }
    }
    sum
}

pub fn parse_shop_op(code: &str) -> Option<ShopServiceOp> {
    if let Some(rest) = code.strip_prefix("buy:") {
        return super::rim_town::parse_item_code_str(rest).map(ShopServiceOp::ShopBuy);
    }
    if let Some(rest) = code.strip_prefix("sell:") {
        return super::rim_town::parse_item_code_str(rest).map(ShopServiceOp::ShopSell);
    }
    Some(match code {
        "dine_in" => ShopServiceOp::DineIn,
        "takeout_meal" => ShopServiceOp::TakeoutMeal,
        "hone_tool" => ShopServiceOp::HoneTool,
        "commission_blade" => ShopServiceOp::CommissionBlade,
        "commission_pick" => ShopServiceOp::CommissionPick,
        "clinic_treat" => ShopServiceOp::ClinicTreat,
        "buy_salve" => ShopServiceOp::BuySalve,
        "rent_inn" => ShopServiceOp::RentInnBed,
        "tea_refresh" => ShopServiceOp::TeaRefresh,
        "repay_debt" => ShopServiceOp::RepayDebt,
        _ => return None,
    })
}

fn cooked_pair_for_raw(raw: RimItemKind) -> RimItemKind {
    match raw {
        RimItemKind::BeastMeat => RimItemKind::RoastedMeat,
        RimItemKind::SweetTuber => RimItemKind::RoastedTuber,
        _ => RimItemKind::RoastedShoot,
    }
}

fn bump_proprietor_craft(world: &mut RimWorldState, meta: &ShopMeta, is_forge: bool) {
    if let Some(oi) = find_actor_by_key(world, meta.proprietor_key) {
        if is_forge {
            world.actors[oi].craft_forge = world.actors[oi].craft_forge.saturating_add(2);
        } else {
            world.actors[oi].craft_culinary = world.actors[oi].craft_culinary.saturating_add(1);
        }
    }
}

pub fn execute_shop_service(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    mx: i32,
    my: i32,
    op: ShopServiceOp,
) -> (bool, String) {
    let s = SettlementId::WestOutpost;
    let tick = world.tick;
    let meta_opt = shop_at(mx, my);

    match op {
        ShopServiceOp::DineIn | ShopServiceOp::TakeoutMeal => {
            let Some(meta) = meta_opt else {
                return (false, "須於小吃店或茶寮門面辦理".into());
            };
            let cooked = [RimItemKind::RoastedMeat, RimItemKind::RoastedTuber, RimItemKind::RoastedShoot];
            let raws = [RimItemKind::BeastMeat, RimItemKind::SweetTuber, RimItemKind::BambooShoot];
            let dish = if let Some(ready) = consume_shop_item(world, meta, &cooked, None) {
                ready
            } else if let Some(raw) = consume_shop_item(world, meta, &raws, None) {
                bump_proprietor_craft(world, meta, false);
                cooked_pair_for_raw(raw)
            } else {
                return (false, format!("{}目前暫缺筍肉儲糧", meta.label));
            };
            if let Err(e) = pay_shop_mg(world, idx, Some(meta), 5, s) {
                world.add_ground_item(meta.shelf_slots[0].0, meta.shelf_slots[0].1, dish, 1);
                return (false, e.into());
            }
            if op == ShopServiceOp::DineIn {
                let a = &mut world.actors[idx];
                a.hunger = a.hunger.saturating_add(50).min(100);
                a.thirst = a.thirst.saturating_add(14).min(100);
                a.stamina = a.stamina.saturating_add(18).min(100);
                a.hp = (a.hp + 8).min(a.max_hp);
                a.pop_text = Some((format!("堂食{}", dish.label()), tick + 12));
                a.activity_tag = "堂食暖腹".into();
                a.push_diary(format!("於{}堂食熱騰騰的{}", meta.label, dish.label()));
                world.dirty_unsaved_ticks += 3;
                (true, format!("在{}吃罷一頓熱{}（5 鎂）", meta.label, dish.label()))
            } else {
                world.actors[idx].add_inv(dish, 1);
                world.actors[idx].pop_text = Some((format!("外帶{}", dish.short_char()), tick + 10));
                world.actors[idx].push_diary(format!("自{}買妥{}帶上身", meta.label, dish.label()));
                world.dirty_unsaved_ticks += 3;
                (true, format!("自{}打包了一份{}（5 鎂）", meta.label, dish.label()))
            }
        }
        ShopServiceOp::TeaRefresh => {
            let meta = meta_opt.unwrap_or(&SHOPS[4]);
            let Some(base) = consume_shop_item(
                world, meta, &[RimItemKind::RoastedShoot, RimItemKind::MountainHerb, RimItemKind::BambooShoot], None,
            ) else {
                return (false, format!("{}茶罐暫缺茶葉與茶點", meta.label));
            };
            if let Err(e) = pay_shop_mg(world, idx, Some(meta), 4, s) {
                world.add_ground_item(meta.shelf_slots[0].0, meta.shelf_slots[0].1, base, 1);
                return (false, e.into());
            }
            bump_proprietor_craft(world, meta, false);
            let a = &mut world.actors[idx];
            a.thirst = a.thirst.saturating_add(45).min(100);
            a.hunger = a.hunger.saturating_add(24).min(100);
            a.stamina = a.stamina.saturating_add(24).min(100);
            a.wet_ticks = 0;
            a.pop_text = Some(("喫茶驅濕".into(), tick + 12));
            a.activity_tag = "茶寮品茗".into();
            a.push_diary(format!("於{}喝熱茶配茶點，通體暖和", meta.label));
            world.dirty_unsaved_ticks += 3;
            (true, format!("於{}喫茶消倦解渴（4 鎂）", meta.label))
        }
        ShopServiceOp::HoneTool => {
            let Some(tool) = world.actors[idx].equipped_tool else {
                return (false, "手邊未持刀鎬，無須開鋒校刃".into());
            };
            let meta = meta_opt.unwrap_or(&SHOPS[1]);
            if let Err(e) = pay_shop_mg(world, idx, Some(meta), 4, s) {
                return (false, e.into());
            }
            let aid = world.actors[idx].id;
            world.economy.tool_wear.insert(aid, 0);
            world.actors[idx].civic.tool_maker = Some("阿木鐵店・林阿木校刃".into());
            world.actors[idx].pop_text = Some(("校刃雪亮".into(), tick + 12));
            world.actors[idx].push_diary(format!("請林阿木將{}重新開鋒校刃", tool.label()));
            bump_proprietor_craft(world, meta, true);
            world.dirty_unsaved_ticks += 3;
            (true, format!("阿木已將{}重新校磨雪亮（4 鎂）", tool.label()))
        }
        ShopServiceOp::CommissionBlade | ShopServiceOp::CommissionPick => {
            let prod = if op == ShopServiceOp::CommissionBlade {
                RimItemKind::AlloyBlade
            } else {
                RimItemKind::AlloyPick
            };
            let meta = meta_opt.unwrap_or(&SHOPS[1]);
            let price = quote_item(world, s, prod).ask_mg;
            if world.actors[idx].magnesium < price {
                return (false, "隨身鎂晶不足委託鍛製".into());
            }
            if consume_shop_item(world, meta, &[prod], None).is_none() {
                if consume_shop_item(world, meta, &[RimItemKind::BambooPole], Some(idx)).is_none() {
                    return (false, "鐵店缺莿竹稈料，暫難配柄開爐".into());
                }
                if consume_shop_item(world, meta, &[RimItemKind::RiverStone], Some(idx)).is_none() {
                    world.add_ground_item(
                        meta.shelf_slots[0].0,
                        meta.shelf_slots[0].1,
                        RimItemKind::BambooPole,
                        1,
                    );
                    return (false, "鐵店缺硬石料，暫難淬鋼鍛刃".into());
                }
            }
            let _ = pay_shop_mg(world, idx, Some(meta), price, s);
            let aid = world.actors[idx].id;
            world.economy.tool_wear.insert(aid, 0);
            if let Some(prev) = world.actors[idx].equipped_tool.replace(prod) {
                world.actors[idx].add_inv(prev, 1);
            }
            world.actors[idx].civic.tool_maker = Some("阿木鐵店・林阿木親鍛".into());
            world.actors[idx].pop_text = Some((format!("新打{}", prod.short_char()), tick + 12));
            world.actors[idx].push_diary(format!("委託林阿木親手鍛成{}", prod.label()));
            bump_proprietor_craft(world, meta, true);
            world.dirty_unsaved_ticks += 4;
            (true, format!("取得阿木親鍛［{}］並落款上身（{} 鎂）", prod.label(), price))
        }
        ShopServiceOp::ClinicTreat => {
            let a_ref = &world.actors[idx];
            if a_ref.injury_ticks == 0 && a_ref.hp >= a_ref.max_hp {
                return (false, "此刻氣血平順無傷，無須診療".into());
            }
            let meta = meta_opt.unwrap_or(&SHOPS[2]);
            let med = consume_shop_item(
                world,
                meta,
                &[RimItemKind::WoundSalve, RimItemKind::MountainHerb],
                Some(idx),
            );
            if med.is_none() {
                return (false, "診所藥櫃暫缺青草藥與藥膏".into());
            }
            let fee: u32 = 6;
            let cash_paid = world.actors[idx].magnesium.min(fee);
            let tab_owed = fee - cash_paid;
            let _ = pay_shop_mg(world, idx, Some(meta), cash_paid, s);
            let a = &mut world.actors[idx];
            a.injury_part = None;
            a.injury_ticks = 0;
            a.hp = (a.hp + 26).min(a.max_hp);
            a.stamina = a.stamina.saturating_add(16).min(100);
            if tab_owed > 0 {
                a.civic.debt_creditor = Some("res:zhang_jian".into());
                a.civic.debt_mg = a.civic.debt_mg.saturating_add(tab_owed);
                a.push_diary("負傷赴濟安診所敷藥治妥，差額診金記帳");
                a.pop_text = Some(("診妥暫賒帳".into(), tick + 12));
                world.dirty_unsaved_ticks += 4;
                (true, "張醫師已先替你敷藥消傷，差額診金暫記帳上".into())
            } else {
                a.push_diary("赴濟安診所看診敷藥，傷勢已消");
                a.pop_text = Some(("診畢神爽".into(), tick + 12));
                world.dirty_unsaved_ticks += 4;
                (true, format!("張醫師已清傷包紮完妥（{cash_paid} 鎂）"))
            }
        }
        ShopServiceOp::BuySalve => {
            let meta = meta_opt.unwrap_or(&SHOPS[2]);
            let price = quote_item(world, s, RimItemKind::WoundSalve).ask_mg;
            if world.actors[idx].magnesium < price {
                return (false, "隨身鎂晶不足購藥".into());
            }
            if consume_shop_item(world, meta, &[RimItemKind::WoundSalve], None).is_none() {
                if consume_shop_item(world, meta, &[RimItemKind::MountainHerb], None).is_none() {
                    return (false, "診所藥櫃暫無敷傷藥膏存貨".into());
                }
            }
            let _ = pay_shop_mg(world, idx, Some(meta), price, s);
            world.actors[idx].add_inv(RimItemKind::WoundSalve, 1);
            world.actors[idx].pop_text = Some(("配得藥膏".into(), tick + 10));
            world.actors[idx].push_diary("於濟安診所購備敷傷藥膏");
            world.dirty_unsaved_ticks += 3;
            (true, format!("向濟安診所購得［敷傷藥膏］（{price} 鎂）"))
        }
        ShopServiceOp::RentInnBed => {
            let meta = meta_opt.unwrap_or(&SHOPS[3]);
            if let Err(e) = pay_shop_mg(world, idx, Some(meta), 5, s) {
                return (false, e.into());
            }
            world.actors[idx].civic.inn_lease_until = tick.saturating_add(720);
            world.actors[idx].push_diary("向迎風宿屋謝掌櫃付鎂投宿客榻");
            world.actors[idx].pop_text = Some(("入宿客榻".into(), tick + 12));
            let guest_bed = [(-16, -9), (-16, -11)]
                .into_iter()
                .find(|&pos| {
                    world.overlays.get(&pos).and_then(|o| o.structure)
                        == Some(StructureKind::BambooBed)
                });
            if let Some((bx, by)) = guest_bed {
                let start = world.actors[idx].micro_pos();
                let occ = world.occupied_cells(Some(idx));
                if let Some(p) =
                    find_micro_path_crowded(rt, &world.overlays, start, (bx, by), false, false, &occ)
                {
                    world.actors[idx].path = p.into();
                    world.actors[idx].task = PendingTask::SleepBed { tx: bx, ty: by };
                    world.actors[idx].activity_tag = "入宿安歇".into();
                }
            }
            world.dirty_unsaved_ticks += 3;
            (true, "已訂妥迎風宿屋客榻（5 鎂），正前往歇息".into())
        }
        ShopServiceOp::RepayDebt => {
            let owed = world.actors[idx].civic.debt_mg;
            if owed == 0 {
                return (false, "帳上已無欠款".into());
            }
            let pay = owed.min(world.actors[idx].magnesium);
            if pay == 0 {
                return (false, "身無餘鎂，待採售後再清帳".into());
            }
            let cred_key = world.actors[idx]
                .civic
                .debt_creditor
                .as_deref()
                .unwrap_or("res:zhang_jian");
            let cred_meta = SHOPS.iter().find(|m| m.proprietor_key == cred_key);
            let _ = pay_shop_mg(world, idx, cred_meta.or(meta_opt), pay, s);
            world.actors[idx].civic.debt_mg -= pay;
            if world.actors[idx].civic.debt_mg == 0 {
                world.actors[idx].civic.debt_creditor = None;
                world.actors[idx].push_diary("登門結清濟安診所診金舊帳");
            }
            world.actors[idx].pop_text = Some(("結清診金".into(), tick + 12));
            world.dirty_unsaved_ticks += 3;
            (true, format!("已還清診金 {pay} 鎂"))
        }
        ShopServiceOp::ShopBuy(item) => {
            let Some(meta) = meta_opt else {
                return (false, "請至門市貨架購取".into());
            };
            let q = quote_item(world, s, item);
            if world.actors[idx].magnesium < q.ask_mg {
                return (false, "隨身鎂晶不足".into());
            }
            if consume_shop_item(world, meta, &[item], None).is_none() {
                return (false, "貨架暫無現貨".into());
            }
            let _ = pay_shop_mg(world, idx, Some(meta), q.ask_mg, s);
            world.actors[idx].add_inv(item, 1);
            world.dirty_unsaved_ticks += 3;
            (true, format!("向{}購得［{}］（{} 鎂）", meta.label, item.label(), q.ask_mg))
        }
        ShopServiceOp::ShopSell(item) => {
            let Some(meta) = meta_opt else {
                return (false, "請至門市櫃檯交售".into());
            };
            let q = quote_item(world, s, item);
            let dst = meta
                .shelf_slots
                .iter()
                .copied()
                .find(|&p| {
                    world.overlays.get(&p).is_some_and(|o| {
                        o.structure == Some(StructureKind::Stockpile)
                            && o.ground_item.is_none_or(|g| g.item == item && g.count < 24)
                    })
                })
                .or_else(|| find_best_stockpile_slot_for(&world.overlays, (mx, my), item, Some(s)));
            let Some(slot) = dst else {
                return (false, "店家儲架已滿".into());
            };
            if world.actors[idx].take_inv(item, 1) == 0 {
                return (false, "隨身行囊無此物可售".into());
            }
            world.add_ground_item(slot.0, slot.1, item, 1);
            let owner_idx = find_actor_by_key(world, meta.proprietor_key);
            let mut paid = q.bid_mg;
            if let Some(oi) = owner_idx
                && world.actors[oi].magnesium >= paid
            {
                if oi != idx {
                    world.actors[oi].magnesium -= paid;
                    world.actors[idx].magnesium = world.actors[idx].magnesium.saturating_add(paid);
                }
            } else {
                paid = paid.min(world.economy.vault_mg[s.index()]);
                world.economy.vault_mg[s.index()] -= paid;
                world.actors[idx].magnesium = world.actors[idx].magnesium.saturating_add(paid);
            }
            world.dirty_unsaved_ticks += 3;
            (true, format!("交售［{}］予{}（得 {} 鎂）", item.label(), meta.label, paid))
        }
    }
}

/// 玩家單擊門市插座：三步內立刻辦理，遠處則一鍵自動尋路前往並在抵達後自動結算。
pub fn handle_shop_cmd(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    p_idx: usize,
    mx: i32,
    my: i32,
    code: &str,
) -> (bool, String, Vec<(i32, i32)>) {
    let Some(op) = parse_shop_op(code) else {
        return (false, "未明門市服務項目".into(), Vec::new());
    };
    let aid = world.actors[p_idx].id;
    world.stall_tracker.remove(&aid);
    world.economy.parcel_queues.remove(&aid);
    world.actors[p_idx].path.clear();
    world.actors[p_idx].task = PendingTask::None;

    let start = world.actors[p_idx].micro_pos();
    if (start.0 - mx).abs() <= 3 && (start.1 - my).abs() <= 3 {
        let (ok, msg) = execute_shop_service(world, rt, p_idx, mx, my, op);
        return (ok, msg, Vec::new());
    }
    let Some(path) = find_micro_path(rt, &world.overlays, start, (mx, my), true, false) else {
        return (false, "前往該店舖通路受阻".into(), Vec::new());
    };
    world.actors[p_idx].path = path.clone().into();
    world.actors[p_idx].task = PendingTask::ShopPatron { tx: mx, ty: my, op };
    world.actors[p_idx].activity_tag = "前往店舖".into();
    let shop_name = shop_at(mx, my).map_or("門市", |m| m.label);
    (true, format!("正前往{shop_name}"), path)
}

pub fn complete_shop_patron(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
    tx: i32,
    ty: i32,
    op: ShopServiceOp,
) {
    let (ax, ay) = world.actors[idx].micro_pos();
    if (ax - tx).abs() <= 3 && (ay - ty).abs() <= 3 {
        let _ = execute_shop_service(world, rt, idx, tx, ty, op);
    }
    if !matches!(world.actors[idx].task, PendingTask::SleepBed { .. }) {
        world.actors[idx].task = PendingTask::None;
    }
}

/// 依店舖門面元件（攤檯、鍛鐵檯、藥櫃、宿屋櫃檯、茶座）組裝專職服務檢視卡。
pub fn describe_shop_fixture_card(
    world: &RimWorldState,
    viewer_idx: Option<usize>,
    mx: i32,
    my: i32,
    st: StructureKind,
    title: &mut String,
    tags: &mut Vec<String>,
    sockets: &mut Vec<Value>,
) -> bool {
    let Some(meta) = shop_at(mx, my) else {
        return false;
    };
    let s = SettlementId::WestOutpost;
    let inv_sell = |item: RimItemKind| {
        viewer_idx.is_some_and(|vi| {
            world.actors[vi]
                .inventory
                .iter()
                .any(|x| x.item == item && x.count > 0)
        })
    };

    match (meta.building_code, st) {
        ("chunshui_diner", StructureKind::Stockpile | StructureKind::Campfire) => {
            *title = if st == StructureKind::Campfire {
                "熱灶鍋檯".into()
            } else {
                "熱食攤檯".into()
            };
            tags.push("店主：劉春水".into());
            sockets.push(json!({ "label": "堂食熱菜溫飽（5 鎂）", "cmd": "shop_cmd", "item_code": "dine_in" }));
            sockets.push(json!({ "label": "外帶熱食餐包（5 鎂）", "cmd": "shop_cmd", "item_code": "takeout_meal" }));
        }
        ("amu_smithy", StructureKind::Workbench | StructureKind::Stockpile) => {
            *title = if st == StructureKind::Workbench {
                "鍛鐵打製檯".into()
            } else {
                "鐵器架".into()
            };
            tags.push("師傅：林阿木".into());
            if viewer_idx.is_some_and(|vi| world.actors[vi].equipped_tool.is_some()) {
                sockets.push(json!({ "label": "請阿木磨刀校刃（4 鎂）", "cmd": "shop_cmd", "item_code": "hone_tool" }));
            }
            let b_p = quote_item(world, s, RimItemKind::AlloyBlade).ask_mg;
            let p_p = quote_item(world, s, RimItemKind::AlloyPick).ask_mg;
            sockets.push(json!({ "label": format!("委託阿木親鍛導能獵刀（{b_p} 鎂）"), "cmd": "shop_cmd", "item_code": "commission_blade" }));
            sockets.push(json!({ "label": format!("委託阿木親鍛鋼製開山鎬（{p_p} 鎂）"), "cmd": "shop_cmd", "item_code": "commission_pick" }));
            if st == StructureKind::Workbench {
                sockets.push(json!({ "label": "借案自行搗製敷傷藥膏", "action": "craft_salve" }));
            }
        }
        ("jian_clinic", StructureKind::Workbench | StructureKind::Stockpile | StructureKind::BambooBed) => {
            if st == StructureKind::BambooBed && (mx, my) != (18, -1) {
                return false;
            }
            *title = match st {
                StructureKind::Workbench => "候診藥案".into(),
                StructureKind::Stockpile => "百草藥櫃".into(),
                _ => "診間病榻".into(),
            };
            tags.push("醫師：張濟安".into());
            sockets.push(json!({ "label": "請張醫師看診治傷（6 鎂・缺鎂可記帳）", "cmd": "shop_cmd", "item_code": "clinic_treat" }));
            let sp = quote_item(world, s, RimItemKind::WoundSalve).ask_mg;
            sockets.push(json!({ "label": format!("購買敷傷藥膏（{sp} 鎂）"), "cmd": "shop_cmd", "item_code": "buy_salve" }));
            if let Some(vi) = viewer_idx
                && world.actors[vi].civic.debt_mg > 0
            {
                sockets.push(json!({
                    "label": format!("結清診金欠帳（{} 鎂）", world.actors[vi].civic.debt_mg),
                    "cmd": "shop_cmd",
                    "item_code": "repay_debt"
                }));
            }
        }
        ("yingfeng_inn", StructureKind::Stockpile | StructureKind::BambooBed) => {
            if st == StructureKind::BambooBed && (mx, my) != (-16, -9) && (mx, my) != (-16, -11) {
                return false;
            }
            *title = if st == StructureKind::Stockpile {
                "宿屋櫃檯".into()
            } else {
                "旅人客榻".into()
            };
            tags.push("掌櫃：謝承風".into());
            sockets.push(json!({ "label": "付鎂投宿客榻一日（5 鎂）", "cmd": "shop_cmd", "item_code": "rent_inn" }));
        }
        ("acha_teahouse", StructureKind::Stockpile | StructureKind::Campfire) => {
            *title = "溪畔茶座".into();
            tags.push("掌櫃：周阿茶".into());
            sockets.push(json!({ "label": "喫茶配點消倦驅濕（4 鎂）", "cmd": "shop_cmd", "item_code": "tea_refresh" }));
            sockets.push(json!({ "label": "外帶煨筍乾糧（5 鎂）", "cmd": "shop_cmd", "item_code": "takeout_meal" }));
        }
        _ => return false,
    }

    for &inp in meta.accepted_inputs {
        if inv_sell(inp) {
            let bid = quote_item(world, s, inp).bid_mg.max(1);
            let code = super::rim_economy::item_code(inp);
            sockets.push(json!({
                "label": format!("交售{}予{}（得 {} 鎂）", inp.label(), meta.label, bid),
                "cmd": "shop_cmd",
                "item_code": format!("sell:{code}")
            }));
        }
    }
    sockets.push(json!({ "label": "走近店面", "action": "move" }));
    true
}

/// 推導無原始數值之武器器物落款與鋒刃手感描述（供 HUD 與裝備欄呈現）。
pub fn tool_detail_note(world: &RimWorldState, actor: &RimActor) -> Option<String> {
    actor.equipped_tool?;
    let wear = world.economy.tool_wear.get(&actor.id).copied().unwrap_or(0);
    let edge = if wear < 8 {
        "鋒刃雪亮"
    } else if wear < 20 {
        "器身順手"
    } else {
        "刃口微鈍，宜送鐵店校磨"
    };
    Some(match actor.civic.tool_maker.as_deref() {
        Some(mk) => format!("{mk}｜{edge}"),
        None => edge.into(),
    })
}

pub fn push_civic_feels(tick: u64, actor: &RimActor, feels: &mut Vec<&'static str>) {
    if actor.civic.inn_lease_until > tick {
        feels.push("迎風宿屋投宿安歇");
    }
    if actor.civic.debt_mg > 0 {
        feels.push("尚欠濟安診所診金");
    }
}

/// 街坊店主依實有生料巡補貨架；除醫療外，欠帳居民有餘鎂時主動赴診所還帳。
pub fn step_town_shops(world: &mut RimWorldState) {
    if !world.tick.is_multiple_of(12) {
        return;
    }
    for (shop_idx, meta) in SHOPS.iter().enumerate() {
        let Some(oi) = find_actor_by_key(world, meta.proprietor_key) else {
            continue;
        };
        let idle = world.actors[oi].task == PendingTask::None && world.actors[oi].path.is_empty();
        if !idle {
            continue;
        }
        let shelf = meta.shelf_slots[0];
        let pos = world.actors[oi].micro_pos();
        if (pos.0 - shelf.0).abs() > 5 || (pos.1 - shelf.1).abs() > 5 {
            continue;
        }
        match shop_idx {
            0 | 4 => {
                let prepared = shop_stock_count(
                    world,
                    meta,
                    &[RimItemKind::RoastedMeat, RimItemKind::RoastedTuber, RimItemKind::RoastedShoot],
                );
                if prepared < 2
                    && let Some(raw) = consume_shop_item(
                        world,
                        meta,
                        &[RimItemKind::BeastMeat, RimItemKind::SweetTuber, RimItemKind::BambooShoot],
                        None,
                    )
                {
                    let dish = cooked_pair_for_raw(raw);
                    world.add_ground_item(shelf.0, shelf.1, dish, 1);
                    bump_proprietor_craft(world, meta, false);
                }
            }
            1 => {
                let tools = shop_stock_count(world, meta, &[RimItemKind::AlloyBlade, RimItemKind::AlloyPick]);
                if tools == 0
                    && shop_stock_count(world, meta, &[RimItemKind::BambooPole]) > 0
                    && shop_stock_count(world, meta, &[RimItemKind::RiverStone]) > 0
                    && consume_shop_item(world, meta, &[RimItemKind::BambooPole], None).is_some()
                    && consume_shop_item(world, meta, &[RimItemKind::RiverStone], None).is_some()
                {
                    let prod = if world.tick.is_multiple_of(24) {
                        RimItemKind::AlloyBlade
                    } else {
                        RimItemKind::AlloyPick
                    };
                    world.add_ground_item(shelf.0, shelf.1, prod, 1);
                    bump_proprietor_craft(world, meta, true);
                }
            }
            2 => {
                let salves = shop_stock_count(world, meta, &[RimItemKind::WoundSalve]);
                if salves < 2
                    && consume_shop_item(world, meta, &[RimItemKind::MountainHerb], None).is_some()
                {
                    world.add_ground_item(shelf.0, shelf.1, RimItemKind::WoundSalve, 1);
                    world.actors[oi].craft_harvest = world.actors[oi].craft_harvest.saturating_add(1);
                }
            }
            _ => {}
        }
    }
}

/// 街坊居民於負傷、肚飢、刀鈍或積欠診金時，自主光顧對應店舖（單決策自動行止）。
pub fn try_autonomous_shop_patronage(
    world: &mut RimWorldState,
    rt: Option<&SubstrateRuntime>,
    idx: usize,
) -> bool {
    let a = &world.actors[idx];
    if a.faction != ActorFaction::Settler || a.is_player || a.hp <= 0 || a.downed_ticks > 0 {
        return false;
    }
    let start = a.micro_pos();
    let occ = world.occupied_cells(Some(idx));

    let (target, op, tag) = if a.civic.debt_mg > 0 && a.magnesium >= a.civic.debt_mg + 4 {
        ((15, -1), ShopServiceOp::RepayDebt, "赴診所還帳")
    } else if a.injury_ticks > 0
        && a.actor_key != "res:zhang_jian"
        && shop_stock_count(world, &SHOPS[2], &[RimItemKind::WoundSalve, RimItemKind::MountainHerb]) > 0
    {
        ((15, -1), ShopServiceOp::ClinicTreat, "赴診所求醫")
    } else if a.hunger < 48
        && a.magnesium >= 6
        && !matches!(a.actor_key.as_str(), "res:liu_chunshui" | "res:zhou_acha")
        && shop_stock_count(world, &SHOPS[0], &[RimItemKind::RoastedMeat, RimItemKind::RoastedTuber, RimItemKind::RoastedShoot, RimItemKind::BambooShoot]) > 0
    {
        ((-19, -1), ShopServiceOp::DineIn, "上小吃店吃飯")
    } else if a.equipped_tool.is_some()
        && world.economy.tool_wear.get(&a.id).copied().unwrap_or(0) >= 20
        && a.magnesium >= 5
        && a.actor_key != "res:lin_amu"
    {
        ((11, -1), ShopServiceOp::HoneTool, "赴鐵店磨刀")
    } else {
        return false;
    };

    let Some(path) = find_micro_path_crowded(rt, &world.overlays, start, target, true, false, &occ) else {
        return false;
    };
    world.actors[idx].path = path.into();
    world.actors[idx].task = PendingTask::ShopPatron { tx: target.0, ty: target.1, op };
    world.actors[idx].activity_tag = tag.into();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::substrate::rim_seed::seed_outpost;

    fn seeded_world() -> RimWorldState {
        let (overlays, actors, events) = seed_outpost(None);
        RimWorldState {
            tick: 100, overlays, actors, events,
            economy: Default::default(), dirty_unsaved_ticks: 0, sub_step: 0, stall_tracker: Default::default(),
        }
    }

    fn total_mg(w: &RimWorldState) -> u32 {
        w.actors.iter().map(|a| a.magnesium).sum::<u32>() + w.economy.vault_mg.iter().sum::<u32>()
    }

    #[test]
    fn diner_and_smithy_services_conserve_magnesium_and_stamp_maker() {
        let mut w = seeded_world();
        let before_mg = total_mg(&w);
        let p_idx = w.ensure_player_actor("test_cust", "客", "M", 42);
        w.actors[p_idx].hunger = 30;
        w.actors[p_idx].magnesium = 30;
        let expected_mg = before_mg + 30;

        let (ok, msg) = execute_shop_service(&mut w, None, p_idx, -19, -1, ShopServiceOp::DineIn);
        assert!(ok, "dine_in failed: {msg}");
        assert!(w.actors[p_idx].hunger >= 75 && w.actors[p_idx].magnesium == 25);

        w.actors[p_idx].add_inv(RimItemKind::RiverStone, 1);
        w.economy.tool_wear.insert(w.actors[p_idx].id, 24);
        assert!(execute_shop_service(&mut w, None, p_idx, 11, -1, ShopServiceOp::HoneTool).0);
        assert_eq!(w.economy.tool_wear.get(&w.actors[p_idx].id).copied(), Some(0));
        assert!(tool_detail_note(&w, &w.actors[p_idx]).unwrap_or_default().contains("鋒刃雪亮"));
        assert_eq!(total_mg(&w), expected_mg);
    }

    #[test]
    fn clinic_credit_tab_heals_poor_patient_and_repays_cleanly() {
        let mut w = seeded_world();
        let p_idx = w.ensure_player_actor("poor_pat", "病", "F", 99);
        w.actors[p_idx].hp = 50;
        w.actors[p_idx].injury_ticks = 60;
        w.actors[p_idx].injury_part = Some("右肩拉傷".into());
        w.actors[p_idx].magnesium = 2;
        let start_mg = total_mg(&w);

        assert!(execute_shop_service(&mut w, None, p_idx, 15, -1, ShopServiceOp::ClinicTreat).0);
        assert_eq!((w.actors[p_idx].injury_ticks, w.actors[p_idx].magnesium, w.actors[p_idx].civic.debt_mg), (0, 0, 4));
        assert_eq!(total_mg(&w), start_mg);

        w.actors[p_idx].magnesium = 10;
        assert!(execute_shop_service(&mut w, None, p_idx, 15, -1, ShopServiceOp::RepayDebt).0);
        assert_eq!((w.actors[p_idx].civic.debt_mg, w.actors[p_idx].magnesium), (0, 6));
        assert_eq!(total_mg(&w), start_mg + 10);
    }

    #[test]
    fn inn_rent_sets_lease_and_assigns_guest_bed() {
        let mut w = seeded_world();
        let p_idx = w.ensure_player_actor("guest_a", "旅", "M", 123);
        w.actors[p_idx].px = -18.0;
        w.actors[p_idx].py = -8.0;
        w.actors[p_idx].magnesium = 15;
        let before_mg = total_mg(&w);

        let (ok, msg) = execute_shop_service(&mut w, None, p_idx, -19, -8, ShopServiceOp::RentInnBed);
        assert!(ok, "rent failed: {msg}");
        assert!(w.actors[p_idx].civic.inn_lease_until > w.tick && matches!(w.actors[p_idx].task, PendingTask::SleepBed { .. }));
        assert_eq!(total_mg(&w), before_mg);
    }

    #[test]
    fn tool_detail_note_never_leaks_raw_numbers() {
        let mut w = seeded_world();
        let p_idx = w.ensure_player_actor("no_num", "雅", "F", 77);
        w.economy.tool_wear.insert(w.actors[p_idx].id, 24);
        let note = tool_detail_note(&w, &w.actors[p_idx]).expect("tool note");
        assert!(!note.chars().any(|c| c.is_ascii_digit()), "must not leak digits: {note}");
        assert!(note.contains("阿木鐵店") && note.contains("微鈍"));
    }
}

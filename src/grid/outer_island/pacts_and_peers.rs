//! 銀鷗島「銀號／港務公櫃保人託管契約（`contract_escrow`）」模組（並重匯出 `peer_interactions` 對等互動動詞）。
//!
//! 嚴格落實審辯室 Room `7eee7478-1493-4faf-b7e3-12de460402e3` 定案：
//! 1. 保人託管守恆：所有委託賞金立契時即經 `delta.transfer_copper` 轉入 `"contract_escrow"`（納入 `120,805` 守恆左式）；
//! 2. 禁止環形轉包（`A -> B -> A`）與兩振失信黑名單；
//! 3. 逾期、傷退、離世絕不無痕系統重置——由銀號／港務公櫃掛牌公告結算並將押金退還原主（原主若亡則轉交同戶至親或市政遺產保管）；不可抗力死亡不冤記承接人違約。

use super::appraisal::ConcernCause;
use super::clock_and_lang::SolarClockSnapshot;
use super::mind::EventPublicity;
use super::movement::{add_carry_item, push_breath, take_clean_carry_items};
use super::persona::{ValueAxis, derive_innate_persona};
use super::sim_tick::broadcast_scene_event;
use super::types::{CarryItem, ContractRecord, IslandWorldDelta, WorldStaticDoc};

pub use super::peer_interactions::{
    apply_peer_assist_skirmish, apply_peer_consensual_spar, apply_peer_direct_speech,
    apply_peer_item_trade, check_and_expose_unwitnessed_boast,
};

const ESCROW_PACT_KINDS: &[&str] = &[
    "courier_errand",
    "supply_commission",
    "shop_cover",
    "escort_watch",
];

pub fn is_escrow_pact_kind(kind: &str) -> bool {
    ESCROW_PACT_KINDS.contains(&kind)
}

/// 解析結構化契約附註 `target_or_slug｜human_readable_desc`
pub fn parse_pact_note(note_zh: &str) -> (&str, &str) {
    note_zh.split_once('｜').unwrap_or(("", note_zh))
}

/// 檢查是否存在環形轉包（`issuer` 正欠 `candidate` 未結託付時，禁止反向承接）或失信黑名單
pub fn is_blocked_from_accepting_pact(
    delta: &IslandWorldDelta,
    issuer_id: &str,
    candidate_id: &str,
) -> Option<&'static str> {
    if issuer_id == candidate_id {
        return Some("不能自己承接自己發出的託付。");
    }
    let breaches = delta
        .contracts
        .values()
        .filter(|c| {
            c.debtor_id == candidate_id && (c.status == "breached" || c.status == "written_off")
        })
        .count();
    if breaches >= 2 {
        return Some("銀號公櫃查出名下已有兩筆違約呆帳紀錄，保人拒絕為此次承接作保。");
    }
    let mut visited = std::collections::BTreeSet::new();
    let mut stack = vec![issuer_id.to_string()];
    while let Some(cur) = stack.pop() {
        if !visited.insert(cur.clone()) {
            continue;
        }
        for c in delta.contracts.values() {
            if is_escrow_pact_kind(&c.kind)
                && (c.status == "escrowed" || c.status == "in_progress")
                && c.debtor_id == cur
            {
                if c.creditor_id == candidate_id {
                    return Some("雙方已有反向未結託付，為防連環轉包死鎖，銀號不予立契。");
                }
                stack.push(c.creditor_id.clone());
            }
        }
    }
    None
}

/// 發起一筆由銀號／港務公櫃保人託管賞金之真實請託契（玩家與居民共用）
pub fn propose_escrow_pact(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    issuer_key: &str,
    kind: &str,
    target_meta: &str,
    reward_copper: i64,
) -> Result<String, String> {
    if !is_escrow_pact_kind(kind) {
        return Err("銀號木冊暫無此類契約格式。".to_string());
    }
    let reward = reward_copper.clamp(3, 60);
    let issuer_id = delta
        .resolve_actor_id(issuer_key)
        .ok_or("請先向港務官核對通行身分。")?;

    let already_open = delta
        .contracts
        .values()
        .filter(|c| {
            c.creditor_id == issuer_id
                && c.kind == kind
                && (c.status == "escrowed" || c.status == "in_progress")
        })
        .count();
    if already_open >= 1 {
        return Err("你名下已有一筆同類待辦託付在銀號掛牌中，待結清後再發新契。".to_string());
    }

    let (issuer_name, human_desc, norm_meta) = {
        let a = delta.actors.get(&issuer_id).ok_or("找不到發包人")?;
        if !a.alive {
            return Err("發包人不在島上。".to_string());
        }
        match kind {
            "courier_errand" => {
                let rec_id = if target_meta.is_empty() {
                    a.mind
                        .ltm
                        .social
                        .values()
                        .filter(|s| {
                            !s.cohabiting
                                && !s.target_id.starts_with("pl:")
                                && s.warmth >= 25
                                && !s.target_region_zh.is_empty()
                        })
                        .max_by_key(|s| s.warmth)
                        .map(|s| s.target_id.clone())
                        .ok_or("目前沒有需要跨鎮捎信的外鎮親故。")?
                } else {
                    delta
                        .resolve_public_target(target_meta)
                        .or_else(|| delta.resolve_actor_id(target_meta))
                        .ok_or("找不到收信對象。")?
                };
                if rec_id == issuer_id {
                    return Err("無須託人捎信給自己。".to_string());
                }
                let rec = delta
                    .actors
                    .get(&rec_id)
                    .filter(|r| r.alive)
                    .ok_or("找不到收信對象。")?;
                let rec_town = super::world_weave_bio::settlement_name_zh(doc, rec);
                let desc = format!(
                    "{}託人將封緘家書與平安口信送交{}的{}（賞金 {} 枚銅幣）",
                    a.display_name, rec_town, rec.display_name, reward
                );
                (a.display_name.clone(), desc, rec_id)
            }
            "supply_commission" => {
                let slug = match target_meta {
                    "rune_stone" | "spider_carapace" | "lizard_scale" | "iron_ore"
                    | "driftwood" | "herb_bundle" => target_meta,
                    _ => {
                        if a.role.contains("藥") {
                            "herb_bundle"
                        } else if a.role.contains("木") {
                            "driftwood"
                        } else {
                            "iron_ore"
                        }
                    }
                };
                let iname = match slug {
                    "rune_stone" => "古符文原石",
                    "spider_carapace" => "吞石巨蛛厚甲殼",
                    "lizard_scale" => "嗜石岩蜥厚鱗皮",
                    "driftwood" => "異國刻紋漂流木",
                    "herb_bundle" => "山林止血草束",
                    _ => "粗鐵礦石",
                };
                let desc = format!(
                    "{}懸賞收購一份乾淨無贓的【{}】（銀號保付 {} 枚銅幣）",
                    a.display_name, iname, reward
                );
                (a.display_name.clone(), desc, slug.to_string())
            }
            "shop_cover" => {
                let wn_name = doc
                    .nodes
                    .get(&a.work_node)
                    .map_or(a.work_node.as_str(), |n| n.name.as_str());
                let desc = format!(
                    "{}因傷乏暫離崗位，請人赴【{}】替班看顧鋪面活計（工資 {} 枚銅幣）",
                    a.display_name, wn_name, reward
                );
                (a.display_name.clone(), desc, a.work_node.clone())
            }
            _ => {
                let dst = if target_meta.is_empty() || !doc.nodes.contains_key(target_meta) {
                    if a.node_id.starts_with("oi:west_strait_ferry") {
                        "oi:white_gull_harbor:plaza"
                    } else {
                        "oi:west_strait_ferry:plaza"
                    }
                } else {
                    target_meta
                };
                let dname = doc.nodes.get(dst).map_or(dst, |n| n.name.as_str());
                let desc = format!(
                    "{}徵求同道結伴護行前往【{}】（抵達保付 {} 枚銅幣）",
                    a.display_name, dname, reward
                );
                (a.display_name.clone(), desc, dst.to_string())
            }
        }
    };

    if !delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "escrow_lock",
        &issuer_id,
        "contract_escrow",
        reward,
        format!("立契託管賞金：{}", human_desc),
    ) {
        return Err(format!(
            "立契需先將 {} 枚銅幣交予銀號公櫃託管，你手頭銅板不足。",
            reward
        ));
    }

    delta.ledger_seq_counter = delta.ledger_seq_counter.saturating_add(1);
    let cid = format!(
        "pact:{}:{}:{}:{}",
        kind, issuer_id, clock.game_days, delta.ledger_seq_counter
    );
    delta.dirty_contracts.insert(cid.clone());
    delta.contracts.insert(
        cid.clone(),
        ContractRecord {
            contract_id: cid,
            kind: kind.to_string(),
            creditor_id: issuer_id.clone(),
            debtor_id: String::new(),
            amount_copper: reward,
            due_day: clock.game_days + 4,
            status: "escrowed".to_string(),
            note_zh: format!("{}｜{}", norm_meta, human_desc),
        },
    );
    let msg = format!(
        "已將 {} 枚銅幣交予銀號公櫃封存作保，立下託付契：【{}】（四日內有效）。",
        reward, human_desc
    );
    if let Some(a) = delta.actors.get_mut(&issuer_id) {
        push_breath(a, format!("{}立下託付：{}", issuer_name, human_desc));
        a.mark_mutated();
    }
    Ok(msg)
}

/// 當面點頭承接一筆託管請託契（捎信契當場交付封緘家書實物！）
pub fn accept_escrow_pact(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    contractor_key: &str,
    contract_id: &str,
) -> Result<String, String> {
    let contractor_id = delta
        .resolve_actor_id(contractor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let c_snap = delta
        .contracts
        .get(contract_id)
        .cloned()
        .ok_or("找不到這筆託付契。")?;
    if c_snap.status != "escrowed" || !c_snap.debtor_id.is_empty() {
        return Err("這筆託付已有人搶先接手或已結清。".to_string());
    }
    if !delta
        .actors
        .get(&c_snap.creditor_id)
        .is_some_and(|iss| iss.alive)
    {
        return Err("發包人已不在島上，銀號正辦理押金清退。".to_string());
    }
    if let Some(reason) = is_blocked_from_accepting_pact(delta, &c_snap.creditor_id, &contractor_id)
    {
        return Err(reason.to_string());
    }

    let (con_node, con_name) = {
        let con = delta.actors.get(&contractor_id).ok_or("找不到承接人")?;
        (con.node_id.clone(), con.display_name.clone())
    };
    let iss_cell = delta.actors.get(&c_snap.creditor_id).and_then(|iss| {
        doc.nodes
            .get(&iss.node_id)
            .map(|n| (iss.node_id.as_str(), n.cell_id))
    });
    let con_cell = doc.nodes.get(&con_node).map(|n| (&n.kind, n.cell_id));
    let same_spot_or_town_board = match (iss_cell, con_cell) {
        (Some((inid, icell)), Some((ckind, ccell))) => {
            inid == con_node || (icell == ccell && (*ckind == "plaza" || *ckind == "shop"))
        }
        _ => false,
    };
    if !same_spot_or_town_board {
        return Err("需在發包人當面或同鎮廣場／酒館公櫃前才能點頭接下這樁託付。".to_string());
    }

    let (meta_target, desc_zh) = parse_pact_note(&c_snap.note_zh);
    let meta_owned = meta_target.to_string();
    let desc_owned = desc_zh.to_string();

    if c_snap.kind == "courier_errand" {
        let rec_name = delta
            .actors
            .get(&meta_owned)
            .filter(|r| r.alive)
            .map(|r| r.display_name.clone())
            .ok_or("收信人目前不在島上，暫無法承接。")?;
        if let Some(con) = delta.actors.get_mut(&contractor_id) {
            add_carry_item(
                &mut con.backpack,
                CarryItem {
                    slug: "sealed_errand_pkg".to_string(),
                    name: format!("致{}的封緘家書", rec_name),
                    count: 1,
                    bound_owner: Some(contract_id.to_string()),
                    stolen_from: None,
                    weight_units: 1,
                    vessel_meta: None,
                },
            );
            con.mark_mutated();
        }
    }

    if let Some(c) = delta.contracts.get_mut(contract_id) {
        c.debtor_id = contractor_id.clone();
        c.status = "in_progress".to_string();
        delta.dirty_contracts.insert(contract_id.to_string());
    }
    if c_snap.kind == "escort_watch"
        && let Some(iss) = delta.actors.get_mut(&c_snap.creditor_id)
        && iss.controller_id.is_none()
    {
        iss.route_plan.clear();
        iss.mark_mutated();
    }
    let _ = clock;
    let msg = format!(
        "{}當面點頭應下託付：【{}】，到期日前辦妥即可領取銀號託管賞金 {} 枚銅幣。",
        con_name, desc_owned, c_snap.amount_copper
    );
    if let Some(con) = delta.actors.get_mut(&contractor_id) {
        push_breath(con, msg.clone());
        con.mark_mutated();
    }
    Ok(msg)
}

/// 履約驗收結清：逐類檢查在場與實物條件，通過後由 `"contract_escrow"` 原子撥付賞金！
pub fn fulfill_escrow_pact(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    contract_id: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let c_snap = delta
        .contracts
        .get(contract_id)
        .cloned()
        .ok_or("找不到這筆託付契。")?;
    if c_snap.status != "escrowed" && c_snap.status != "in_progress" {
        return Err("這筆託付契已結案或已失效。".to_string());
    }
    if !c_snap.debtor_id.is_empty() && c_snap.debtor_id != aid {
        return Err("這筆託付已由他人承接辦理中。".to_string());
    }
    if c_snap.status == "escrowed"
        && let Some(reason) = is_blocked_from_accepting_pact(delta, &c_snap.creditor_id, &aid)
    {
        return Err(reason.to_string());
    }

    let (meta_target, desc_zh) = parse_pact_note(&c_snap.note_zh);
    let meta_owned = meta_target.to_string();
    let desc_owned = desc_zh.to_string();
    let cur_node = delta
        .actors
        .get(&aid)
        .map(|a| a.node_id.clone())
        .ok_or("找不到行蹤")?;

    match c_snap.kind.as_str() {
        "courier_errand" => {
            let rec_node = delta
                .actors
                .get(&meta_owned)
                .filter(|r| r.alive)
                .map(|r| r.node_id.clone())
                .ok_or("收信人目前不在島上。")?;
            if cur_node != rec_node {
                return Err("需親自走到收信人面前，才能當面遞交封緘家書。".to_string());
            }
            let pkg_opt = delta.actors.get_mut(&aid).and_then(|a| {
                let pos = a.backpack.iter().position(|i| {
                    i.slug == "sealed_errand_pkg"
                        && i.bound_owner.as_deref() == Some(contract_id)
                        && i.count > 0
                })?;
                let removed = a.backpack.remove(pos);
                a.mark_mutated();
                Some(removed)
            });
            let Some(pkg_item) = pkg_opt else {
                return Err("你身上沒有這封對應的封緘家書（須先向發信人接下家書）。".to_string());
            };
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "escrow_payout",
                "contract_escrow",
                &aid,
                c_snap.amount_copper,
                format!("捎信履約結清：{}", desc_owned),
            ) {
                if let Some(a) = delta.actors.get_mut(&aid) {
                    add_carry_item(&mut a.backpack, pkg_item);
                }
                return Err("銀號託管帳目核對未過。".to_string());
            }
            if let Some(a) = delta.actors.get_mut(&aid) {
                a.mind.ltm.mastery.wayfare_crossings =
                    a.mind.ltm.mastery.wayfare_crossings.saturating_add(1);
                a.mark_mutated();
            }
            if let Some(iss) = delta.actors.get_mut(&c_snap.creditor_id) {
                iss.mind.resolve_concerns_matching(
                    ConcernCause::CrossTownYearning,
                    Some(&meta_owned),
                    45,
                );
                iss.mind.stm.today_goal_progressed = true;
                iss.mark_mutated();
            }
            if let Some(rec) = delta.actors.get_mut(&meta_owned) {
                rec.mind.resolve_concerns_matching(
                    ConcernCause::CrossTownYearning,
                    Some(&c_snap.creditor_id),
                    45,
                );
                rec.mark_mutated();
            }
            let _ = super::phase_and_vessels::record_craft_work_and_check_phase(
                doc,
                delta,
                clock,
                &aid,
                "cross_town_courier",
                "wayfare",
                &cur_node,
            );
        }
        "supply_commission" => {
            let iss_place_ok = delta.actors.get(&c_snap.creditor_id).is_some_and(|iss| {
                iss.alive && (iss.node_id == cur_node || iss.work_node == cur_node)
            });
            if !iss_place_ok {
                return Err("需站在委託人面前或其鋪面內才能點交貨品。".to_string());
            }
            let taken_opt = delta.actors.get_mut(&aid).and_then(|a| {
                let t = take_clean_carry_items(&mut a.backpack, &meta_owned, 1)?;
                a.mark_mutated();
                Some(t)
            });
            let Some(delivered_item) = taken_opt else {
                return Err("你背囊裡沒有乾淨無贓的指定貨品可交付。".to_string());
            };
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "escrow_payout",
                "contract_escrow",
                &aid,
                c_snap.amount_copper,
                format!("代購交貨結清：{}", desc_owned),
            ) {
                if let Some(a) = delta.actors.get_mut(&aid) {
                    add_carry_item(&mut a.backpack, delivered_item);
                }
                return Err("銀號託管帳目核對未過。".to_string());
            }
            if let Some(iss) = delta.actors.get_mut(&c_snap.creditor_id) {
                add_carry_item(&mut iss.backpack, delivered_item);
                iss.mind.stm.today_goal_progressed = true;
                iss.mark_mutated();
            }
        }
        "shop_cover" => {
            if cur_node != meta_owned {
                return Err("需親自站在委託鋪面內才能替班當值。".to_string());
            }
            if delta.actors.get(&aid).is_some_and(|a| a.fatigue >= 80) {
                return Err("你已經氣喘吁吁，先歇息再替人看店吧。".to_string());
            }
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "escrow_payout",
                "contract_escrow",
                &aid,
                c_snap.amount_copper,
                format!("鋪面替班結清：{}", desc_owned),
            ) {
                return Err("銀號託管帳目核對未過。".to_string());
            }
            if let Some(a) = delta.actors.get_mut(&aid) {
                a.fatigue = (a.fatigue + 14).min(95);
                a.mind.stm.today_goal_progressed = true;
                a.mark_mutated();
            }
        }
        _ => {
            if c_snap.status != "in_progress" || c_snap.debtor_id != aid {
                return Err(
                    "須先接下同行護衛契並一路護送委託人抵達約定地點，方能當面結清。".to_string(),
                );
            }
            let together = delta
                .actors
                .get(&c_snap.creditor_id)
                .is_some_and(|iss| iss.alive && iss.node_id == cur_node)
                && cur_node == meta_owned;
            if !together {
                return Err("需與委託人一同平安抵達約定地點方能結清護行契。".to_string());
            }
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "escrow_payout",
                "contract_escrow",
                &aid,
                c_snap.amount_copper,
                format!("同行護衛結清：{}", desc_owned),
            ) {
                return Err("銀號託管帳目核對未過。".to_string());
            }
            if let Some(iss) = delta.actors.get_mut(&c_snap.creditor_id) {
                iss.mind
                    .resolve_concerns_matching(ConcernCause::CrossTownYearning, None, 45);
                iss.mind.stm.today_goal_progressed = true;
                iss.mark_mutated();
            }
        }
    }

    if let Some(c) = delta.contracts.get_mut(contract_id) {
        c.debtor_id = aid.clone();
        c.amount_copper = 0;
        c.status = "settled".to_string();
        delta.dirty_contracts.insert(contract_id.to_string());
    }

    let (iss_name, con_name) = (
        delta
            .actors
            .get(&c_snap.creditor_id)
            .map_or("委託人".to_string(), |x| x.display_name.clone()),
        delta
            .actors
            .get(&aid)
            .map_or("旅人".to_string(), |x| x.display_name.clone()),
    );
    if let Some(iss) = delta.actors.get_mut(&c_snap.creditor_id) {
        iss.mind.record_pact_or_trade_evidence(
            &aid,
            &con_name,
            clock.game_days,
            true,
            false,
            "依約辦妥銀號託付契",
        );
        iss.mark_mutated();
    }
    if let Some(con) = delta.actors.get_mut(&aid) {
        con.mind.record_pact_or_trade_evidence(
            &c_snap.creditor_id,
            &iss_name,
            clock.game_days,
            true,
            false,
            "依約辦妥銀號託付契",
        );
        con.mind.stm.today_goal_progressed = true;
        let msg = format!(
            "依約辦妥【{}】！銀號公櫃核銷印信，當場撥付 {} 枚銅幣入你懷中。",
            desc_owned, c_snap.amount_copper
        );
        push_breath(con, msg.clone());
        con.mark_mutated();
        return Ok(msg);
    }
    Ok("託付順利辦妥結清。".to_string())
}

/// 每響鐘檢查逾期或當事人謝幕之託管契：絕不無痕抹除，由銀號掛牌公告退還押金並留存違約事件！
pub fn settle_expired_escrow_pacts_as_world_events(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
    let due_ids: Vec<String> = delta
        .contracts
        .iter()
        .filter(|(_, c)| {
            if !is_escrow_pact_kind(&c.kind)
                || (c.status != "escrowed" && c.status != "in_progress")
            {
                return false;
            }
            let cred_dead = delta
                .actors
                .get(&c.creditor_id)
                .is_none_or(|iss| !iss.alive);
            let rec_dead = c.kind == "courier_errand" && {
                let (rec_id, _) = parse_pact_note(&c.note_zh);
                delta.actors.get(rec_id).is_none_or(|r| !r.alive)
            };
            clock.game_days > c.due_day || cred_dead || rec_dead
        })
        .map(|(k, _)| k.clone())
        .collect();

    for cid in due_ids {
        let Some(c_snap) = delta.contracts.get(&cid).cloned() else {
            continue;
        };
        let (meta_id, desc_zh) = parse_pact_note(&c_snap.note_zh);
        let meta_owned = meta_id.to_string();
        let desc_owned = desc_zh.to_string();

        let cred_alive = delta
            .actors
            .get(&c_snap.creditor_id)
            .is_some_and(|iss| iss.alive);
        let rec_alive = if c_snap.kind == "courier_errand" {
            delta.actors.get(&meta_owned).is_some_and(|r| r.alive)
        } else {
            true
        };
        let force_majeure = !cred_alive || !rec_alive;

        let refund_to = if cred_alive {
            c_snap.creditor_id.clone()
        } else {
            let hh = delta
                .actors
                .get(&c_snap.creditor_id)
                .map(|iss| iss.household_id.clone())
                .unwrap_or_default();
            delta
                .actors
                .values()
                .find(|x| x.alive && !hh.is_empty() && x.household_id == hh)
                .map_or_else(|| "municipal_treasury".to_string(), |x| x.actor_id.clone())
        };

        if c_snap.amount_copper > 0
            && !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "escrow_return",
                "contract_escrow",
                &refund_to,
                c_snap.amount_copper,
                format!("銀號公告期滿退還託管押金：{}", desc_owned),
            )
        {
            continue;
        }

        let had_contractor = c_snap.status == "in_progress" && !c_snap.debtor_id.is_empty();
        let is_culpable_breach =
            had_contractor && clock.game_days > c_snap.due_day && !force_majeure;

        if had_contractor {
            if let Some(deb) = delta.actors.get_mut(&c_snap.debtor_id) {
                deb.backpack
                    .retain(|i| i.bound_owner.as_deref() != Some(cid.as_str()));
                deb.inn_locker
                    .retain(|i| i.bound_owner.as_deref() != Some(cid.as_str()));
                for bv in deb.inn_lockers.values_mut() {
                    bv.retain(|i| i.bound_owner.as_deref() != Some(cid.as_str()));
                }
                deb.mark_mutated();
            }
            if is_culpable_breach {
                let deb_name = delta
                    .actors
                    .get(&c_snap.debtor_id)
                    .map_or("承接人".to_string(), |d| d.display_name.clone());
                let iss_info = delta
                    .actors
                    .get(&c_snap.creditor_id)
                    .map(|i| (i.display_name.clone(), i.home_node.clone(), i.soul_seed));
                if let Some((iss_name, iss_home, iss_seed)) = iss_info {
                    if let Some(iss) = delta.actors.get_mut(&c_snap.creditor_id) {
                        iss.mind.record_pact_or_trade_evidence(
                            &c_snap.debtor_id,
                            &deb_name,
                            clock.game_days,
                            false,
                            false,
                            "承接銀號託付逾期未歸，由銀號掛牌退押金記違約",
                        );
                        let inn = derive_innate_persona(iss_seed.max(1));
                        let _ = iss.mind.appraise_event(
                            &inn,
                            ConcernCause::PactBetrayed,
                            Some(&c_snap.debtor_id),
                            None,
                            48,
                            abs_bell,
                        );
                        iss.mark_mutated();
                    }
                    broadcast_scene_event(
                        doc,
                        delta,
                        clock,
                        &iss_home,
                        &c_snap.debtor_id,
                        &deb_name,
                        EventPublicity::PublicLoud,
                        Some("harbor_trade"),
                        66,
                        44,
                        ValueAxis::PactIntegrity,
                        "escrow_breach_notice",
                        &format!(
                            "銀號公櫃掛牌：{}承接{}之託付逾期未辦妥，押金已依約退還原主並記失信一筆",
                            deb_name, iss_name
                        ),
                    );
                }
            }
        }

        if let Some(c) = delta.contracts.get_mut(&cid) {
            c.amount_copper = 0;
            c.status = if is_culpable_breach {
                "breached".to_string()
            } else {
                "settled".to_string()
            };
            delta.dirty_contracts.insert(cid);
        }
    }
}

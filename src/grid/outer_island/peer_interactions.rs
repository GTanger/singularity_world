//! 銀鷗島「同場對等互動動詞（`玩家↔NPC / 玩家↔玩家 / NPC↔NPC`）」模組。
//!
//! 嚴格落實審辯室 Room `7eee7478-1493-4faf-b7e3-12de460402e3` 定案：
//! 1. 開口傳話：套用四族語言熟練度遮罩＋同場旁聽＋通曉該語言目擊者拆穿無見證誇口（玩家與 NPC 雙路徑共用）；
//! 2. 亮物談價交割：禁止強購任何真人旅人背囊，採「先取物、再轉帳、失敗立即原物歸還」原子序，過期要約回傳 `Ok` 確保 PG 落盤清除；
//! 3. 同場拔刀助戰：檢驗助戰者傷勢與戰術架勢削減（`foe_stagger`），區分元祖巨蛛與空巢易主岩蜥戰利品；
//! 4. 點到為止切磋：真人雙段邀約應戰（禁未同意消耗疲勞）、NPC 時窗冷卻、切磋統一歸單一情境鍵防刷武相。

use crate::combat::wordplate::TacticalSlot;

use super::clock_and_lang::{SolarClockSnapshot, mask_speech_by_proficiency};
use super::dialogue_llm::primary_racial_lang;
use super::mind::EventPublicity;
use super::movement::{add_carry_item, add_language_exposure, push_breath};
use super::persona::{ValueAxis, derive_innate_persona};
pub use super::phase_and_vessels::check_and_expose_unwitnessed_boast;
use super::phase_and_vessels::{
    PeerTradeOffer, appraise_item_fair_copper, evaluate_actor_bout,
    record_martial_victory_and_check_phase,
};
use super::sim_tick::broadcast_scene_event;
use super::types::{CarryItem, IslandWorldDelta, WorldStaticDoc};

/// 同場真人↔真人（或真人↔居民）直接傳話：套用四族語言熟練度遮罩＋同場旁聽＋無目擊吹牛拆穿！
pub fn apply_peer_direct_speech(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    speaker_key: &str,
    target_token: &str,
    raw_message: &str,
) -> Result<(String, Option<String>), String> {
    let sp_aid = delta
        .resolve_actor_id(speaker_key)
        .ok_or("找不到旅人行蹤")?;
    let tg_aid = delta
        .resolve_public_target(target_token)
        .or_else(|| delta.resolve_actor_id(target_token))
        .ok_or("眼前找不到這位同道。")?;
    if sp_aid == tg_aid {
        return Err("無須對自己喊話。".to_string());
    }
    let clean_msg: String = raw_message.trim().chars().take(80).collect();
    if clean_msg.is_empty() {
        return Err("請輸入想對他說的話。".to_string());
    }

    let (node_id, sp_name, sp_race, sp_langs) = {
        let sp = delta.actors.get(&sp_aid).ok_or("找不到說話者")?;
        let tg = delta.actors.get(&tg_aid).ok_or("對方已不在場")?;
        if !tg.alive || tg.node_id != sp.node_id {
            return Err("對方已轉過街角走遠了。".to_string());
        }
        (
            sp.node_id.clone(),
            sp.display_name.clone(),
            sp.race.clone(),
            sp.languages.clone(),
        )
    };

    let (lang_code, body_text) = if let Some(rest) = clean_msg.strip_prefix("〔矮人語〕")
        && sp_langs.get("dwarven").copied().unwrap_or(0) >= 50
    {
        ("dwarven", rest.trim())
    } else if let Some(rest) = clean_msg.strip_prefix("〔精靈語〕")
        && sp_langs.get("elvish").copied().unwrap_or(0) >= 50
    {
        ("elvish", rest.trim())
    } else if let Some(rest) = clean_msg.strip_prefix("〔獸人語〕")
        && sp_langs.get("orcish").copied().unwrap_or(0) >= 50
    {
        ("orcish", rest.trim())
    } else {
        ("harbor_trade", clean_msg.as_str())
    };
    if body_text.is_empty() {
        return Err("請輸入想對他說的話。".to_string());
    }

    let (tg_name, masked_for_target) = {
        let tg = delta.actors.get(&tg_aid).ok_or("對方已不在場")?;
        let prof = if lang_code == "harbor_trade" {
            tg.languages.get("harbor_trade").copied().unwrap_or(80)
        } else {
            tg.languages.get(lang_code).copied().unwrap_or(0)
        };
        let (masked, _) = if lang_code == "harbor_trade" && prof >= 60 {
            (format!("「{}」{}當面說道。", body_text, sp_name), true)
        } else {
            mask_speech_by_proficiency(&sp_name, lang_code, prof, body_text)
        };
        (tg.display_name.clone(), masked)
    };

    if let Some(tg) = delta.actors.get_mut(&tg_aid) {
        tg.mind.record_talk_exchange(
            &sp_aid,
            &sp_name,
            clock.game_days,
            &clock.bell_label,
            body_text,
            "（當面聆聽）",
            false,
        );
        push_breath(tg, masked_for_target.clone());
        if let Some(r_lang) = primary_racial_lang(&sp_race)
            && r_lang == lang_code
        {
            let _ = add_language_exposure(tg, clock.game_days, r_lang, 6);
        }
        tg.mark_mutated();
    }
    if let Some(sp) = delta.actors.get_mut(&sp_aid) {
        sp.mind.record_talk_exchange(
            &tg_aid,
            &tg_name,
            clock.game_days,
            &clock.bell_label,
            "（當面聆聽）",
            body_text,
            false,
        );
        push_breath(sp, format!("你對{}說：「{}」", tg_name, body_text));
        sp.mark_mutated();
    }

    broadcast_scene_event(
        doc,
        delta,
        clock,
        &node_id,
        &sp_aid,
        &sp_name,
        EventPublicity::TableWhisper,
        Some(lang_code),
        46,
        24,
        ValueAxis::KinshipLoyalty,
        "peer_speak",
        &format!("{}對{}低聲交談：「{}」", sp_name, tg_name, body_text),
    );

    let exposed = check_and_expose_unwitnessed_boast(
        doc, delta, clock, &sp_aid, &node_id, lang_code, body_text,
    );
    Ok((masked_for_target, exposed))
}

fn take_exact_clean_item(
    bag: &mut Vec<CarryItem>,
    slug: &str,
    uid: &str,
    allow_stolen_for_peer_offer: bool,
) -> Option<CarryItem> {
    let pos = bag.iter().position(|i| {
        i.slug == slug
            && i.count > 0
            && i.bound_owner.is_none()
            && (allow_stolen_for_peer_offer || i.stolen_from.is_none())
            && (uid.is_empty() || i.vessel_meta.as_ref().is_some_and(|m| m.uid == uid))
    })?;
    if bag[pos].count > 1 {
        bag[pos].count -= 1;
        let mut one = bag[pos].clone();
        one.count = 1;
        Some(one)
    } else {
        Some(bag.remove(pos))
    }
}

/// 同場亮物議價交割（嚴禁強購真人玩家背囊；採先取物、再轉帳、失敗立即原物歸還原子序！）
pub fn apply_peer_item_trade(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    mode: &str,
    param: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);

    match mode {
        "buy_from_peer" => {
            let (seller_tok, item_slug) =
                param.split_once('|').ok_or("請指明交易對象與器物名稱。")?;
            let seller_id = delta
                .resolve_public_target(seller_tok)
                .or_else(|| delta.resolve_actor_id(seller_tok))
                .ok_or("眼前找不到這位賣家。")?;
            if seller_id == aid {
                return Err("無須向自己購買器物。".to_string());
            }
            let (same_node, seller_is_human, item_snap, ask_price, seller_name) = {
                let buyer = delta.actors.get(&aid).ok_or("找不到買家")?;
                let seller = delta.actors.get(&seller_id).ok_or("找不到賣家")?;
                let is_human = seller.controller_id.is_some() || seller.actor_id.starts_with("pl:");
                if is_human {
                    return Err(
                        "對方是同道旅人，請等對方主動亮出器物報價後當面點頭交割。".to_string()
                    );
                }
                let it = seller
                    .backpack
                    .iter()
                    .find(|i| {
                        i.slug == item_slug
                            && i.count > 0
                            && i.bound_owner.is_none()
                            && i.stolen_from.is_none()
                    })
                    .cloned()
                    .ok_or("對方背囊裡已無這件可讓渡的乾淨物件。")?;
                let inn = derive_innate_persona(seller.soul_seed.max(1));
                let warmth = seller
                    .mind
                    .ltm
                    .social
                    .get(&aid)
                    .map_or(0, |s| i64::from(s.warmth));
                let fair = appraise_item_fair_copper(&it);
                let ask = if warmth >= 35 {
                    (fair - 1).max(2)
                } else if inn.concession_slope < -10 {
                    fair + 2
                } else {
                    fair + 1
                };
                (
                    buyer.node_id == seller.node_id && seller.alive,
                    is_human,
                    it,
                    ask,
                    seller.display_name.clone(),
                )
            };
            let _ = seller_is_human;
            if !same_node {
                return Err("需與對方站在同一處才能當面驗貨點錢。".to_string());
            }
            let uid = item_snap
                .vessel_meta
                .as_ref()
                .map_or("", |m| m.uid.as_str());
            let taken = delta
                .actors
                .get_mut(&seller_id)
                .and_then(|s| {
                    let t = take_exact_clean_item(&mut s.backpack, &item_snap.slug, uid, false);
                    if t.is_some() {
                        s.mark_mutated();
                    }
                    t
                })
                .ok_or("交割瞬間器物已不在對方囊中。")?;
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "peer_vessel_trade",
                &aid,
                &seller_id,
                ask_price,
                format!("當面購得【{}】", taken.name),
            ) {
                if let Some(s) = delta.actors.get_mut(&seller_id) {
                    add_carry_item(&mut s.backpack, taken);
                    s.mark_mutated();
                }
                return Err(format!(
                    "對方開價 {} 枚銅幣讓渡【{}】，你懷裡銅板還不夠。",
                    ask_price, item_snap.name
                ));
            }
            let iname = taken.name.clone();
            let buyer_name = delta
                .actors
                .get(&aid)
                .map_or("旅人".to_string(), |b| b.display_name.clone());
            let Some(b) = delta.actors.get_mut(&aid) else {
                return Err("交割未完成。".to_string());
            };
            add_carry_item(&mut b.backpack, taken);
            b.mind.record_pact_or_trade_evidence(
                &seller_id,
                &seller_name,
                clock.game_days,
                true,
                false,
                &format!("以 {} 枚銅幣當面購得【{}】", ask_price, iname),
            );
            let msg = format!(
                "你點出 {} 枚銅幣交予{}，當面驗收接過【{}】收進背囊，銀貨兩訖！",
                ask_price, seller_name, iname
            );
            push_breath(b, msg.clone());
            b.mark_mutated();
            if let Some(s) = delta.actors.get_mut(&seller_id) {
                s.mind.record_pact_or_trade_evidence(
                    &aid,
                    &buyer_name,
                    clock.game_days,
                    true,
                    false,
                    &format!("當面讓渡【{}】收得 {} 枚銅幣", iname, ask_price),
                );
                s.mark_mutated();
            }
            Ok(msg)
        }
        "offer_to_peer" => {
            let mut sp = param.split('|');
            let buyer_tok = sp.next().unwrap_or("");
            let item_slug = sp.next().unwrap_or("");
            let price: i64 = sp
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(10)
                .clamp(1, 500);
            let buyer_id = delta
                .resolve_public_target(buyer_tok)
                .or_else(|| delta.resolve_actor_id(buyer_tok))
                .ok_or("眼前找不到這位買家。")?;
            if buyer_id == aid {
                return Err("無須向自己報價。".to_string());
            }
            let (cur_node, seller_name, item_snap, buyer_is_npc) = {
                let s = delta.actors.get(&aid).ok_or("找不到賣家")?;
                let b = delta.actors.get(&buyer_id).ok_or("找不到買家")?;
                if !b.alive || b.node_id != s.node_id {
                    return Err("對方已不在眼前。".to_string());
                }
                let is_npc = b.controller_id.is_none() && !buyer_id.starts_with("pl:");
                let it = s
                    .backpack
                    .iter()
                    .find(|i| {
                        i.slug == item_slug
                            && i.count > 0
                            && i.bound_owner.is_none()
                            && (!is_npc || i.stolen_from.is_none())
                    })
                    .cloned()
                    .ok_or(if is_npc {
                        "街坊只收來歷乾淨且非記名借領之物件。"
                    } else {
                        "這件物品屬於記名借領或不在囊中，不可轉賣。"
                    })?;
                (s.node_id.clone(), s.display_name.clone(), it, is_npc)
            };

            if buyer_is_npc {
                let fair = appraise_item_fair_copper(&item_snap);
                let b_copper = delta.actors.get(&buyer_id).map_or(0, |b| b.copper);
                if price > fair + 2 {
                    return Err(format!(
                        "對方搖了搖頭：「這件【{}】在港裡頂多值 {} 枚銅幣，要價太高了。」",
                        item_snap.name, fair
                    ));
                }
                if b_copper < price + 8 {
                    return Err(
                        "對方摸了摸錢袋：「東西不錯，可惜我手頭還得留著買糧的銅板。」".to_string(),
                    );
                }
                let uid = item_snap
                    .vessel_meta
                    .as_ref()
                    .map_or("", |m| m.uid.as_str());
                let taken = delta
                    .actors
                    .get_mut(&aid)
                    .and_then(|s| {
                        let t = take_exact_clean_item(&mut s.backpack, &item_snap.slug, uid, false);
                        if t.is_some() {
                            s.mark_mutated();
                        }
                        t
                    })
                    .ok_or("找不到背囊乾淨物件。")?;
                if !delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "peer_vessel_trade",
                    &buyer_id,
                    &aid,
                    price,
                    format!("當面收購【{}】", taken.name),
                ) {
                    if let Some(s) = delta.actors.get_mut(&aid) {
                        add_carry_item(&mut s.backpack, taken);
                        s.mark_mutated();
                    }
                    return Err("對方銅幣不足。".to_string());
                }
                let buyer_name = delta
                    .actors
                    .get(&buyer_id)
                    .map_or("街坊".to_string(), |b| b.display_name.clone());
                if let Some(b) = delta.actors.get_mut(&buyer_id) {
                    add_carry_item(&mut b.backpack, taken);
                    b.mark_mutated();
                }
                let msg = format!(
                    "{}端詳過【{}】爽快點頭，當場數出 {} 枚銅幣與你銀貨兩訖！",
                    buyer_name, item_snap.name, price
                );
                if let Some(s) = delta.actors.get_mut(&aid) {
                    push_breath(s, msg.clone());
                    s.mark_mutated();
                }
                return Ok(msg);
            }

            let offer = PeerTradeOffer {
                seller_id: aid.clone(),
                seller_name: seller_name.clone(),
                item_slug: item_snap.slug.clone(),
                item_uid: item_snap
                    .vessel_meta
                    .as_ref()
                    .map_or_else(String::new, |m| m.uid.clone()),
                item_name: item_snap.name.clone(),
                price_copper: price,
                node_id: cur_node,
                created_bell: abs_bell,
            };
            if let Some(b) = delta.actors.get_mut(&buyer_id) {
                b.mind.stm.pending_trade_offer = Some(offer);
                push_breath(
                    b,
                    format!(
                        "{}向你亮出【{}】，開價 {} 枚銅幣問你是否當面點清接手。",
                        seller_name, item_snap.name, price
                    ),
                );
                b.mark_mutated();
            }
            Ok(format!(
                "已向對方亮出【{}】（開價 {} 枚銅幣），待對方當面點頭即可銀貨兩訖。",
                item_snap.name, price
            ))
        }
        "accept_peer_offer" => {
            let offer = delta
                .actors
                .get(&aid)
                .and_then(|b| b.mind.stm.pending_trade_offer.clone())
                .ok_or("目前沒有待你點頭確認的當面交易。")?;
            let expired = abs_bell.saturating_sub(offer.created_bell) > 6;
            let same_here = !expired
                && delta
                    .actors
                    .get(&aid)
                    .is_some_and(|b| b.node_id == offer.node_id)
                && delta
                    .actors
                    .get(&offer.seller_id)
                    .is_some_and(|s| s.alive && s.node_id == offer.node_id);
            if !same_here {
                if let Some(b) = delta.actors.get_mut(&aid) {
                    b.mind.stm.pending_trade_offer = None;
                    let note = "賣家已離開現場或口頭報價逾時作罷，未扣取任何銅幣。";
                    push_breath(b, note);
                    b.mark_mutated();
                }
                return Ok("賣家已離開現場或口頭報價逾時作罷，未扣取任何銅幣。".to_string());
            }
            let taken_opt = delta.actors.get_mut(&offer.seller_id).and_then(|s| {
                let t =
                    take_exact_clean_item(&mut s.backpack, &offer.item_slug, &offer.item_uid, true);
                if t.is_some() {
                    s.mark_mutated();
                }
                t
            });
            let Some(taken) = taken_opt else {
                if let Some(b) = delta.actors.get_mut(&aid) {
                    b.mind.stm.pending_trade_offer = None;
                    push_breath(b, "那件器物已不在對方囊中，報價作罷，未扣取銅幣。");
                    b.mark_mutated();
                }
                return Ok("那件器物剛才已不在對方囊中，報價作罷且未扣取任何銅幣。".to_string());
            };
            if !delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "peer_vessel_trade",
                &aid,
                &offer.seller_id,
                offer.price_copper,
                format!("當面點頭購得【{}】", offer.item_name),
            ) {
                if let Some(s) = delta.actors.get_mut(&offer.seller_id) {
                    add_carry_item(&mut s.backpack, taken);
                    s.mark_mutated();
                }
                return Err(format!(
                    "需要 {} 枚銅幣才能點清這筆交易。",
                    offer.price_copper
                ));
            }
            if let Some(s) = delta.actors.get_mut(&offer.seller_id) {
                push_breath(
                    s,
                    format!(
                        "對方點頭接過【{}】，當面付清 {} 枚銅幣。",
                        offer.item_name, offer.price_copper
                    ),
                );
                s.mark_mutated();
            }
            let b = delta.actors.get_mut(&aid).ok_or("找不到買家")?;
            b.mind.stm.pending_trade_offer = None;
            add_carry_item(&mut b.backpack, taken);
            let msg = format!(
                "你點頭交出 {} 枚銅幣，從{}手中接過【{}】，當場銀貨兩訖！",
                offer.price_copper, offer.seller_name, offer.item_name
            );
            push_breath(b, msg.clone());
            b.mark_mutated();
            Ok(msg)
        }
        _ => Err("未知的當面交易指令。".to_string()),
    }
}

/// 同節點拔刀助戰：檢驗助戰者傷勢與戰術架勢削減（`foe_stagger`），達破敵閾值始合擊破敵！
pub fn apply_peer_assist_skirmish(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    helper_key: &str,
    ally_token: &str,
) -> Result<String, String> {
    let helper_id = delta
        .resolve_actor_id(helper_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let ally_id = delta
        .resolve_public_target(ally_token)
        .or_else(|| delta.resolve_actor_id(ally_token))
        .ok_or("眼前找不到要相助的對象。")?;
    if helper_id == ally_id {
        return Err("你已在戰圈之中。".to_string());
    }

    let (node_id, sk, helper_name, ally_name, helper_wep, helper_power) = {
        let h = delta.actors.get(&helper_id).ok_or("找不到助戰者")?;
        let al = delta.actors.get(&ally_id).ok_or("找不到受助者")?;
        if !al.alive || al.node_id != h.node_id {
            return Err("對方已不在同一處。".to_string());
        }
        if h.wound_part.is_some() {
            return Err("你手臂帶傷未癒，強行切入戰圈只會雙雙遇險。".to_string());
        }
        if h.fatigue >= 80 {
            return Err("你已氣喘如牛，強行切入戰圈只會添亂。".to_string());
        }
        let Some(ref s) = al.active_skirmish else {
            return Err("對方眼前並無敵手交鋒。".to_string());
        };
        let pwr = h.bound_weapon.rune_mark.is_some()
            || h.mind.ltm.mastery.private_martial_phase.is_some()
            || h.bound_weapon.grip_familiarity >= 14;
        (
            h.node_id.clone(),
            s.clone(),
            h.display_name.clone(),
            al.display_name.clone(),
            h.bound_weapon.name.clone(),
            pwr,
        )
    };

    let is_apex = super::cell_resources::is_apex_beast_slug(&sk.foe_slug);
    let is_nest_beast = matches!(sk.foe_slug.as_str(), "stone_spider" | "cave_lizard");
    let win_threshold = u8::from((is_nest_beast || is_apex) && !helper_power) + 1;
    let next_stagger = sk.foe_stagger + if helper_power { 2 } else { 1 };

    if let Some(h) = delta.actors.get_mut(&helper_id) {
        h.fatigue = (h.fatigue + 9).min(95);
        h.bound_weapon.grip_familiarity = h.bound_weapon.grip_familiarity.saturating_add(3);
        h.mind.record_pact_or_trade_evidence(
            &ally_id,
            &ally_name,
            clock.game_days,
            true,
            true,
            &format!("於危急時拔刀助{}迎擊{}", ally_name, sk.foe_name),
        );
        h.mark_mutated();
    }

    if next_stagger < win_threshold {
        if let Some(al) = delta.actors.get_mut(&ally_id) {
            if let Some(ref mut s) = al.active_skirmish {
                s.foe_stagger = next_stagger;
                s.player_guard = true;
            }
            push_breath(
                al,
                format!(
                    "{}揮【{}】自側翼猛擊牽制【{}】，替你穩住守勢並砸亂凶獸重心！",
                    helper_name, helper_wep, sk.foe_name
                ),
            );
            al.mark_mutated();
        }
        let msg = format!(
            "你踏步揮【{}】震偏【{}】前撲之勢，使其重心大晃——再補一擊即可合擊破敵！",
            helper_wep, sk.foe_name
        );
        if let Some(h) = delta.actors.get_mut(&helper_id) {
            push_breath(h, msg.clone());
            h.mark_mutated();
        }
        return Ok(msg);
    }

    let is_boss = sk.foe_slug == "stone_spider" && delta.first_spider_slain.is_none();
    if let Some(al) = delta.actors.get_mut(&ally_id) {
        al.active_skirmish = None;
        al.bound_weapon.grip_familiarity = al.bound_weapon.grip_familiarity.saturating_add(3);
        if is_nest_beast {
            let (l_slug, l_name, l_w) = if is_boss {
                ("spider_carapace", "吞石巨蛛厚甲殼", 3u16)
            } else {
                ("lizard_scale", "嗜石岩蜥厚鱗皮", 2u16)
            };
            add_carry_item(
                &mut al.backpack,
                CarryItem {
                    slug: l_slug.to_string(),
                    name: l_name.to_string(),
                    count: 1,
                    bound_owner: None,
                    stolen_from: None,
                    weight_units: l_w,
                    vessel_meta: None,
                    gear_meta: None,
                    manual_meta: None,
                },
            );
        }
        al.mind.record_pact_or_trade_evidence(
            &helper_id,
            &helper_name,
            clock.game_days,
            true,
            true,
            &format!("遇險時蒙{}仗義拔刀合擊解圍", helper_name),
        );
        push_breath(
            al,
            format!(
                "{}從旁仗義出手，揮【{}】與你合擊逼退了【{}】！",
                helper_name, helper_wep, sk.foe_name
            ),
        );
        al.mark_mutated();
    }
    super::living_chronicler::record_crisis_rescue(
        doc,
        delta,
        clock,
        &helper_id,
        &ally_id,
        &node_id,
        &helper_wep,
        &sk.foe_name,
    );

    if is_nest_beast {
        let vac_days = if is_boss { 8 } else { 6 };
        delta
            .cleared_nests_until_day
            .insert(node_id.clone(), clock.game_days + vac_days);
        if is_boss {
            delta.first_spider_slain = Some((
                ally_id.clone(),
                format!("{}與{}", ally_name, helper_name),
                clock.game_days,
            ));
            super::living_chronicler::record_primordial_spider_slain(
                doc,
                delta,
                clock,
                &ally_id,
                Some(&helper_id),
                &node_id,
                &helper_wep,
            );
        }
        let (ev_key, ev_zh) = if is_boss {
            (
                "slay_stone_spider",
                format!("{}與{}聯手擊潰吞石巨蛛並撬下厚甲殼", ally_name, helper_name),
            )
        } else {
            (
                "repel_cave_lizard",
                format!(
                    "{}與{}聯手擊退空巢易主的{}並剝下厚鱗皮",
                    ally_name, helper_name, sk.foe_name
                ),
            )
        };
        broadcast_scene_event(
            doc,
            delta,
            clock,
            &node_id,
            &ally_id,
            &ally_name,
            EventPublicity::PublicLoud,
            None,
            88,
            65,
            ValueAxis::CraftPride,
            ev_key,
            &ev_zh,
        );
    } else if is_apex {
        if let Some(m) = super::cell_resources::record_apex_beast_slain(
            doc,
            delta,
            clock,
            &ally_id,
            &node_id,
            &sk.foe_name,
        ) && let Some(al) = delta.actors.get_mut(&ally_id)
        {
            push_breath(al, m);
            al.mark_mutated();
        }
        let sum = format!(
            "{}拔刀助{}合擊誅除盤踞百年的太古巨物【{}】",
            helper_name, ally_name, sk.foe_name
        );
        broadcast_scene_event(
            doc,
            delta,
            clock,
            &node_id,
            &helper_id,
            &helper_name,
            EventPublicity::PublicLoud,
            None,
            92,
            72,
            ValueAxis::CraftPride,
            "slay_apex_beast",
            &sum,
        );
    } else {
        let _ = delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "goblin_loot",
            "municipal_treasury",
            &ally_id,
            3,
            format!("合擊擊退{}奪回散落銅幣", sk.foe_name),
        );
        broadcast_scene_event(
            doc,
            delta,
            clock,
            &node_id,
            &helper_id,
            &helper_name,
            EventPublicity::PublicLoud,
            None,
            66,
            45,
            ValueAxis::KinshipLoyalty,
            "repel_goblin",
            &format!(
                "{}拔刀助{}合擊擊退【{}】",
                helper_name, ally_name, sk.foe_name
            ),
        );
    }

    let _ = record_martial_victory_and_check_phase(
        doc,
        delta,
        clock,
        &helper_id,
        &sk.foe_slug,
        &sk.foe_name,
        &node_id,
        is_boss,
    );
    let _ = record_martial_victory_and_check_phase(
        doc,
        delta,
        clock,
        &ally_id,
        &sk.foe_slug,
        &sk.foe_name,
        &node_id,
        is_boss,
    );

    let msg = format!(
        "你踏步切入戰圈揮動【{}】側翼猛擊，與{}聯手將【{}】震退潰逃！",
        helper_wep, ally_name, sk.foe_name
    );
    if let Some(h) = delta.actors.get_mut(&helper_id) {
        push_breath(h, msg.clone());
        h.mark_mutated();
    }
    Ok(msg)
}

/// 講定「點到為止」切磋：真人雙段邀約應戰（禁未同意強制扣疲勞）、NPC 時窗冷卻、切磋統一歸單一情境鍵防刷武相！
pub fn apply_peer_consensual_spar(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    challenger_key: &str,
    partner_token: &str,
) -> Result<String, String> {
    let ch_id = delta
        .resolve_actor_id(challenger_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let pa_id = delta
        .resolve_public_target(partner_token)
        .or_else(|| delta.resolve_actor_id(partner_token))
        .ok_or("眼前找不到切磋對象。")?;
    if ch_id == pa_id {
        return Err("無法與自己對拆招式。".to_string());
    }
    let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
    let spar_win_key = format!("spar:{}@{}", pa_id, clock.six_phase_key);

    let (
        node_id,
        pa_is_human,
        already_invited_by_pa,
        a_wins,
        bout_note,
        ch_name,
        pa_name,
        ch_wep,
        pa_wep,
    ) = {
        let ch = delta.actors.get(&ch_id).ok_or("找不到發起人")?;
        let pa = delta.actors.get(&pa_id).ok_or("找不到對手")?;
        if !pa.alive || pa.node_id != ch.node_id {
            return Err("對方已不在同一處。".to_string());
        }
        if ch.fatigue >= 78 || ch.wound_part.is_some() {
            return Err("你身上帶傷或氣力不濟，先調養妥當再與人試手吧。".to_string());
        }
        if pa.fatigue >= 78 || pa.wound_part.is_some() || pa.active_skirmish.is_some() {
            return Err("對方此刻氣息未勻或另有要事，暫不便拆招試手。".to_string());
        }
        let is_human = pa.controller_id.is_some() || pa_id.starts_with("pl:");
        if !is_human && ch.explored_windows.contains(&spar_win_key) {
            return Err("這個時段剛與對方拆過招，先歇口氣琢磨勁道再試手吧。".to_string());
        }
        let invited = ch
            .mind
            .stm
            .pending_spar_from
            .as_ref()
            .is_some_and(|(from_id, nid, cb)| {
                from_id == &pa_id && nid == &ch.node_id && abs_bell.saturating_sub(*cb) <= 6
            });
        let (win, note) = evaluate_actor_bout(ch, pa, TacticalSlot::Midgame, 1);
        (
            ch.node_id.clone(),
            is_human,
            invited,
            win,
            note,
            ch.display_name.clone(),
            pa.display_name.clone(),
            ch.bound_weapon.name.clone(),
            pa.bound_weapon.name.clone(),
        )
    };

    if pa_is_human && !already_invited_by_pa {
        if let Some(pa) = delta.actors.get_mut(&pa_id) {
            pa.mind.stm.pending_spar_from = Some((ch_id.clone(), node_id, abs_bell));
            push_breath(
                pa,
                format!(
                    "{}向你拱手邀約「點到為止」拆招試手，可點開交談抽屜應戰。",
                    ch_name
                ),
            );
            pa.mark_mutated();
        }
        let msg = format!(
            "你向{}拱手邀約點到為止切磋，待對方當面點頭應戰即可過招。",
            pa_name
        );
        if let Some(ch) = delta.actors.get_mut(&ch_id) {
            push_breath(ch, msg.clone());
            ch.mark_mutated();
        }
        return Ok(msg);
    }

    if let Some(ch) = delta.actors.get_mut(&ch_id) {
        ch.mind.stm.pending_spar_from = None;
        if !pa_is_human {
            ch.explored_windows.insert(spar_win_key);
        }
        ch.fatigue = (ch.fatigue + 10).min(95);
        ch.bound_weapon.grip_familiarity = ch.bound_weapon.grip_familiarity.saturating_add(3);
        ch.mark_mutated();
    }
    if let Some(pa) = delta.actors.get_mut(&pa_id) {
        pa.mind.stm.pending_spar_from = None;
        pa.fatigue = (pa.fatigue + 10).min(95);
        pa.bound_weapon.grip_familiarity = pa.bound_weapon.grip_familiarity.saturating_add(3);
        pa.mark_mutated();
    }

    let victor_id = if a_wins { &ch_id } else { &pa_id };
    let _ = record_martial_victory_and_check_phase(
        doc,
        delta,
        clock,
        victor_id,
        "consensual_spar",
        "同道切磋對手",
        &node_id,
        false,
    );

    let outcome_clause = if a_wins {
        format!(
            "你以【{}】切入空隙略勝半籌，在觸及{}手腕前穩穩收勢",
            ch_wep, pa_name
        )
    } else {
        format!(
            "{}持【{}】沉穩拆解你的來勢，反手點到即止略勝半招",
            pa_name, pa_wep
        )
    };
    let msg = format!(
        "雙方講定點到為止後拉開架勢過招（{}）——{}，彼此拱手致意！",
        bout_note, outcome_clause
    );
    if let Some(ch) = delta.actors.get_mut(&ch_id) {
        push_breath(ch, msg.clone());
        ch.mark_mutated();
    }
    if let Some(pa) = delta.actors.get_mut(&pa_id) {
        push_breath(
            pa,
            format!(
                "{}與你講定點到為止拆了數招（{}），收手拱手致意。",
                ch_name, bout_note
            ),
        );
        pa.mark_mutated();
    }
    Ok(msg)
}

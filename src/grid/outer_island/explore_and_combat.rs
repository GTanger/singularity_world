//! 全服單一 `UnifiedMindBody` 防刷探索、隱藏坑道發現、戰術過招與「損本趟、留真疤、保根基」落敗結算。

use super::clock_and_lang::SolarClockSnapshot;
use super::mind::{EventPublicity, EvidenceTier};
use super::movement::{add_carry_item, add_language_exposure, push_breath, take_carry_item};
use super::persona::ValueAxis;
use super::sim_tick::broadcast_scene_event;
use super::types::{CarryItem, DroppedLootBag, IslandWorldDelta, SkirmishState, WorldStaticDoc};

pub fn apply_explore(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
) -> Result<String, String> {
    let Some(aid) = delta.resolve_actor_id(actor_key) else {
        return Err("請先向港務官核對通行身分。".to_string());
    };
    let (node_id, has_rumor, already_unlocked) = {
        let a = delta.actors.get(&aid).ok_or("請先向港務官核對通行身分。")?;
        if !a.setup_done {
            return Err("請先在港務登岸木冊留下你的名號與來歷。".to_string());
        }
        if a.active_skirmish.is_some() {
            return Err("眼前正與敵手交鋒，無暇分心翻找四周！".to_string());
        }
        if a.active_transit.is_some() {
            return Err("篷車正在大路上疾馳，無法彎腰翻找路邊。".to_string());
        }
        (
            a.node_id.clone(),
            a.known_rumors.contains("pit_rumor"),
            a.unlocked_secrets.contains(&a.node_id),
        )
    };

    let Some(node) = doc.nodes.get(&node_id) else {
        return Err("四周霧氣迷濛，暫時辨不清方位。".to_string());
    };

    let win_key = format!("{}@{}", node_id, clock.six_phase_key);
    if delta
        .actors
        .get(&aid)
        .is_some_and(|a| a.explored_windows.contains(&win_key))
        && !(doc.hidden_entrances.contains_key(&node_id) && has_rumor && !already_unlocked)
    {
        return Ok(format!(
            "這個時段（{}）你已仔細搜巡過【{}】四周，暫無新的蛛絲馬跡；不妨換個方向走走或等下個鐘響再看。",
            clock.bell_label, node.name
        ));
    }

    if let Some(hid) = doc.hidden_entrances.get(&node_id)
        && !already_unlocked
    {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        a.mark_mutated();
        if has_rumor {
            a.unlocked_secrets.insert(node_id);
            a.mind.ltm.known_nodes.insert(hid.target.clone());
            push_breath(a, hid.found_text.clone());
            return Ok(hid.found_text.clone());
        }
        push_breath(a, hid.clue_text.clone());
        return Ok(hid.clue_text.clone());
    }

    if delta
        .dropped_loot_by_node
        .get(&node_id)
        .is_some_and(|v| !v.is_empty())
    {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        a.explored_windows.insert(win_key);
        a.mark_mutated();
        let msg = "你在草叢石縫間發現一隻沾泥的遺落行囊，隨時可彎腰拾起清點！".to_string();
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if node_id == "oi:dng_pit:stele" {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "stele_rubbing".to_string(),
                name: "矮人古碑拓片".to_string(),
                count: 1,
                bound_owner: None,
                stolen_from: None,
                weight_units: 1,
            },
        );
        let lang_note =
            add_language_exposure(a, clock.game_days, "dwarven", 15).unwrap_or_default();
        a.mark_mutated();
        let msg = format!(
            "你用炭條與麻紙將青灰石碑上的矮人古銘文拓印下來，收進背囊（獲得【矮人古碑拓片】）。{}",
            lang_note
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if node_id == "oi:dng_pit:side" {
        let can_coin = delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "pit_salvage",
            "municipal_treasury",
            &aid,
            6,
            "試煉坑木架下拾獲舊銅幣".to_string(),
        );
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        a.mark_mutated();
        let msg = if can_coin {
            "你在腐朽木架底下翻出 6 枚散落的舊銅幣，吹去灰土揣入懷中。".to_string()
        } else {
            "你在腐朽木架下只翻出幾片鏽蝕鐵屑，值錢銅板早已被搜刮一空。".to_string()
        };
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if node_id == "oi:dng_pit:lair" {
        let cleared_until = delta
            .cleared_nests_until_day
            .get(&node_id)
            .copied()
            .unwrap_or(-1);
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        a.mark_mutated();
        if clock.game_days < cleared_until {
            let msg = "巨蛛巢穴裡只剩斷裂的灰白蛛網，那頭吞石巨蛛暫時尚未重返巢中。".to_string();
            push_breath(a, msg.clone());
            return Ok(msg);
        }
        a.active_skirmish = Some(SkirmishState {
            foe_slug: "stone_spider".to_string(),
            foe_name: "背嵌原石的吞石巨蛛".to_string(),
            can_speak_goblin: false,
            foe_stagger: 0,
            player_guard: false,
            round_idx: 1,
        });
        let msg = "灰白蛛網劇烈震顫，一頭甲殼厚如鐵板的吞石巨蛛嘶鳴著撲出，前螯直取你的手腕！"
            .to_string();
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if node_id == "oi:dng_pit:altar" {
        let cleared_spider = delta
            .cleared_nests_until_day
            .get("oi:dng_pit:lair")
            .copied()
            .unwrap_or(-1);
        if clock.game_days >= cleared_spider {
            return Err("外頭巢穴的吞石巨蛛仍在虎視眈眈，不敢貿然鑿石驚動牠！".to_string());
        }
        let next_avail = delta
            .harvested_altars_until_day
            .get(&node_id)
            .copied()
            .unwrap_or(-1);
        if clock.game_days < next_avail {
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let msg = "石台岩縫間新凝的原石尚小，還需過幾日才會成形。".to_string();
            push_breath(a, msg.clone());
            return Ok(msg);
        }
        delta
            .harvested_altars_until_day
            .insert(node_id, clock.game_days + 3);
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "rune_stone".to_string(),
                name: "古符文原石".to_string(),
                count: 1,
                bound_owner: None,
                stolen_from: None,
                weight_units: 2,
            },
        );
        a.mark_mutated();
        let msg = "你從石台岩縫間小心撬下一塊溫潤發亮的【古符文原石】！帶回巴爾鍛鐵鋪可鑲刻進隨身兵刃，或賣得好價錢。"
            .to_string();
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    let cell = doc.cells.get(&node.cell_id.to_string());
    let is_coast = cell.is_some_and(|c| c.is_coast);
    let height = cell.map_or(20, |c| c.height);

    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
    a.explored_windows.insert(win_key);
    a.last_active_unix = clock.real_unix;
    a.mark_mutated();

    if node.kind != "wild" {
        let msg = format!(
            "你留心觀察【{}】四周往來行色：門廊木牌字跡清晰，地面石板被車馬磨得光滑。",
            node.name
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    let roll = ((u64::from(node.cell_id) * 31)
        .wrapping_add(clock.game_days.unsigned_abs() * 17)
        .wrapping_add(u64::from(clock.hour)))
        % 4;

    if roll == 0 && !is_coast {
        let can_gob = a.languages.get("goblin").copied().unwrap_or(0) >= 25;
        let boss_prefix = delta
            .dropped_loot_by_node
            .get(&node_id)
            .and_then(|v| v.last())
            .and_then(|b| b.promoted_boss_name.clone());
        let foe_name = boss_prefix.unwrap_or_else(|| "灰耳哥布林斥候".to_string());
        a.active_skirmish = Some(SkirmishState {
            foe_slug: "grey_ear_goblin".to_string(),
            foe_name: foe_name.clone(),
            can_speak_goblin: can_gob,
            foe_stagger: 0,
            player_guard: false,
            round_idx: 1,
        });
        let msg = format!(
            "灌木叢驟然分開，一名手持骨柄短刀的【{}】伏低身子竄出，緊盯著你腰間行囊！",
            foe_name
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if is_coast {
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "driftwood".to_string(),
                name: "異國刻紋漂流木".to_string(),
                count: 1,
                bound_owner: None,
                stolen_from: None,
                weight_units: 2,
            },
        );
        let msg = "你在潮線碎石間拾獲一截堅實的【異國刻紋漂流木】，帶回港邊木匠或碼頭商販處可換幾枚銅幣。"
            .to_string();
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if height >= 23 {
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "iron_ore".to_string(),
                name: "粗鐵礦石".to_string(),
                count: 1,
                bound_owner: None,
                stolen_from: None,
                weight_units: 3,
            },
        );
        let msg =
            "你在裸露岩縫間敲下一塊沉甸甸的【粗鐵礦石】，鐵匠鋪長年收購這類礦料。".to_string();
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    add_carry_item(
        &mut a.backpack,
        CarryItem {
            slug: "herb_bundle".to_string(),
            name: "山林止血草束".to_string(),
            count: 1,
            bound_owner: None,
            stolen_from: None,
            weight_units: 1,
        },
    );
    let msg = "你在背陰老樹根旁採到一束鮮嫩的【山林止血草束】，草藥堂很樂意收購。".to_string();
    push_breath(a, msg.clone());
    Ok(msg)
}

pub fn resolve_skirmish_turn(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    action: &str,
) -> Result<String, String> {
    let aid = delta.resolve_actor_id(actor_key).ok_or("找不到旅人行蹤")?;
    let (node_id, sk, has_rune, fam, has_bread, gob_prof, fatigue, wound_exists) = {
        let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
        let Some(ref s) = a.active_skirmish else {
            return Err("目前並未處於交鋒對峙之中。".to_string());
        };
        (
            a.node_id.clone(),
            s.clone(),
            a.bound_weapon.rune_mark.is_some(),
            a.bound_weapon.grip_familiarity,
            a.backpack
                .iter()
                .any(|i| i.slug == "rye_bread" && i.count > 0),
            a.languages.get("goblin").copied().unwrap_or(0),
            a.fatigue,
            a.wound_part.is_some(),
        )
    };

    match action {
        "skirmish_retreat" => {
            let retreat_dst = {
                let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤")?;
                doc.nodes.get(&node_id).and_then(|cur_node| {
                    cur_node
                        .exits
                        .iter()
                        .find(|e| e.kind == "secret_out" || a.mind.ltm.known_nodes.contains(&e.to))
                        .or_else(|| cur_node.exits.first())
                        .map(|ex| ex.to.clone())
                })
            };
            if let Some(dst_id) = retreat_dst
                && let Some(dst_nd) = doc.nodes.get(&dst_id)
            {
                delta.relocate_actor(&aid, &dst_id, dst_nd.cell_id, false);
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.active_skirmish = None;
            a.fatigue = (a.fatigue + 8).min(95);
            a.mark_mutated();
            let msg = format!(
                "你虛晃一招逼退【{}】，借著地形掩護迅速抽身退開，保住了隨身行囊。",
                sk.foe_name
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "skirmish_parley" => {
            if sk.foe_slug != "grey_ear_goblin" {
                return Err("眼前異獸不通言語，無法喝止交涉！".to_string());
            }
            if gob_prof < 25 {
                return Err(
                    "你還聽不懂哥布林部族語，胡亂叫嚷只會激怒對方（可向碼頭獸人苦力請教幾句）。"
                        .to_string(),
                );
            }
            if !has_bread {
                return Err("背囊裡沒有黑麥乾糧可拋給對方換路。".to_string());
            }
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            let _ = take_carry_item(&mut a.backpack, "rye_bread", 1);
            a.active_skirmish = None;
            let _ = add_language_exposure(a, clock.game_days, "goblin", 8);
            a.mark_mutated();
            let msg = format!(
                "你拋出一塊黑麥乾糧，以剛學會的短促喉音喝道「停手、拿去吃」！【{}】鼻子一聳抓起乾糧，吱吱叫著鑽回灌木叢讓開了去路。",
                sk.foe_name
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "skirmish_guard" => {
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.fatigue = (a.fatigue + 6).min(95);
            if let Some(ref mut s) = a.active_skirmish {
                s.player_guard = true;
                s.round_idx = s.round_idx.saturating_add(1);
            }
            a.mark_mutated();
            let msg = format!(
                "你沉肩橫握【{}】護住手腕要害，硬生生架開【{}】撲來的一擊，雙臂微震但穩住了陣腳！",
                a.bound_weapon.name, sk.foe_name
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        "skirmish_wrist" | "skirmish_heavy" => {
            let is_spider = sk.foe_slug == "stone_spider";
            let reckless_defeat = action == "skirmish_heavy"
                && !sk.player_guard
                && ((is_spider && !has_rune && (fatigue >= 65 || wound_exists))
                    || (fatigue >= 85 && wound_exists));

            if reckless_defeat {
                return trigger_non_lethal_defeat(delta, clock, &aid, &node_id, &sk.foe_name);
            }

            let win_threshold = if is_spider && !has_rune { 2 } else { 1 };
            let next_stagger = if action == "skirmish_wrist" {
                sk.foe_stagger + 1
            } else if has_rune || fam >= 14 || sk.player_guard {
                sk.foe_stagger + 2
            } else {
                sk.foe_stagger + 1
            };

            if next_stagger >= win_threshold {
                if is_spider {
                    delta
                        .cleared_nests_until_day
                        .insert(node_id.clone(), clock.game_days + 3);
                }
                {
                    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                    a.active_skirmish = None;
                    a.fatigue = (a.fatigue + 10).min(95);
                    a.bound_weapon.grip_familiarity =
                        a.bound_weapon.grip_familiarity.saturating_add(3);
                    if a.bound_weapon.wear_stage < 2 && !has_rune {
                        a.bound_weapon.wear_stage += 1;
                    }
                    a.mind.stm.today_goal_progressed = true;
                    a.mark_mutated();
                }

                if is_spider {
                    let (pname, wname) = {
                        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                        add_carry_item(
                            &mut a.backpack,
                            CarryItem {
                                slug: "spider_carapace".to_string(),
                                name: "吞石巨蛛厚甲殼".to_string(),
                                count: 1,
                                bound_owner: None,
                                stolen_from: None,
                                weight_units: 3,
                            },
                        );
                        (a.display_name.clone(), a.bound_weapon.name.clone())
                    };
                    broadcast_scene_event(
                        doc,
                        delta,
                        clock,
                        &node_id,
                        &aid,
                        &pname,
                        EventPublicity::PublicLoud,
                        None,
                        88,
                        65,
                        ValueAxis::CraftPride,
                        "slay_stone_spider",
                        &format!("{}在灰松試煉坑底擊潰吞石巨蛛並撬下厚甲殼", pname),
                    );
                    let msg = format!(
                        "你趁巨蛛前螯彈開的空檔踏步猛擊關節，【{}】應聲碎裂重甲！巨蛛嘶鳴潰逃入岩縫，地上留下一片值錢的【吞石巨蛛厚甲殼】，南面通往地脈凝石台的道路已暢行無阻！",
                        wname
                    );
                    if let Some(a) = delta.actors.get_mut(&aid) {
                        push_breath(a, msg.clone());
                    }
                    return Ok(msg);
                }

                let got_bounty = delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "goblin_loot",
                    "municipal_treasury",
                    &aid,
                    3,
                    format!("擊退{}奪回散落銅幣", sk.foe_name),
                );
                let pname = delta
                    .actors
                    .get(&aid)
                    .map_or("旅人", |a| a.display_name.as_str())
                    .to_string();
                broadcast_scene_event(
                    doc,
                    delta,
                    clock,
                    &node_id,
                    &aid,
                    &pname,
                    EventPublicity::PublicLoud,
                    None,
                    64,
                    45,
                    ValueAxis::SurvivalSafety,
                    "repel_goblin",
                    &format!("{}在山徑親手擊退【{}】", pname, sk.foe_name),
                );
                let coin_clause = if got_bounty {
                    "，地上還滾落對方搜刮來的 3 枚銅幣被你拾起"
                } else {
                    ""
                };
                let msg = format!(
                    "你瞅準空檔精準敲中【{}】持刃手腕，對方吃痛棄戰竄入密林{}！",
                    sk.foe_name, coin_clause
                );
                if let Some(a) = delta.actors.get_mut(&aid) {
                    push_breath(a, msg.clone());
                }
                return Ok(msg);
            }

            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.fatigue = (a.fatigue + 12).min(95);
            if !sk.player_guard && is_spider {
                a.wound_part = Some("左腕瘀腫".to_string());
            }
            if let Some(ref mut s) = a.active_skirmish {
                s.foe_stagger = next_stagger;
                s.player_guard = false;
                s.round_idx = s.round_idx.saturating_add(1);
            }
            a.mark_mutated();
            let msg = format!(
                "你的【{}】重重砸在【{}】甲殼上濺起火星，震得虎口發麻，但已逼得對方連退半步——再補一記精準打擊即可破敵！",
                a.bound_weapon.name, sk.foe_name
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        _ => Err("這一式尚未使順，先穩住架勢再出招。".to_string()),
    }
}

pub fn trigger_non_lethal_defeat(
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    node_id: &str,
    foe_name: &str,
) -> Result<String, String> {
    let aid = delta.resolve_actor_id(actor_key).ok_or("找不到旅人行蹤")?;

    let (wake_node, dropped_copper, dropped_items, wname) = {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        a.active_skirmish = None;
        let dc = a.copper;
        a.copper = 0;

        let mut kept = Vec::new();
        let mut dropped = Vec::new();
        for it in a.backpack.drain(..) {
            if it.bound_owner.is_some() {
                kept.push(it);
            } else {
                dropped.push(it);
            }
        }
        a.backpack = kept;
        a.bound_weapon.wear_stage = (a.bound_weapon.wear_stage + 1).min(3);
        a.wound_part = Some("左腕重挫瘀腫".to_string());
        a.severe_wrist_warning = true;
        a.fatigue = 65;

        let wake = if a.rescue_pact_active {
            a.rescue_pact_active = false;
            "oi:white_gull_harbor:herb_hall"
        } else {
            "oi:white_gull_harbor:rusty_anchor_tavern"
        };
        let pname = a.display_name.clone();
        let langs = a.languages.clone();
        let _ = a.mind.ingest_sensory_event(
            &langs,
            EventPublicity::TableWhisper,
            None,
            true,
            &format!(
                "defeat:{}:{}:{}:{}",
                aid, clock.game_days, node_id, foe_name
            ),
            clock.game_days,
            clock.bell_index,
            node_id,
            &aid,
            &pname,
            EvidenceTier::DirectWitness,
            85,
            85,
            ValueAxis::SurvivalSafety,
            &format!("於{}力竭落敗於{}之手，左腕重挫昏厥", node_id, foe_name),
        );
        a.mark_mutated();
        (wake, dc, dropped, a.bound_weapon.name.clone())
    };

    delta.relocate_actor(&aid, wake_node, 4616, true);

    if dropped_copper > 0 {
        delta.push_ledger_entry(
            clock.game_days,
            clock.bell_index,
            "defeat_drop_escrow",
            &aid,
            "dropped_loot_escrow",
            dropped_copper,
            format!("於{}落敗散落隨身銅幣", node_id),
        );
    }

    if dropped_copper > 0 || !dropped_items.is_empty() {
        let bags = delta
            .dropped_loot_by_node
            .entry(node_id.to_string())
            .or_default();
        if let Some(own_bag) = bags.iter_mut().find(|b| b.owner_id == aid) {
            own_bag.copper += dropped_copper;
            for it in dropped_items {
                add_carry_item(&mut own_bag.items, it);
            }
        } else {
            bags.push(DroppedLootBag {
                owner_id: aid.clone(),
                copper: dropped_copper,
                items: dropped_items,
                promoted_boss_name: Some(format!("奪囊的{}", foe_name)),
            });
        }
    }

    let msg = format!(
        "你在力竭之下被【{}】震開架勢昏厥倒地！幸得過路獵戶將你拖回城裡甦醒——手裡仍死死攥著【{}】，二樓宿房木箱裡的存貨也安然無恙，但懷中 {} 枚銅幣與隨身散貨遺落在了原處，左腕留下一道重挫瘀腫，敷藥前切莫再逞強硬撼。",
        foe_name, wname, dropped_copper
    );
    if let Some(a) = delta.actors.get_mut(&aid) {
        push_breath(a, msg.clone());
    }
    Ok(msg)
}

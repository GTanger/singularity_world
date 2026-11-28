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
        let req_rumor = doc
            .hidden_entrances
            .get(&a.node_id)
            .map_or("", |h| h.rumor_key.as_str());
        (
            a.node_id.clone(),
            req_rumor.is_empty() || a.known_rumors.contains(req_rumor),
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
        super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        a.mark_mutated();
        if has_rumor {
            super::mesh_expedition::unlock_bidirectional_secret(doc, a, &node_id);
            push_breath(a, hid.found_text.clone());
            return Ok(hid.found_text.clone());
        }
        push_breath(a, hid.clue_text.clone());
        return Ok(hid.clue_text.clone());
    }

    if let Some(mb) = delta.live_wilds.mutated_boss_by_node.get(&node_id).cloned() {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        a.active_skirmish = Some(SkirmishState {
            foe_slug: format!("mutated_boss:{node_id}"),
            foe_name: mb.boss_name_zh.clone(),
            can_speak_goblin: false,
            foe_stagger: 0,
            player_guard: false,
            round_idx: 1,
        });
        a.mark_mutated();
        let msg = format!(
            "碎石陡然震顫，剛吞下古符文原石而體軀膨脹的【{}】伏低重心朝你猛撲過來！",
            mb.boss_name_zh
        );
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if delta
        .dropped_loot_by_node
        .get(&node_id)
        .is_some_and(|v| !v.is_empty())
    {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(win_key);
        a.mark_mutated();
        let msg = "你在草叢石縫間發現一隻沾泥的遺落行囊，隨時可彎腰拾起清點！".to_string();
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    if node_id == "oi:dng_pit:stele" {
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "stele_rubbing".to_string(),
                name: "矮人古碑拓片".to_string(),
                count: 1,
                weight_units: 1,
                ..CarryItem::default()
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
        super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
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
        super::phase_and_vessels::ensure_chaos_rune_pool_seeded(delta);
        let cleared_until = delta
            .cleared_nests_until_day
            .get(&node_id)
            .copied()
            .unwrap_or(-1);
        let primordial_gone = delta.first_spider_slain.is_some();
        let altar_next = delta
            .harvested_altars_until_day
            .get("oi:dng_pit:altar")
            .copied()
            .unwrap_or(-1);
        let altar_also_empty = clock.game_days < altar_next || delta.chaos_rune_pool == 0;
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(win_key);
        a.last_active_unix = clock.real_unix;
        a.mark_mutated();
        if clock.game_days < cleared_until {
            if altar_also_empty {
                a.known_rumors.insert("pit_cleared".to_string());
            }
            let msg = if primordial_gone {
                "昔年盤踞於此的吞石巨蛛早已伏誅，空曠石巢裡只剩碎裂甲片，目前尚無其他荒野異獸遷入。"
                    .to_string()
            } else {
                "巨蛛巢穴裡只剩斷裂的灰白蛛網，裡頭已被前人清掃一空。".to_string()
            };
            push_breath(a, msg.clone());
            return Ok(msg);
        }
        a.known_rumors.remove("pit_cleared");
        let (f_slug, f_name, can_gob, intro_msg) = if !primordial_gone {
            (
                "stone_spider",
                "背嵌原石的吞石巨蛛".to_string(),
                false,
                "灰白蛛網劇烈震顫，盤踞多年的吞石巨蛛嘶鳴著撲出，前螯直取你的手腕！".to_string(),
            )
        } else {
            super::live_wilds::pick_dynamic_pit_lair_occupant(delta, clock.game_days)
        };
        if let Some(a) = delta.actors.get_mut(&aid) {
            let can_speak = can_gob && a.languages.get("goblin").copied().unwrap_or(0) >= 25;
            a.active_skirmish = Some(SkirmishState {
                foe_slug: f_slug.to_string(),
                foe_name: f_name,
                can_speak_goblin: can_speak,
                foe_stagger: 0,
                player_guard: false,
                round_idx: 1,
            });
            push_breath(a, intro_msg.clone());
        }
        return Ok(intro_msg);
    }

    if node_id == "oi:dng_pit:altar" {
        super::phase_and_vessels::ensure_chaos_rune_pool_seeded(delta);
        let cleared_spider = delta
            .cleared_nests_until_day
            .get("oi:dng_pit:lair")
            .copied()
            .unwrap_or(-1);
        if clock.game_days >= cleared_spider {
            return Err("外頭巢穴仍有凶獸盤踞，不敢貿然鑿石驚動牠！".to_string());
        }
        let next_avail = delta
            .harvested_altars_until_day
            .get(&node_id)
            .copied()
            .unwrap_or(-1);
        if clock.game_days < next_avail || delta.chaos_rune_pool == 0 {
            let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
            a.explored_windows.insert(win_key);
            a.known_rumors.insert("pit_cleared".to_string());
            a.mark_mutated();
            let msg = if delta.chaos_rune_pool == 0 {
                "石台地脈荒氣已被採斂殆盡，須待外頭銘器磨損解構或下一波地脈潮湧方能再凝原石。"
                    .to_string()
            } else {
                "石台岩縫間已被前人撬空，殘餘荒氣尚未重新凝成原石。".to_string()
            };
            push_breath(a, msg.clone());
            return Ok(msg);
        }
        delta.chaos_rune_pool = delta.chaos_rune_pool.saturating_sub(1);
        delta
            .harvested_altars_until_day
            .insert(node_id, clock.game_days + 6);
        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
        super::live_tide::prune_stale_day_window_keys(&mut a.explored_windows, clock.game_days);
        a.explored_windows.insert(win_key);
        a.known_rumors.insert("pit_cleared".to_string());
        a.last_active_unix = clock.real_unix;
        add_carry_item(
            &mut a.backpack,
            CarryItem {
                slug: "rune_stone".to_string(),
                name: "古符文原石".to_string(),
                count: 1,
                weight_units: 2,
                ..CarryItem::default()
            },
        );
        a.mark_mutated();
        let msg = "你從石台岩縫間小心撬下一塊溫潤發亮的【古符文原石】！帶回巴爾鍛鐵鋪可鑲刻進隨身兵刃，或賣得好價錢。"
            .to_string();
        push_breath(a, msg.clone());
        return Ok(msg);
    }

    super::cell_resources::apply_cell_ecology_explore(doc, delta, clock, &aid, node, win_key)
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
            if !sk.foe_slug.contains("goblin") {
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
            let is_apex = super::cell_resources::is_apex_beast_slug(&sk.foe_slug);
            let is_spider = matches!(sk.foe_slug.as_str(), "stone_spider" | "cave_lizard");
            let is_heavy_boss = sk.foe_slug.starts_with("goblin_camp_chief:")
                || sk.foe_slug.starts_with("mutated_boss:");
            let is_primordial_spider =
                sk.foe_slug == "stone_spider" && delta.first_spider_slain.is_none();
            let reckless_defeat = action == "skirmish_heavy"
                && !sk.player_guard
                && (((is_spider || is_apex || is_heavy_boss)
                    && !has_rune
                    && (fatigue >= 65 || wound_exists))
                    || (fatigue >= 85 && wound_exists));

            if reckless_defeat {
                return trigger_non_lethal_defeat(
                    doc,
                    delta,
                    clock,
                    &aid,
                    &node_id,
                    &sk.foe_slug,
                    &sk.foe_name,
                );
            }

            let has_priv_phase = delta
                .actors
                .get(&aid)
                .is_some_and(|a| a.mind.ltm.mastery.private_martial_phase.is_some());
            let win_threshold =
                if (is_spider || is_apex || is_heavy_boss) && !has_rune && !has_priv_phase {
                    2
                } else {
                    1
                };
            let next_stagger = if action == "skirmish_wrist" {
                sk.foe_stagger + 1
            } else if has_rune || fam >= 14 || sk.player_guard || has_priv_phase {
                sk.foe_stagger + 2
            } else {
                sk.foe_stagger + 1
            };

            if next_stagger >= win_threshold {
                if is_spider {
                    let vac_days = if is_primordial_spider { 8 } else { 6 };
                    delta
                        .cleared_nests_until_day
                        .insert(node_id.clone(), clock.game_days + vac_days);
                }
                let mut rune_loosened = false;
                {
                    let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                    a.active_skirmish = None;
                    a.fatigue = (a.fatigue + 10).min(95);
                    a.bound_weapon.grip_familiarity =
                        a.bound_weapon.grip_familiarity.saturating_add(3);
                    if a.bound_weapon.wear_stage < 2 && !has_rune {
                        a.bound_weapon.wear_stage += 1;
                    } else if has_rune && is_spider && a.mind.ltm.mastery.martial_bouts % 4 == 3 {
                        a.bound_weapon.wear_stage = (a.bound_weapon.wear_stage + 1).min(3);
                        if a.bound_weapon.wear_stage >= 3 {
                            let was_true_rune =
                                a.bound_weapon.rune_mark.as_deref() == Some("穩固鐵紋");
                            a.bound_weapon.rune_mark = None;
                            rune_loosened = was_true_rune;
                        }
                    }
                    a.mind.stm.today_goal_progressed = true;
                    a.mark_mutated();
                }
                if rune_loosened {
                    delta.chaos_rune_pool = (delta.chaos_rune_pool + 1).min(12);
                }
                let _ = super::phase_and_vessels::record_martial_victory_and_check_phase(
                    doc,
                    delta,
                    clock,
                    &aid,
                    &sk.foe_slug,
                    &sk.foe_name,
                    &node_id,
                    is_primordial_spider,
                );

                if let Some(wilds_res) = super::live_wilds::try_settle_wilds_skirmish_victory(
                    doc,
                    delta,
                    clock,
                    &aid,
                    &node_id,
                    &sk.foe_slug,
                    &sk.foe_name,
                ) {
                    if wilds_res.is_ok() {
                        super::map_memory::record_local_witness_at_cur_node(
                            doc,
                            delta,
                            &aid,
                            clock.game_days,
                        );
                    }
                    return wilds_res;
                }

                if is_apex
                    && let Some(apex_msg) = super::cell_resources::record_apex_beast_slain(
                        doc,
                        delta,
                        clock,
                        &aid,
                        &node_id,
                        &sk.foe_name,
                    )
                {
                    if let Some(a) = delta.actors.get_mut(&aid) {
                        push_breath(a, apex_msg.clone());
                    }
                    return Ok(apex_msg);
                }

                if is_spider {
                    let (loot_slug, loot_name, loot_w) = if is_primordial_spider {
                        ("spider_carapace", "吞石巨蛛厚甲殼", 3u16)
                    } else {
                        ("lizard_scale", "嗜石岩蜥厚鱗皮", 2u16)
                    };
                    let (pname, wname) = {
                        let a = delta.actors.get_mut(&aid).ok_or("找不到旅人行蹤")?;
                        add_carry_item(
                            &mut a.backpack,
                            CarryItem {
                                slug: loot_slug.to_string(),
                                name: loot_name.to_string(),
                                count: 1,
                                bound_owner: None,
                                stolen_from: None,
                                weight_units: loot_w,
                                vessel_meta: None,
                                gear_meta: None,
                            },
                        );
                        (a.display_name.clone(), a.bound_weapon.name.clone())
                    };
                    let (ev_key, ev_zh, msg) = if is_primordial_spider {
                        delta.first_spider_slain =
                            Some((aid.clone(), pname.clone(), clock.game_days));
                        super::living_chronicler::record_primordial_spider_slain(
                            doc, delta, clock, &aid, None, &node_id, &wname,
                        );
                        (
                            "slay_stone_spider",
                            format!("{}在灰松試煉坑底擊潰吞石巨蛛並撬下厚甲殼", pname),
                            format!(
                                "你趁巨蛛前螯彈開的空檔踏步猛擊關節，【{}】應聲碎裂重甲！巨蛛嘶鳴潰逃入岩縫，地上留下一片值錢的【吞石巨蛛厚甲殼】，南面通往地脈凝石台的道路已暢行無阻！",
                                wname
                            ),
                        )
                    } else {
                        (
                            "repel_cave_lizard",
                            format!("{}在舊蛛巢擊退空巢易主的{}並剝下厚鱗皮", pname, sk.foe_name),
                            format!(
                                "你沉腰揮動【{}】重重劈開【{}】背脊硬鱗！異獸吃痛鑽回深澗，留下一片堅韌的【嗜石岩蜥厚鱗皮】，通往深處石台的去路重歸暢通！",
                                wname, sk.foe_name
                            ),
                        )
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
                        ev_key,
                        &ev_zh,
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
            let arms_shielded = super::body_gear_craft::arms_absorbs_wrist_recoil(a);
            let recoil_fat: u8 = if arms_shielded { 8 } else { 12 };
            a.fatigue = (a.fatigue + recoil_fat).min(95);
            if !sk.player_guard && is_spider && !arms_shielded {
                a.wound_part = Some("左腕瘀腫".to_string());
            }
            if let Some(ref mut s) = a.active_skirmish {
                s.foe_stagger = next_stagger;
                s.player_guard = false;
                s.round_idx = s.round_idx.saturating_add(1);
            }
            a.mark_mutated();
            let arm_note = if arms_shielded
                && let Some(ref arm_g) = a.body_gear.arms
            {
                format!("，雙臂【{}】穩穩卸去大半反震力道、護住手腕未傷", arm_g.name)
            } else {
                "，震得虎口發麻".to_string()
            };
            let msg = format!(
                "你的【{}】重重砸在【{}】甲殼上濺起火星{}，已逼得對方連退半步——再補一記精準打擊即可破敵！",
                a.bound_weapon.name, sk.foe_name, arm_note
            );
            push_breath(a, msg.clone());
            Ok(msg)
        }
        _ => Err("這一式尚未使順，先穩住架勢再出招。".to_string()),
    }
}

pub fn trigger_non_lethal_defeat(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    node_id: &str,
    foe_slug: &str,
    foe_name: &str,
) -> Result<String, String> {
    let aid = delta.resolve_actor_id(actor_key).ok_or("找不到旅人行蹤")?;
    if super::live_lifecycle::should_trigger_npc_mortal_peril(delta, clock, &aid, node_id) {
        let cause = format!("重傷在身仍孤身死戰，力竭倒在【{foe_name}】手下失救殞命");
        if super::live_lifecycle::execute_npc_passing_and_inheritance(
            doc, delta, clock, &aid, &cause,
        )
        .is_some()
        {
            delta.extras_dirty = true;
        }
        return Ok(cause);
    }

    let (wake_node, dropped_copper, mut dropped_items, wname, victim_label_zh) = {
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
        a.known_rumors.insert("pit_casualty".to_string());

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
        let victim_lbl = if a.controller_id.is_some() || aid.starts_with("pl:") {
            "外來旅人".to_string()
        } else {
            a.display_name.clone()
        };
        a.mark_mutated();
        (wake, dc, dropped, a.bound_weapon.name.clone(), victim_lbl)
    };

    // 同場袖手旁觀留痕：若現場有氣力尚健之熟人卻未拔刀相助，落敗者心頭記下寒意（見死不救）
    let idle_bystanders: Vec<(String, String)> =
        super::phase_and_vessels::witnesses_at_node(delta, node_id, &aid)
            .into_iter()
            .filter(|(wid, _)| {
                delta.actors.get(wid).is_some_and(|w| {
                    w.fatigue < 65 && w.wound_part.is_none() && w.active_skirmish.is_none()
                })
            })
            .collect();
    if !idle_bystanders.is_empty()
        && let Some(a) = delta.actors.get_mut(&aid)
    {
        for (wid, wname) in idle_bystanders {
            if let Some(soc) = a.mind.ltm.social.get_mut(&wid) {
                soc.warmth = (soc.warmth - 16).max(-95);
                soc.notable_note_zh = format!("那日遇險力竭時，{}就在旁邊卻袖手旁觀", wname);
            }
        }
        a.mark_mutated();
    }

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

    let mutated_boss = super::live_wilds::try_absorb_dropped_rune_into_mutated_boss(
        delta,
        node_id,
        foe_slug,
        foe_name,
        &mut dropped_items,
        &victim_label_zh,
        clock.game_days,
    );

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
                promoted_boss_name: Some(
                    mutated_boss
                        .clone()
                        .unwrap_or_else(|| format!("奪囊的{}", foe_name)),
                ),
            });
        }
    }

    let rune_clause = mutated_boss.map_or(String::new(), |bn| {
        format!("更糟的是，懷裡未鎖定的古符文原石被當場吞走，對方已異變成【{bn}】盤踞原地！")
    });
    let msg = format!(
        "你在力竭之下被【{}】震開架勢昏厥倒地！幸得過路獵戶將你拖回城裡甦醒——手裡仍死死攥著【{}】，二樓宿房木箱裡的存貨也安然無恙，但懷中 {} 枚銅幣與隨身散貨遺落在了原處，左腕留下一道重挫瘀腫，敷藥前切莫再逞強硬撼。{}",
        foe_name, wname, dropped_copper, rune_clause
    );
    if let Some(a) = delta.actors.get_mut(&aid) {
        push_breath(a, msg.clone());
    }
    Ok(msg)
}

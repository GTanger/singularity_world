//! `do_action` — Subdue / Slay 分支：內外雙輪戰力 × 田忌賽馬去數值化戰鬥結算、死亡廣播。

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, RwLock};

use crate::ai::narrative::{CombatBoutFactPacket, compose_combat_bouts_concurrent};
use crate::combat::wordplate::{
    derive_elementary_motions_from_chars, infer_item_carrier_affinity, resolve_tianji_matchup,
    resolve_tianji_matchup_with_weights, CarrierAffinity, PostureState, StarInlay, StarSentence,
    TacticalLoadout, TacticalSlot, TargetPart, WordCasket, WordRegistry,
};
use crate::pg::living_world::AfflictionKind;
use crate::db::{
    self, EVT_DEATH, FAV_SLAY, FAV_SUBDUE, ThoughtSource, add_thought, adjust_favorability,
    get_assignments_for_entity, get_room_ids_for_venue, log_npc_event,
    remove_assignments_for_entity, remove_schedule_for_entity, update_vit,
};
use crate::entity::{Character, EntityKind};
use crate::event::{self, types};
use crate::gametext;
use crate::npc::npc_behavior_reaction_line;
use crate::server::handler::{get_player_tactical_loadout, get_player_word_casket};

use super::super::broadcast::send_narrate_to_room;
use super::super::handler::WsConnection;
use super::super::protocol::ActionResultMsg;
use super::super::session::SessionStore;
use super::display_target_name;

pub static ACTIVE_ENGAGEMENTS: LazyLock<RwLock<HashMap<String, (String, i64)>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn handle_tactical_choice(conn: &WsConnection, msg: &crate::server::protocol::ClientMsg) {
    let pid_opt = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(pid) = pid_opt else {
        conn.send_error("login first");
        return;
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let engagement = {
        let mut lock = ACTIVE_ENGAGEMENTS.write().unwrap();
        lock.remove(&pid)
    };
    let Some((engagement_target_id, expires_at)) = engagement else {
        conn.send_error("目前並無進行中之交鋒對峙");
        return;
    };
    if now > expires_at {
        conn.send_error("目前並無進行中之交鋒對峙");
        return;
    }

    let choice = msg.choice.trim();
    let (narrative, vit_delta, qi_delta) = match choice {
        "趁勢搶攻" => (
            "【戰勢後續・趁勢搶攻】你覷準對方氣機微頓的空隙，足踏連環搶步進逼，連續數記快攻凌厲直逼對手門面，逼得對方步步後退、氣息浮動！",
            0,
            1,
        ),
        "專攻部位" => (
            "【戰勢後續・專攻部位】你凝神沈氣鎖定對方防守薄弱之處，拳掌暗勁透勁而入，準確截斷對手發力節點，使其關節發麻攻勢受挫！",
            0,
            2,
        ),
        "守勢游鬥" => (
            "【戰勢後續・守勢游鬥】你氣沈丹田步法空靈，以柔化剛卸開對手剛猛勁力，借力打力周旋游走，體內真氣反倒漸次平順充盈。",
            1,
            0,
        ),
        "抽身脫離" => (
            "【戰勢後續・抽身脫離】你雙袖一振借反震之力倒縱而出，飄然落於丈許開外，收斂周身真氣氣機，從容自交手對峙中脫身而出。",
            2,
            -1,
        ),
        "搏命硬撼" => (
            "【戰勢後續・搏命硬撼】你逆行周天強提真元，不惜臟腑暗傷反噬，以玉石俱焚之勢正面悍然硬撼對手，狂暴剛勁摧枯拉朽！",
            -8,
            3,
        ),
        _ => (
            "【戰勢後續・斂氣凝神】你順應招式餘勢調勻內息，氣勁沉降丹田，對峙之勢隨之淡去。",
            0,
            0,
        ),
    };

    let target_id = if !msg.entity_id.is_empty() {
        msg.entity_id.clone()
    } else {
        engagement_target_id
    };

    if choice == "搏命硬撼" {
        if let Ok(Some(ent)) = db::get_entity(&pid) {
            let new_vit = (ent.vit - 8).max(1);
            let _ = db::update_vit(&pid, new_vit);
        }
        if let Ok(Some(tgt)) = db::get_entity(&target_id) {
            let tgt_new_vit = (tgt.vit - 18).max(1);
            let _ = db::update_vit(&target_id, tgt_new_vit);
        }
        crate::pg::living_world::record_entity_affliction(
            &pid,
            AfflictionKind::TorsoInternal,
            "tactical_desperate_strike",
            now,
        );
    } else if let Ok(Some(ent)) = db::get_entity(&pid) {
        if vit_delta != 0 {
            let new_vit = (ent.vit + vit_delta).max(1);
            let _ = db::update_vit(&pid, new_vit);
        }
    }

    if qi_delta != 0 {
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(&pid, |e| {
                e.qi = (e.qi + qi_delta).clamp(0, 100);
            });
        }
    }


    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "TacticalChoice".into(),
        target_id,
        target_name: "戰術博弈".into(),
        narrative: narrative.into(),
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}

pub(super) fn do_entity_combat(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    target_id: &str,
    action: &str,
    target: &Character,
    now: i64,
) {
    let Ok(Some(attacker)) = db::get_entity(pid) else {
        conn.send_error(gametext::client("get_self_failed"));
        return;
    };
    let subdue = action == "Subdue";
    let (narrative, fight_winner, attacker_hp, defender_hp, a_broken) =
        build_attack_narrative_with_broken(pid, &attacker, target, subdue);
    let mut attacker_hp = attacker_hp;
    let mut narrative = narrative;
    if fight_winner == "defender" && !subdue && (a_broken || attacker_hp <= 18) {
        let (dest_x, dest_y, dest_name) =
            crate::server::handler::movement::find_nearest_clinic_for_combat(attacker.x, attacker.y);
        let (gx, gy) = (attacker.x, attacker.y);
        let dropped_names =
            crate::server::handler::movement::faint_drop_items_to_ground(pid, &attacker.inventory, gx, gy, now);
        let _ = db::set_entity_grid(pid, dest_x, dest_y);
        attacker_hp = 30;
        crate::pg::living_world::record_entity_affliction(
            pid,
            AfflictionKind::TorsoInternal,
            "combat_faint",
            now,
        );
        let drop_note = if dropped_names.is_empty() {
            String::new()
        } else {
            format!(
                "醒轉時察覺行囊微敞，部分隨身物資（{}）已不知散落何方。",
                dropped_names.join("、")
            )
        };
        narrative.push_str(&format!(
            "\n眼前驟然一黑沉入虛無——待悠悠轉醒，鼻端縈繞著草藥與導能凝膠的清冽香氣，你已被巡哨救回【{dest_name}】病榻包紮止血，然體內仍留有暗傷待調養。{}",
            drop_note
        ));
    }
    if let Some((talisman_name, vit_bonus, qi_bonus, bonus_desc)) =
        crate::game::player_shanhai_talisman_bonus(&attacker.inventory)
    {
        attacker_hp = (attacker_hp + vit_bonus).min(100);
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(pid, |e| {
                e.qi = (e.qi + qi_bonus).min(100);
            });
        }
        narrative.push_str(&format!(
            "\n【山海佩飾護佑】隨身佩戴的「{}」展現山海奇效——{}（隨身靈佩引動山海瑞氣，護住心脈並滋養元氣）",
            talisman_name, bonus_desc
        ));
    }
    if target.kind == EntityKind::Npc {
        let extra = if subdue {
            match fight_winner.as_str() {
                "attacker" => npc_behavior_reaction_line(target_id, pid, "subdue_victim"),
                "defender" => npc_behavior_reaction_line(target_id, pid, "lost_subdue"),
                _ => String::new(),
            }
        } else {
            String::new()
        };
        if !extra.is_empty() {
            narrative.push('\n');
            narrative.push_str(&extra);
        }
    }
    if target_id.starts_with("ambient_") || target_id.starts_with("shanhai_") {
        let assisting_party = {
            let parties = crate::store::LIVE_HUNTING_PARTIES.read();
            let pledged_zone = crate::store::player_pledged_hunting_zone(pid);
            parties
                .values()
                .find(|p| {
                    let near_prey = (attacker.x - p.prey_coord.0).abs().max((attacker.y - p.prey_coord.1).abs()) <= 12;
                    let same_prey = p.prey_name == target.display_title;
                    let is_pledged = pledged_zone.as_deref() == Some(p.zone_name.as_str());
                    near_prey || (is_pledged && same_prey)
                })
                .cloned()
        };
        if let Some(party) = assisting_party {
            attacker_hp = (attacker_hp + 12).min(100);
            narrative.push_str(&format!(
                "\n【合社獵團協同助陣】領隊【{}】率領之{}合社獵團（共 {} 人）自側翼吹響山豬牙號角、刀盾齊進封死「{}」退路，助你穩住陣腳（體魄回護 +12）！",
                party.leader_name,
                party.zone_name,
                party.member_ids.len(),
                target.display_title
            ));
            if fight_winner == "attacker" {
                if let Some((loot_id, loot_name, _)) = crate::game::ambient_creature_loot(&target.display_title) {
                    let _ = db::add_to_inventory(pid, loot_id, 1);
                    if let Some(arc) = crate::store::get_store() {
                        let _ = arc.write().update_entity(pid, |e| {
                            e.magnesium = e.magnesium.saturating_add(36);
                        });
                    }
                    {
                        let mut parties = crate::store::LIVE_HUNTING_PARTIES.write();
                        if let Some(p_mut) = parties.get_mut(&party.zone_name) {
                            p_mut.phase = crate::store::HuntingPartyPhase::ReturningToMarket;
                            p_mut.last_phase_change_unix = now;
                        }
                    }
                    narrative.push_str(&format!(
                        "\n【見者有份・合社分鹿】領隊【{}】率眾上前擊掌喝采，依半島合社古規與你當場分鹿割皮——你額外分得一份「{}」與合圍紅利 36 枚鎂幣，獵團隨即滿載獵獲啟程趕往【{}】交割！",
                        party.leader_name, loot_name, party.market_name
                    ));
                }
            }
        }
    }
    if (target_id.starts_with("ambient_") || target_id.starts_with("shanhai_")) && fight_winner == "attacker" {
        if let Some((loot_id, loot_name, shanhai_note)) = crate::game::ambient_creature_loot(&target.display_title) {
            if let Some(rest) = target_id.strip_prefix("shanhai_") {
                let parts: Vec<&str> = rest.split('_').collect();
                if parts.len() == 4
                    && let (Ok(z_idx), Ok(slot_idx), Ok(x), Ok(y)) = (
                        parts[0].parse::<usize>(),
                        parts[1].parse::<usize>(),
                        parts[2].parse::<i32>(),
                        parts[3].parse::<i32>(),
                    )
                {
                    crate::game::record_shanhai_slot_hunted(
                        z_idx,
                        slot_idx,
                        (x, y),
                        &target.display_title,
                        loot_id,
                        loot_name,
                        &attacker.display_title,
                        now,
                    );
                }
            }
            let corpse = crate::pg::living_world::register_beast_corpse(
                &target.display_title,
                loot_id,
                loot_name,
                shanhai_note,
                2,
                now,
            );
            let _ = db::add_to_inventory(pid, &corpse.corpse_item_id, 1);
            narrative.push_str(&format!(
                "\n【山海獵獲】「{}」轟然倒地，你將溫熱的【{}殘軀】收入行囊——可趁靈韻未散嘗試【整軀剝名】一搏高階結晶，或穩妥執行【肢體分解】取下「{}」等部位材料。",
                target.display_title, target.display_title, loot_name
            ));
        }
    }
    let out_action = if subdue { "Subdue" } else { "Slay" };
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: out_action.into(),
        target_id: target.id.clone(),
        target_name: super::player_visible_target_name(pid, target),
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![
            "趁勢搶攻".into(),
            "專攻部位".into(),
            "守勢游鬥".into(),
            "搏命硬撼".into(),
            "抽身脫離".into(),
        ],
    });
    ACTIVE_ENGAGEMENTS
        .write()
        .unwrap()
        .insert(pid.to_string(), (target.id.clone(), now + 300));
    let _ = event::append(now, pid, types::COMBAT, target_id);
    let _ = update_vit(pid, attacker_hp);
    let _ = update_vit(target_id, defender_hp);
    let _ = event::append(now, pid, types::VIT, &attacker_hp.to_string());
    let _ = event::append(now, target_id, types::VIT, &defender_hp.to_string());
    if let Some(pool) = &crate::pg::pool() {
        let _ = crate::pg::living_world::record_relationship_delta(
            pool,
            target_id,
            pid,
            "grudge",
            2,
            action,
            now,
        );
        let _ = crate::pg::living_world::open_obligation(
            pool,
            target_id,
            "treat_injury",
            player_room,
            Some(now + 3600),
            action,
            now,
        );
        let action_key = if subdue { "subdue" } else { "slay" };
        let _ = crate::pg::living_world::record_player_world_action(
            pool,
            pid,
            action_key,
            target_id,
            player_room,
            &fight_winner,
            serde_json::json!({
                "subdue": subdue,
                "attacker_hp": attacker_hp,
                "defender_hp": defender_hp,
                "winner": fight_winner,
            }),
            now,
        );
    }
    if target.kind == EntityKind::Npc {
        if subdue {
            let _ = adjust_favorability(target_id, pid, FAV_SUBDUE);
            if fight_winner == "attacker" {
                let _ = add_thought(target_id, ThoughtSource::Subdued);
            }
        } else {
            let _ = adjust_favorability(target_id, pid, FAV_SLAY);
        }
    }
    if target.kind == EntityKind::Npc && !subdue && defender_hp <= 0 {
        if let Some(pool) = &crate::pg::pool() {
            let _ = crate::pg::living_world::mark_entity_dead(pool, target_id, "combat:slay", now);
        }
        let death_name = crate::store::resolve_npc_person_name(target_id, &target.display_title, None);
        let final_death_name = if !death_name.is_empty() && death_name != "陌生人" {
            death_name
        } else if !target.display_title.is_empty() {
            target.display_title.rsplit(['|', '｜']).next().unwrap_or(&target.display_title).trim().to_string()
        } else if target.id.is_ascii() || target.id.trim().is_empty() {
            "陌生人".to_string()
        } else {
            target.id.clone()
        };
        broadcast_npc_death(
            conn,
            player_room,
            target_id,
            &final_death_name,
            now,
        );
        let _ = remove_assignments_for_entity(target_id);
        let _ = remove_schedule_for_entity(target_id);
    }
}
fn calculate_inscribed_combat_bonus(
    equipment_slots: &str,
    casket: &WordCasket,
    registry: &WordRegistry,
) -> (f64, f64, f64) {
    if equipment_slots.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(equipment_slots) else {
        return (0.0, 0.0, 0.0);
    };
    let mut total_atk = 0.0;
    let mut total_pen = 0.0;
    let mut total_def = 0.0;
    for (_slot, item_id) in slots {
        if !item_id.is_empty() {
            let (atk, pen, def) = casket.inscribed_combat_bonus(&item_id, registry);
            total_atk += atk;
            total_pen += pen;
            total_def += def;
        }
    }
    (total_atk, total_pen, total_def)
}

fn resolve_equipped_weapon_affinity_with_inscriptions(
    equipment_slots: &str,
    casket: &WordCasket,
    registry: &WordRegistry,
) -> (CarrierAffinity, Option<String>) {
    if equipment_slots.is_empty() {
        return (CarrierAffinity::Unarmed, None);
    }
    let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(equipment_slots) else {
        return (CarrierAffinity::Unarmed, None);
    };
    let check_slots = ["hold_r", "weapon", "main_hand", "hold", "hold_l", "tool"];
    for slot in check_slots {
        if let Some(item_id) = slots.get(slot).filter(|id| !id.is_empty()) {
            let def = db::get_item_def(item_id);
            let raw_name = def.as_ref().map(|d| d.name.clone()).unwrap_or_else(|| {
                if item_id.is_ascii() || item_id.trim().is_empty() {
                    "隨身器具".to_string()
                } else {
                    item_id.clone()
                }
            });
            let suffix = casket.format_inscribed_suffix(item_id, registry);
            let full_item_name = format!("{}{}", raw_name, suffix);

            let affinity = infer_item_carrier_affinity(item_id, &raw_name)
                .unwrap_or_else(|| classify_weapon_affinity(&raw_name, "", ""));
            return (affinity, Some(full_item_name));
        }
    }
    (CarrierAffinity::Unarmed, None)
}

fn execute_tianji_wordplate_combat_internal(
    pid: &str,
    attacker: &Character,
    defender: &Character,
    subdue: bool,
) -> (String, String, i32, i32, bool) {
    let registry = WordRegistry::global();

    // 1. 取得攻方戰術配置，若有缺漏自動補齊
    let mut a_loadout = get_player_tactical_loadout(pid);
    ensure_loadout_complete(&mut a_loadout, registry, true);

    // 2. 構建守方適配戰術配置（真正萬人萬相：soul_seed + 760 條拓撲星軌阻力）
    let d_loadout = build_defender_tactical_loadout(defender, registry);

    // 3. 取得攻守雙方詞匣以查詢外盤器物銘文刻字
    let a_casket = get_player_word_casket(pid);
    let d_casket = get_player_word_casket(&defender.id);

    // 4. 計算持物裝備之銘文戰鬥加成
    let (extra_a_atk, extra_a_pen, extra_a_def) =
        calculate_inscribed_combat_bonus(&attacker.equipment_slots, &a_casket, registry);
    let (extra_d_atk, extra_d_pen, extra_d_def) =
        calculate_inscribed_combat_bonus(&defender.equipment_slots, &d_casket, registry);

    // 5. 檢查攻方裝備載體親和度共鳴（器物無分工具與武器：手持即載體），並在名稱顯化刻字後綴
    let (a_weapon_carrier, a_weapon_name) = resolve_equipped_weapon_affinity_with_inscriptions(
        &attacker.equipment_slots,
        &a_casket,
        registry,
    );
    let (d_weapon_carrier, d_weapon_name) = resolve_equipped_weapon_affinity_with_inscriptions(
        &defender.equipment_slots,
        &d_casket,
        registry,
    );
    let resonance_note = inspect_carrier_resonance(
        &a_loadout,
        a_weapon_carrier,
        a_weapon_name.as_deref(),
        registry,
    );
    // 6. 計算有效體魄與攻擊屬性（結合 a_dex, a_atk, d_dex, d_atk 與載體共鳴增益）
    let (a_vit, _a_qi, a_dex, a_atk) = db::effective_stats(attacker);
    let (d_vit, _d_qi, d_dex, d_atk) = db::effective_stats(defender);

    let a_res_bonus = if resonance_note.is_some() { 1.15 } else { 1.0 };
    let a_breath = crate::pg::living_world::get_or_derive_breath_habit(
        pid,
        &attacker.display_title,
        attacker.soul_seed.unwrap_or(0),
    );
    let d_breath = crate::pg::living_world::get_or_derive_breath_habit(
        &defender.id,
        &defender.display_title,
        defender.soul_seed.unwrap_or(0),
    );
    let (a_b_atk, a_b_def, a_b_pen, a_b_line) = a_breath.dominant.combat_modifiers();
    let (d_b_atk, d_b_def, d_b_pen, _) = d_breath.dominant.combat_modifiers();

    let effective_a_atk = ((a_atk as f64) + extra_a_atk + (a_dex as f64 * 0.25)) * a_res_bonus * a_b_atk;
    let effective_d_atk = ((d_atk as f64) + extra_d_atk + (d_dex as f64 * 0.25)) * d_b_atk;
    let effective_a_pen = extra_a_pen + a_b_pen;
    let effective_d_pen = extra_d_pen + d_b_pen;

    let cur_a_hp = ((a_vit as f64 + extra_a_def) * a_b_def).max(1.0).round() as i32;
    let cur_d_hp = ((d_vit as f64 + extra_d_def) * d_b_def).max(1.0).round() as i32;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let a_afflictions = crate::pg::living_world::get_entity_afflictions(pid);
    let d_afflictions = crate::pg::living_world::get_entity_afflictions(&defender.id);

    let calc_affliction_penalty = |afflictions: &[AfflictionKind]| -> f64 {
        let count = afflictions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    AfflictionKind::ArmSprain
                        | AfflictionKind::TorsoInternal
                        | AfflictionKind::HeadConcussion
                        | AfflictionKind::LegSprain
                        | AfflictionKind::Miasma
                )
            })
            .count();
        if count == 0 {
            1.0
        } else {
            (0.88f64).powi(count as i32).max(0.65)
        }
    };
    let penalty_a = calc_affliction_penalty(&a_afflictions);
    let penalty_d = calc_affliction_penalty(&d_afflictions);

    // 7. 調用田忌賽馬時序對決結算（融入量級權重與專克／一力降十會裁決）
    let weight_a =
        (effective_a_atk * 0.65 + (cur_a_hp as f64) * 0.35 + effective_a_pen * 0.5) * penalty_a;
    let weight_b =
        (effective_d_atk * 0.65 + (cur_d_hp as f64) * 0.35 + effective_d_pen * 0.5) * penalty_d;

    let matchup_result = resolve_tianji_matchup_with_weights(
        &a_loadout,
        cur_a_hp,
        &d_loadout,
        cur_d_hp,
        weight_a,
        weight_b,
        registry,
    );

    let (raw_winner, rounds, side_a_final, side_b_final) = match matchup_result {
        Ok(report) => (
            report.winner,
            report.rounds,
            report.side_a_final_hp,
            report.side_b_final_hp,
        ),
        Err(_) => {
            return (
                "雙方氣機交錯，各自收勢退開。".to_string(),
                "draw".to_string(),
                cur_a_hp,
                cur_d_hp,
                false,
            );
        }
    };

    let fight_winner = match raw_winner.as_str() {
        "side_a" => "attacker".to_string(),
        "side_b" => "defender".to_string(),
        _ => "draw".to_string(),
    };

    // 8. 計算血量結算（Subdue 切磋留人絕不致死，Slay 決一死戰可奪命）
    let (raw_final_a, raw_final_d) = if subdue {
        (side_a_final.max(1), side_b_final.max(1))
    } else if fight_winner == "attacker" {
        let hp_d = if defender.id.starts_with("ambient_") || defender.id.starts_with("shanhai_") || side_b_final <= 5 {
            0
        } else {
            crate::pg::living_world::record_entity_affliction(
                &defender.id,
                AfflictionKind::TorsoInternal,
                "combat_scuffle",
                now,
            );
            side_b_final.max(1)
        };
        (side_a_final.max(1), hp_d)
    } else {
        (side_a_final.max(1), side_b_final.max(1))
    };
    let final_a_hp = raw_final_a.min(attacker.vit);
    let final_d_hp = raw_final_d.min(defender.vit);

    // 9. 生成 100% 完全去數值化武俠感官散文
    let a_name = if !attacker.display_title.is_empty() {
        attacker.display_title.clone()
    } else {
        "你".to_string()
    };
    let mut d_name = super::player_visible_target_name(pid, defender);
    if d_name.is_empty() || d_name.is_ascii() {
        d_name = if defender.id.is_ascii() || defender.id.trim().is_empty() {
            "陌生人".to_string()
        } else {
            defender.id.clone()
        };
    }

    let mut prose_log = render_dequantified_combat_prose(
        pid,
        &defender.id,
        now,
        &a_name,
        &d_name,
        subdue,
        &fight_winner,
        &rounds,
        resonance_note.as_deref(),
        defender.soul_seed,
        a_weapon_name.as_deref().unwrap_or("徒手"),
        d_weapon_name.as_deref().unwrap_or("徒手"),
        a_weapon_carrier,
        d_weapon_carrier,
    );
    prose_log.push_str(&format!("\n{a_b_line}"));

    crate::pg::living_world::record_breath_activity(
        pid,
        crate::pg::living_world::BreathHabitKind::BattleFierce,
        2,
        now,
    );

    let weapon_id_opt: Option<String> = if !attacker.equipment_slots.is_empty() {
        if let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(&attacker.equipment_slots) {
            let check_slots = ["weapon", "hold_r", "main_hand", "hold", "tool", "hold_l"];
            check_slots.iter().find_map(|s| slots.get(*s).cloned().filter(|id| !id.is_empty()))
        } else {
            None
        }
    } else {
        None
    };

    if let Some(weapon_id) = weapon_id_opt {
        let mut vstate = crate::pg::living_world::record_vessel_use(pid, &weapon_id, 1, 1, now);
        let weapon_clashed = rounds.iter().any(|r| {
            r.b_target_part == TargetPart::Weapon
                || (matches!(r.a_carrier, CarrierAffinity::Blade | CarrierAffinity::Pole) && matches!(r.b_carrier, CarrierAffinity::Blunt | CarrierAffinity::Shield | CarrierAffinity::Pole))
        });
        if weapon_clashed {
            vstate = crate::pg::living_world::record_vessel_use(pid, &weapon_id, 2, 1, now);
        }
        match vstate.condition {
            crate::pg::living_world::VesselCondition::SlightlyDull => {
                prose_log.push_str("\n【器況體察】連番交鋒硬撼，手中兵刃已顯「刃口微鈍」，戰後宜尋街邊青石磨刀槽推磨開鋒。");
            }
            crate::pg::living_world::VesselCondition::RolledEdge => {
                prose_log.push_str("\n【器況體察】連番交鋒硬撼，手中兵刃已現「捲刃待修」，宜赴鍛造工坊回火修整。");
            }
            crate::pg::living_world::VesselCondition::Sharp => {}
        }
    }
    let a_posture_broken = rounds
        .last()
        .map(|r| r.a_posture_after == PostureState::Broken)
        .unwrap_or(false);

    (prose_log, fight_winner, final_a_hp, final_d_hp, a_posture_broken)
}

pub fn execute_tianji_wordplate_combat(
    pid: &str,
    attacker: &Character,
    defender: &Character,
    subdue: bool,
) -> (String, String, i32, i32) {
    let (prose, winner, a_hp, d_hp, _) =
        execute_tianji_wordplate_combat_internal(pid, attacker, defender, subdue);
    (prose, winner, a_hp, d_hp)
}

fn build_attack_narrative(
    pid: &str,
    attacker: &Character,
    defender: &Character,
    subdue: bool,
) -> (String, String, i32, i32) {
    execute_tianji_wordplate_combat(pid, attacker, defender, subdue)
}

fn build_attack_narrative_with_broken(
    pid: &str,
    attacker: &Character,
    defender: &Character,
    subdue: bool,
) -> (String, String, i32, i32, bool) {
    execute_tianji_wordplate_combat_internal(pid, attacker, defender, subdue)
}

fn build_defender_tactical_loadout(
    defender: &Character,
    registry: &WordRegistry,
) -> TacticalLoadout {
    // 1. 若守方已有自訂／持久化之戰術配置，優先使用
    let existing_loadout = get_player_tactical_loadout(&defender.id);
    let casket = get_player_word_casket(&defender.id);

    if existing_loadout.opening.is_some()
        || existing_loadout.midgame.is_some()
        || existing_loadout.finisher.is_some()
    {
        let mut loadout = existing_loadout;
        ensure_loadout_complete(&mut loadout, registry, false);
        return loadout;
    }

    // 2. 徹底移除舊版硬編碼模板，依據生命種子（soul_seed）＋ 760 條星軌拓撲展開真正的萬人萬相
    if let Some(seed) = defender.soul_seed.filter(|&s| s != 0) {
        crate::combat::wordplate::build_loadout_from_soul_seed(seed, Some(&casket), registry)
    } else {
        let synthetic_seed = (defender.vit as i64)
            .wrapping_mul(31)
            .wrapping_add(defender.dex as i64)
            .wrapping_mul(31)
            .wrapping_add(defender.qi as i64)
            .wrapping_mul(1000003)
            .wrapping_add(
                defender
                    .id
                    .bytes()
                    .fold(0i64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as i64)),
            );
        let actual_seed = if synthetic_seed == 0 { 42 } else { synthetic_seed };
        crate::combat::wordplate::build_loadout_from_soul_seed(actual_seed, Some(&casket), registry)
    }
}

fn make_3word_sentence(
    nodes: [&str; 3],
    chars: [&str; 3],
    name: &str,
    registry: &WordRegistry,
    fallback_offset: usize,
) -> StarSentence {
    let get_id = |ch: &str, offset: usize| -> String {
        if let Some(ids) = registry.get_by_char(ch)
            && let Some(first) = ids.first()
        {
            first.clone()
        } else if let Some(elem) = registry.ordered_elements().nth(offset) {
            elem.id.clone()
        } else {
            "wu".to_string()
        }
    };
    let inlays = vec![
        StarInlay::new(nodes[0], get_id(chars[0], fallback_offset)),
        StarInlay::new(nodes[1], get_id(chars[1], fallback_offset + 1)),
        StarInlay::new(nodes[2], get_id(chars[2], fallback_offset + 2)),
    ];
    StarSentence::new(inlays)
        .unwrap_or_else(|_| {
            let elems: Vec<String> = registry
                .ordered_elements()
                .take(3)
                .map(|e| e.id.clone())
                .collect();
            StarSentence::new(vec![
                StarInlay::new(nodes[0], elems.first().cloned().unwrap_or_default()),
                StarInlay::new(nodes[1], elems.get(1).cloned().unwrap_or_default()),
                StarInlay::new(nodes[2], elems.get(2).cloned().unwrap_or_default()),
            ])
            .expect("fallback sentence must be valid")
        })
        .with_custom_name(name)
}

fn ensure_loadout_complete(
    loadout: &mut TacticalLoadout,
    registry: &WordRegistry,
    is_attacker: bool,
) {
    let offset_base = if is_attacker { 1 } else { 13 };
    if loadout.opening.is_none() {
        let n1 = format!("N{:03}", offset_base);
        let n2 = format!("N{:03}", offset_base + 1);
        let n3 = format!("N{:03}", offset_base + 2);
        loadout.set_slot(
            TacticalSlot::Opening,
            make_3word_sentence([&n1, &n2, &n3], ["疾", "刺", "斬"], "疾刺斬", registry, 0),
        );
    }
    if loadout.midgame.is_none() {
        let n4 = format!("N{:03}", offset_base + 3);
        let n5 = format!("N{:03}", offset_base + 4);
        let n6 = format!("N{:03}", offset_base + 5);
        loadout.set_slot(
            TacticalSlot::Midgame,
            make_3word_sentence([&n4, &n5, &n6], ["流", "破", "斬"], "流破斬", registry, 3),
        );
    }
    if loadout.finisher.is_none() {
        let n7 = format!("N{:03}", offset_base + 6);
        let n8 = format!("N{:03}", offset_base + 7);
        let n9 = format!("N{:03}", offset_base + 8);
        loadout.set_slot(
            TacticalSlot::Finisher,
            make_3word_sentence([&n7, &n8, &n9], ["吾", "震", "破"], "吾震破", registry, 6),
        );
    }
    if loadout.defense.is_none() {
        let n10 = format!("N{:03}", offset_base + 9);
        let n11 = format!("N{:03}", offset_base + 10);
        let n12 = format!("N{:03}", offset_base + 11);
        loadout.set_slot(
            TacticalSlot::Defense,
            make_3word_sentence([&n10, &n11, &n12], ["凝", "流", "步"], "凝流步", registry, 9),
        );
    }
}

fn resolve_equipped_weapon_affinity(equipment_slots: &str) -> (CarrierAffinity, Option<String>) {
    if equipment_slots.is_empty() {
        return (CarrierAffinity::Unarmed, None);
    }
    let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(equipment_slots) else {
        return (CarrierAffinity::Unarmed, None);
    };
    let check_slots = ["hold_r", "weapon", "main_hand", "hold", "hold_l"];
    for slot in check_slots {
        if let Some(item_id) = slots.get(slot).filter(|id| !id.is_empty()) {
            if let Some(def) = db::get_item_def(item_id) {
                let affinity =
                    classify_weapon_affinity(&def.name, &def.description, &def.item_type);
                return (affinity, Some(def.name));
            } else {
                let affinity = classify_weapon_affinity(item_id, "", "");
                let weapon_name = if item_id.is_ascii() || item_id.trim().is_empty() {
                    Some("隨身兵刃".to_string())
                } else {
                    Some(item_id.clone())
                };
                return (affinity, weapon_name);
            }
        }
    }
    (CarrierAffinity::Unarmed, None)
}

fn classify_weapon_affinity(name: &str, desc: &str, item_type: &str) -> CarrierAffinity {
    let lower = format!("{name} {desc} {item_type}").to_lowercase();
    if lower.contains('劍')
        || lower.contains('刀')
        || lower.contains('刃')
        || lower.contains('匕')
        || lower.contains("blade")
        || lower.contains("sword")
        || lower.contains("dagger")
    {
        CarrierAffinity::Blade
    } else if lower.contains('槍')
        || lower.contains('矛')
        || lower.contains('戟')
        || lower.contains('棍')
        || lower.contains('杖')
        || lower.contains("pole")
        || lower.contains("spear")
        || lower.contains("lance")
        || lower.contains("staff")
    {
        CarrierAffinity::Pole
    } else if lower.contains('錘')
        || lower.contains('斧')
        || lower.contains('鎬')
        || lower.contains('鈍')
        || lower.contains("blunt")
        || lower.contains("hammer")
        || lower.contains("mace")
        || lower.contains("axe")
    {
        CarrierAffinity::Blunt
    } else if lower.contains('盾') || lower.contains("shield") {
        CarrierAffinity::Shield
    } else {
        CarrierAffinity::Unarmed
    }
}

fn inspect_carrier_resonance(
    loadout: &TacticalLoadout,
    weapon_affinity: CarrierAffinity,
    weapon_name: Option<&str>,
    registry: &WordRegistry,
) -> Option<String> {
    let resonates = [&loadout.opening, &loadout.midgame, &loadout.finisher]
        .iter()
        .filter_map(|s| s.as_ref())
        .any(|sentence| {
            if let Ok(tech) = sentence.synthesize(registry) {
                tech.primary_carrier == weapon_affinity
            } else {
                false
            }
        });

    if !resonates && weapon_affinity == CarrierAffinity::Unarmed {
        return Some("身形空靈，雙掌氣韻流轉，勁力透骨化有形為無形！".to_string());
    }

    if resonates {
        match weapon_affinity {
            CarrierAffinity::Blade => {
                if let Some(w) = weapon_name {
                    Some(format!(
                        "手握【{w}】，刀劍鋒芒與招式吞吐相通，森寒劍意直逼肺腑！"
                    ))
                } else {
                    Some("劍氣縱橫破空，式意與刀劍鋒芒渾然天成！".to_string())
                }
            }
            CarrierAffinity::Pole => {
                if let Some(w) = weapon_name {
                    Some(format!(
                        "緊握【{w}】，長兵揮灑生風，槍勢勢大力沉貫穿長空！"
                    ))
                } else {
                    Some("長槍破空長嘶，氣芒大開大闔，銳不可當！".to_string())
                }
            }
            CarrierAffinity::Blunt => {
                if let Some(w) = weapon_name {
                    Some(format!(
                        "揮舞【{w}】，重錘剛猛無匹，威沉如泰山崩岳！"
                    ))
                } else {
                    Some("剛猛重擊破風而落，勢若奔雷下凡！".to_string())
                }
            }
            CarrierAffinity::Shield => {
                if let Some(w) = weapon_name {
                    Some(format!(
                        "挽起【{w}】，盾隨身轉，攻守圓融固若金湯！"
                    ))
                } else {
                    Some("堅盾化勁護體，守勢滴水不漏！".to_string())
                }
            }
            CarrierAffinity::Unarmed => {
                Some("拳掌通達返璞歸真，舉手投足俱是純陽氣勁！".to_string())
            }
            CarrierAffinity::Footwork => {
                Some("步法神妙飄逸，踏風而行，身影虛實交錯！".to_string())
            }
        }
    } else {
        None
    }
}

fn render_dequantified_combat_prose(
    a_id: &str,
    d_id: &str,
    now: i64,
    a_name: &str,
    d_name: &str,
    subdue: bool,
    fight_winner: &str,
    rounds: &[crate::combat::wordplate::PhaseClashRecord],
    resonance_note: Option<&str>,
    defender_seed: Option<i64>,
    a_weapon_name: &str,
    d_weapon_name: &str,
    a_weapon_carrier: CarrierAffinity,
    d_weapon_carrier: CarrierAffinity,
) -> String {
    let mut lines = Vec::new();

    // 意圖起手
    let intent_line = if subdue {
        format!("你凝神收住力道，向【{d_name}】展開制伏攻擊！")
    } else {
        format!("你目光如炬，向【{d_name}】全力出擊，意在致死！")
    };
    lines.push(intent_line);

    // 載體共鳴提示
    if let Some(res) = resonance_note {
        lines.push(format!("（{res}）"));
    }

    // 身形感知
    if let Some(seed) = defender_seed.filter(|&s| s != 0) {
        let style = combat_style_flavor(seed);
        lines.push(format!("（【{d_name}】氣息沉凝，身法動靜間盡顯{style}之象）"));
    }

    lines.push(String::new());

    let mut packets = Vec::new();
    for record in rounds {
        if record.slot == TacticalSlot::Defense {
            continue;
        }

        let a_motions = if record.a_motions.is_empty() || record.a_carrier != a_weapon_carrier {
            derive_elementary_motions_from_chars(&[], a_weapon_carrier, record.a_target_part)
        } else {
            record.a_motions.clone()
        };

        let b_motions = if record.b_motions.is_empty() || record.b_carrier != d_weapon_carrier {
            derive_elementary_motions_from_chars(&[], d_weapon_carrier, record.b_target_part)
        } else {
            record.b_motions.clone()
        };

        let winner_side = if record.a_manifested_as_skill && !record.b_manifested_as_skill {
            "attacker"
        } else if record.b_manifested_as_skill && !record.a_manifested_as_skill {
            "defender"
        } else if record.damage_to_b > record.damage_to_a {
            "attacker"
        } else if record.damage_to_a > record.damage_to_b {
            "defender"
        } else {
            "draw"
        };

        let active_residue = record.active_residue_before.map(|r| r.label().to_string());

        let (residue_interaction, loser_posture_after, inflicted_injury) = match winner_side {
            "attacker" => {
                let res_inter = record.a_residue_interaction.map(|i| i.label().to_string());
                let posture = if record.b_posture_after != PostureState::Steady {
                    Some(record.b_posture_after.label().to_string())
                } else {
                    None
                };
                let inj = if !subdue && (record.damage_to_b >= 18 || record.b_posture_after == PostureState::Broken) {
                    let (label, kind_opt) = match record.a_target_part {
                        TargetPart::Head => (Some("頭面震盪"), Some(AfflictionKind::HeadConcussion)),
                        TargetPart::Torso => (Some("胸脅內傷"), Some(AfflictionKind::TorsoInternal)),
                        TargetPart::Arm => (Some("腕肘挫傷"), Some(AfflictionKind::ArmSprain)),
                        TargetPart::Leg => (Some("下盤扭傷"), Some(AfflictionKind::LegSprain)),
                        TargetPart::Weapon => (Some("兵刃相交硬磕之下，虎口劇震、刃口隱現磨損微鈍之跡"), None),
                        TargetPart::SelfTarget => (None, None),
                    };
                    if let Some(kind) = kind_opt {
                        crate::pg::living_world::record_entity_affliction(d_id, kind, "combat", now);
                    }
                    label.map(str::to_string)
                } else {
                    None
                };
                (res_inter, posture, inj)
            }
            "defender" => {
                let res_inter = record.b_residue_interaction.map(|i| i.label().to_string());
                let posture = if record.a_posture_after != PostureState::Steady {
                    Some(record.a_posture_after.label().to_string())
                } else {
                    None
                };
                let inj = if !subdue && (record.damage_to_a >= 18 || record.a_posture_after == PostureState::Broken) {
                    let (label, kind_opt) = match record.b_target_part {
                        TargetPart::Head => (Some("頭面震盪"), Some(AfflictionKind::HeadConcussion)),
                        TargetPart::Torso => (Some("胸脅內傷"), Some(AfflictionKind::TorsoInternal)),
                        TargetPart::Arm => (Some("腕肘挫傷"), Some(AfflictionKind::ArmSprain)),
                        TargetPart::Leg => (Some("下盤扭傷"), Some(AfflictionKind::LegSprain)),
                        TargetPart::Weapon => (Some("兵刃相交硬磕之下，虎口劇震、刃口隱現磨損微鈍之跡"), None),
                        TargetPart::SelfTarget => (None, None),
                    };
                    if let Some(kind) = kind_opt {
                        crate::pg::living_world::record_entity_affliction(a_id, kind, "combat", now);
                    }
                    label.map(str::to_string)
                } else {
                    None
                };
                (res_inter, posture, inj)
            }
            _ => (None, None, None),
        };

        let packet = match winner_side {
            "attacker" => {
                let severity_label = if record.damage_to_b >= 25 {
                    "重創失據".to_string()
                } else if record.damage_to_b >= 12 {
                    "氣血翻湧".to_string()
                } else {
                    "輕微擦震".to_string()
                };
                CombatBoutFactPacket {
                    slot: record.slot,
                    attacker_name: a_name.to_string(),
                    defender_name: d_name.to_string(),
                    attacker_weapon: a_weapon_name.to_string(),
                    defender_weapon: d_weapon_name.to_string(),
                    attacker_carrier: a_weapon_carrier,
                    defender_carrier: d_weapon_carrier,
                    winner_side: "attacker".to_string(),
                    winner_skill_name: record.a_technique_name.clone(),
                    loser_skill_name: record.b_technique_name.clone(),
                    winner_motions: a_motions,
                    loser_attempted_motions: b_motions
                        .into_iter()
                        .take(record.b_completed_motions.max(1))
                        .collect(),
                    counter_reason: record.counter_reason.clone(),
                    hit_part: record.a_target_part,
                    severity_label,
                    active_residue,
                    residue_interaction,
                    loser_posture_after,
                    inflicted_injury,
                }
            }
            "defender" => {
                let severity_label = if record.damage_to_a >= 25 {
                    "重創失據".to_string()
                } else if record.damage_to_a >= 12 {
                    "氣血翻湧".to_string()
                } else {
                    "輕微擦震".to_string()
                };
                CombatBoutFactPacket {
                    slot: record.slot,
                    attacker_name: a_name.to_string(),
                    defender_name: d_name.to_string(),
                    attacker_weapon: a_weapon_name.to_string(),
                    defender_weapon: d_weapon_name.to_string(),
                    attacker_carrier: a_weapon_carrier,
                    defender_carrier: d_weapon_carrier,
                    winner_side: "defender".to_string(),
                    winner_skill_name: record.b_technique_name.clone(),
                    loser_skill_name: record.a_technique_name.clone(),
                    winner_motions: b_motions,
                    loser_attempted_motions: a_motions
                        .into_iter()
                        .take(record.a_completed_motions.max(1))
                        .collect(),
                    counter_reason: record.counter_reason.clone(),
                    hit_part: record.b_target_part,
                    severity_label,
                    active_residue,
                    residue_interaction,
                    loser_posture_after,
                    inflicted_injury,
                }
            }
            _ => CombatBoutFactPacket {
                slot: record.slot,
                attacker_name: a_name.to_string(),
                defender_name: d_name.to_string(),
                attacker_weapon: a_weapon_name.to_string(),
                defender_weapon: d_weapon_name.to_string(),
                attacker_carrier: a_weapon_carrier,
                defender_carrier: d_weapon_carrier,
                winner_side: "draw".to_string(),
                winner_skill_name: String::new(),
                loser_skill_name: String::new(),
                winner_motions: vec![],
                loser_attempted_motions: a_motions
                    .into_iter()
                    .take(record.a_completed_motions.max(1))
                    .collect(),
                counter_reason: record.counter_reason.clone(),
                hit_part: record.a_target_part,
                severity_label: "輕微擦震".to_string(),
                active_residue,
                residue_interaction: None,
                loser_posture_after: None,
                inflicted_injury: None,
            },
        };
        packets.push(packet);
    }

    let bout_lines = compose_combat_bouts_concurrent(&packets);
    lines.extend(bout_lines);

    lines.push(String::new());

    let summary = match (subdue, fight_winner) {
        (true, "attacker") => {
            "雙方身形驟分。你收住最後一分力道，將對方徹底制伏。"
        }
        (true, "defender") => {
            "雙方收招罷手。對方身法沉穩老練，你未能破開其守勢，暫落下風。"
        }
        (true, _) => "數合交手告終，兩人氣息相若，各自點頭收勢。",
        (false, "attacker") => "塵埃落定。你一鼓作氣瓦解敵勢，奪下勝局！",
        (false, "defender") => "狂瀾散去。對方招式老辣，你力有不逮，敗下陣來。",
        (false, _) => "漫天塵土消散，雙方互不相讓，戰局陷入對峙。",
    };
    lines.push(summary.to_string());

    lines.join("\n")
}

fn combat_style_flavor(seed: i64) -> &'static str {
    let (alpha, beta, gamma) = db::expand_soul_seed_to_combat_axes(seed);
    let norm_amp = (alpha - 0.5) / 1.5;
    let norm_freq = (beta - 0.5) / 1.0;
    let norm_phase = (gamma - 0.05) / 0.15;
    if norm_amp >= norm_freq && norm_amp >= norm_phase {
        "剛猛"
    } else if norm_freq >= norm_amp && norm_freq >= norm_phase {
        "疾促"
    } else {
        "沉穩"
    }
}

fn broadcast_npc_death(
    conn: &WsConnection,
    death_room: &str,
    npc_id: &str,
    name: &str,
    now: i64,
) {
    let payload = format!("在{death_room}倒下");
    log_npc_event(now, npc_id, EVT_DEATH, &payload);
    let safe_name = if name.is_ascii() || name.trim().is_empty() {
        "陌生人"
    } else {
        name
    };
    let narrate_text = format!("傳來消息：【{safe_name}】倒下了。");
    let mut rooms: HashSet<String> = HashSet::new();
    rooms.insert(death_room.to_string());
    db::with_room_graph(|g| {
        for nb in g.neighbors(death_room) {
            rooms.insert(nb);
        }
    });
    if let Ok(assignments) = get_assignments_for_entity(npc_id) {
        for a in assignments {
            if let Ok(Some(vr)) = get_room_ids_for_venue(&a.venue_id) {
                for rid in vr {
                    rooms.insert(rid);
                }
            }
        }
    }
    let store_ref: &SessionStore = &conn.sessions;
    for rid in rooms {
        send_narrate_to_room(store_ref, &rid, &narrate_text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_weapon_affinity_keywords() {
        assert_eq!(
            classify_weapon_affinity("青鋼長劍", "三尺青鋒", "weapon"),
            CarrierAffinity::Blade
        );
        assert_eq!(
            classify_weapon_affinity("點鋼槍", "長丈二", "weapon"),
            CarrierAffinity::Pole
        );
        assert_eq!(
            classify_weapon_affinity("八角重錘", "沉重無匹", "weapon"),
            CarrierAffinity::Blunt
        );
        assert_eq!(
            classify_weapon_affinity("精鐵大盾", "厚實重禦", "shield"),
            CarrierAffinity::Shield
        );
        assert_eq!(
            classify_weapon_affinity("草鞋", "普通草鞋", "armor"),
            CarrierAffinity::Unarmed
        );
    }

    #[test]
    fn test_dequantified_prose_contains_no_digits() {
        let rounds = vec![
            crate::combat::wordplate::PhaseClashRecord {
                phase: 1,
                slot: TacticalSlot::Opening,
                a_technique_name: "疾刺斬".into(),
                b_technique_name: "震天裂石勢".into(),
                a_initiative_score: 1.5,
                b_initiative_score: 1.0,
                damage_to_a: 0,
                damage_to_b: 10,
                narrative: "此處不管原字串".into(),
                a_manifested_as_skill: true,
                b_manifested_as_skill: false,
                counter_reason: Some("專克拆招・以巧破剛：疾速巧勁穿透重擊硬直破防！".into()),
                ..Default::default()
            },
            crate::combat::wordplate::PhaseClashRecord {
                phase: 2,
                slot: TacticalSlot::Midgame,
                a_technique_name: "流破斬".into(),
                b_technique_name: "摧山覆岳擊".into(),
                a_initiative_score: 2.0,
                b_initiative_score: 1.2,
                damage_to_a: 0,
                damage_to_b: 15,
                narrative: "此處不管原字串".into(),
                a_manifested_as_skill: true,
                b_manifested_as_skill: false,
                counter_reason: None,
                ..Default::default()
            },
            crate::combat::wordplate::PhaseClashRecord {
                phase: 3,
                slot: TacticalSlot::Finisher,
                a_technique_name: "吾震破".into(),
                b_technique_name: "九霄神殛破".into(),
                a_initiative_score: 2.5,
                b_initiative_score: 1.5,
                damage_to_a: 0,
                damage_to_b: 25,
                narrative: "此處不管原字串".into(),
                a_manifested_as_skill: true,
                b_manifested_as_skill: false,
                counter_reason: None,
                ..Default::default()
            },
        ];

        let prose = render_dequantified_combat_prose(
            "hero_1",
            "npc_blacksmith",
            0,
            "你",
            "王鐵匠",
            true,
            "attacker",
            &rounds,
            Some("手握【青鋼長劍】，刀劍鋒芒與招式吞吐相通，森寒劍意直逼肺腑！"),
            Some(123456789),
            "青鋼長劍",
            "鍛造鐵錘",
            CarrierAffinity::Blade,
            CarrierAffinity::Blunt,
        );

        // 嚴格校驗：向玩家輸出的感官散文絕無任何量化阿拉伯數字
        for ch in prose.chars() {
            assert!(
                !ch.is_ascii_digit(),
                "戰鬥散文不得包含任何阿拉伯數字：發現 '{ch}' 在散文中：\n{prose}"
            );
        }
        assert!(prose.contains("「"));
        assert!(prose.contains("」"));
    }

    #[test]
    fn test_execute_tianji_wordplate_combat_with_inscribed_tool() {
        let registry = WordRegistry::global();
        let w1 = registry.ordered_elements().next().unwrap().id.clone();
        let w2 = registry.ordered_elements().nth(1).unwrap().id.clone();
        let mut casket = WordCasket::new();
        // 銘刻削皮彎刀
        casket.add(&w1, 5);
        casket.add(&w2, 5);
        let _ = casket.inscribe_item("curved_peeling_knife", &w1);
        let _ = casket.inscribe_item("curved_peeling_knife", &w2);
        crate::server::handler::set_player_word_casket("test_hero_combat", casket);

        let attacker: Character = serde_json::from_str(
            r#"{"id":"test_hero_combat","kind":"player","display_title":"名刀俠客","vit":120,"dex":20,"qi":15,"equipment_slots":"{\"hold_r\":\"curved_peeling_knife\"}"}"#,
        ).unwrap();

        let defender: Character = serde_json::from_str(
            r#"{"id":"npc_target_combat","kind":"npc","display_title":"黑風山寇","vit":90,"dex":12,"qi":8,"soul_seed":987654321}"#,
        ).unwrap();

        let (prose, winner, final_a, final_d) =
            execute_tianji_wordplate_combat("test_hero_combat", &attacker, &defender, true);

        assert!(!prose.is_empty());
        assert!(final_a >= 1);
        assert!(final_d >= 1);
        assert!(winner == "attacker" || winner == "defender" || winner == "draw");
        for ch in prose.chars() {
            assert!(!ch.is_ascii_digit(), "Prose must not contain digits: {ch}");
        }
        assert!(prose.contains("「"));
    }
}

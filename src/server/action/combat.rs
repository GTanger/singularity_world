//! `do_action` — Subdue / Slay 分支：內外雙輪戰力 × 田忌賽馬去數值化戰鬥結算、死亡廣播。

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, RwLock};

use crate::ai::narrative::{CombatBoutFactPacket, compose_combat_bouts_concurrent};
use crate::combat::wordplate::{
    derive_elementary_motions_from_chars, infer_item_carrier_affinity, CarrierAffinity,
    PostureState, StarInlay, StarSentence, TacticalLoadout, TacticalSlot, TargetPart, WordCasket,
    WordRegistry,
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
            "【戰勢後續・專攻部位】你凝神沉氣鎖定對方防守薄弱之處，拳掌暗勁透勁而入，準確截斷對手發力節點，使其關節發麻攻勢受挫！",
            0,
            2,
        ),
        "守勢游鬥" => (
            "【戰勢後續・守勢游鬥】你氣沉氣海步法空靈，以柔化剛卸開對手剛猛勁力，借力打力周旋游走，體內真氣反倒漸次平順充盈。",
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
            "【戰勢後續・斂氣凝神】你順應招式餘勢調勻內息，氣勁沉降氣海，對峙之勢隨之淡去。",
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
    let target_is_beast = is_wild_creature_entity(target);
    if target.kind == EntityKind::Npc && !target_is_beast {
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
                "\n【合社獵團協同助陣】領隊【{}】率領之{}合社獵團（共 {} 人）自側翼吹響山豬牙號角、刀盾齊進封死「{}」退路，助你穩住陣腳、氣息大振！",
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
                        "\n【見者有份・合社分鹿】領隊【{}】率眾上前擊掌喝彩，依半島合社古規與你當場分鹿割皮——你額外分得一份「{}」與合圍紅利 36 枚鎂幣，獵團隨即滿載獵獲啟程趕往【{}】交割！",
                        party.leader_name, loot_name, party.market_name
                    ));
                }
            }
        }
    }
    if target_is_beast && fight_winner == "attacker" {
        if let Some((loot_id, loot_name, shanhai_note)) = crate::game::ambient_creature_loot(&target.display_title) {
            let (gx, gy) = if let Some(rest) = target_id.strip_prefix("shanhai_") {
                let parts: Vec<&str> = rest.split('_').collect();
                if parts.len() == 4
                    && let (Ok(x), Ok(y)) = (parts[2].parse::<i32>(), parts[3].parse::<i32>())
                {
                    (x, y)
                } else {
                    (target.x, target.y)
                }
            } else if let Some(rest) = target_id.strip_prefix("apex_") {
                let parts: Vec<&str> = rest.split('_').collect();
                if parts.len() == 3
                    && let (Ok(x), Ok(y)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>())
                {
                    (x, y)
                } else {
                    (target.x, target.y)
                }
            } else if let Some(rest) = target_id.strip_prefix("ambient_flock_") {
                let parts: Vec<&str> = rest.split('_').collect();
                if parts.len() == 2
                    && let (Ok(x), Ok(y)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>())
                {
                    (x, y)
                } else {
                    (target.x, target.y)
                }
            } else if let Some(rest) = target_id.strip_prefix("ambient_") {
                let parts: Vec<&str> = rest.split('_').collect();
                if parts.len() == 2
                    && let (Ok(x), Ok(y)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>())
                {
                    (x, y)
                } else {
                    (target.x, target.y)
                }
            } else if target.x != 0 || target.y != 0 {
                (target.x, target.y)
            } else {
                (attacker.x, attacker.y)
            };

            crate::game::record_wilderness_kill_at_coord(
                gx,
                gy,
                target_id,
                &target.display_title,
                loot_id,
                loot_name,
                &attacker.display_title,
                now,
            );
            let corpse = crate::pg::living_world::register_beast_corpse(
                &target.display_title,
                loot_id,
                loot_name,
                shanhai_note,
                2,
                now,
            );
            let _ = db::add_to_inventory(pid, &corpse.corpse_item_id, 1);
            if subdue {
                narrative.push_str(&format!(
                    "\n【山海生擒】你收住最後一分勁道將「{}」生擒制伏，隨手收妥【{}殘軀】——可執行【肢體分解】取下「{}」等部位材料，或攜往百工坊與庖廚入饌成器。",
                    target.display_title, target.display_title, loot_name
                ));
            } else {
                narrative.push_str(&format!(
                    "\n【山海獵獲】「{}」轟然倒地，你將尚留餘溫的【{}殘軀】收入行囊——可執行【肢體分解】取下「{}」等部位材料，或攜往百工坊與庖廚入饌成器。",
                    target.display_title, target.display_title, loot_name
                ));
            }
        }
    }
    let room_name = super::target_grid_room_name(target);
    let effective_room_name = if !room_name.is_empty() {
        room_name
    } else {
        db::get_room_name(player_room).unwrap_or_default()
    };
    let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
        &effective_room_name,
        &target.display_title,
    );

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
        tactical_choices: if (target_is_beast || !subdue) && fight_winner == "attacker" {
            vec![]
        } else {
            vec![
                "趁勢搶攻".into(),
                "專攻部位".into(),
                "守勢游鬥".into(),
                "搏命硬撼".into(),
                "抽身脫離".into(),
            ]
        },
    });
    ACTIVE_ENGAGEMENTS
        .write()
        .unwrap()
        .insert(pid.to_string(), (target.id.clone(), now + 300));
    let clean_target_title = target
        .display_title
        .rsplit(['|', '｜'])
        .next()
        .unwrap_or("")
        .trim();
    let combat_log_target = if !clean_target_title.is_empty() {
        clean_target_title.to_string()
    } else {
        super::player_visible_target_name(pid, &target)
    };
    let _ = event::append(now, pid, types::COMBAT, &format!("與{}交手", combat_log_target));
    let _ = update_vit(pid, attacker_hp);
    let _ = update_vit(target_id, defender_hp);
    if let Some(pool) = &crate::pg::pool() {
        if !target_is_beast {
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
        }
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
    if target.kind == EntityKind::Npc && !target_is_beast {
        let player_name = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.display_title)
            .unwrap_or_else(|| "遊歷客友".to_string());
        crate::npc::causal_core::mutate_npc_from_player_action(
            target_id,
            pid,
            &player_name,
            player_room,
            if subdue { "combat_subdue" } else { "combat_slay" },
            &target.display_title,
            0,
            now,
        );
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

include!("combat/vessel.rs");
include!("combat/tactics.rs");
include!("combat/tests.rs");

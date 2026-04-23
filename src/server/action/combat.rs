//! `do_action` — Subdue / Slay 分支：戰鬥結算、死亡廣播。

use std::collections::HashSet;

use crate::combat::{resolve_v2, CombatOpt};
use crate::db::{
    self, adjust_disposition, adjust_favorability, get_assignments_for_entity,
    get_room_ids_for_venue, log_npc_event, remove_assignments_for_entity,
    remove_schedule_for_entity, terrain_from_room, update_vit, DISP_SUBDUED, EVT_DEATH,
    FAV_SLAY, FAV_SUBDUE,
};
use crate::entity::{Character, EntityKind};
use crate::event::{self, types};
use crate::gametext;
use crate::npc::npc_behavior_reaction_line;

use super::super::broadcast::send_narrate_to_room;
use super::super::handler::WsConnection;
use super::super::protocol::ActionResultMsg;
use super::super::session::SessionStore;
use super::display_target_name;

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
    let (narrative, fight_winner, attacker_hp, defender_hp) =
        build_attack_narrative(player_room, &attacker, target, subdue);
    let mut narrative = narrative;
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
    let out_action = if subdue { "Subdue" } else { "Slay" };
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: out_action.into(),
        target_id: target.id.clone(),
        target_name: display_target_name(target),
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
    });
    let _ = event::append(now, pid, types::COMBAT, target_id);
    let _ = update_vit(pid, attacker_hp);
    let _ = update_vit(target_id, defender_hp);
    let _ = event::append(now, pid, types::VIT, &attacker_hp.to_string());
    let _ = event::append(now, target_id, types::VIT, &defender_hp.to_string());
    if target.kind == EntityKind::Npc {
        if subdue {
            let _ = adjust_favorability(target_id, pid, FAV_SUBDUE);
            if fight_winner == "attacker" {
                let _ = adjust_disposition(target_id, DISP_SUBDUED);
            }
        } else {
            let _ = adjust_favorability(target_id, pid, FAV_SLAY);
        }
    }
    if target.kind == EntityKind::Npc && !subdue && defender_hp <= 0 {
        broadcast_npc_death(conn, player_room, target_id, &display_target_name(target), now);
        let _ = remove_assignments_for_entity(target_id);
        let _ = remove_schedule_for_entity(target_id);
    }
}

fn build_attack_narrative(
    room_id: &str,
    attacker: &Character,
    defender: &Character,
    subdue: bool,
) -> (String, String, i32, i32) {
    let mut opt = CombatOpt {
        subdue,
        ..Default::default()
    };
    let t = terrain_from_room(room_id);
    if !t.is_empty() {
        opt.terrain = t;
    }
    if let Some(seed) = attacker.soul_seed {
        let (a, _b, g) = db::expand_soul_seed_to_combat_axes(seed);
        opt.alpha = a;
        opt.gamma = g;
    }
    if let Some(seed) = defender.soul_seed {
        let (_a, _b, g) = db::expand_soul_seed_to_combat_axes(seed);
        opt.gamma = (opt.gamma + g) / 2.0;
    }
    let (a_vit, _a_qi, a_dex, a_atk) = db::effective_stats(attacker);
    let (d_vit, _d_qi, d_dex, d_atk) = db::effective_stats(defender);
    if a_atk > 0 {
        opt.alpha += a_atk as f64 * 0.1;
    }
    if d_atk > 0 {
        opt.alpha += d_atk as f64 * 0.05;
    }

    let (winner, raw_log, a_hp, d_hp) =
        resolve_v2(a_vit, a_dex, d_vit, d_dex, Some(&opt));
    let mut a_name = display_target_name(attacker);
    let mut d_name = display_target_name(defender);
    if a_name.is_empty() {
        a_name.clone_from(&attacker.id);
    }
    if d_name.is_empty() {
        d_name.clone_from(&defender.id);
    }
    let log = raw_log
        .replace("攻方", &format!("【{a_name}】"))
        .replace("守方", &format!("【{d_name}】"));
    let prefix = if subdue {
        format!("你對【{d_name}】出手，意在留人！")
    } else {
        format!("你對【{d_name}】出手，意在送行！")
    };
    let suffix = if winner == "attacker" {
        if subdue {
            "\n你留住了對方。"
        } else {
            "\n你取得了勝利。"
        }
    } else {
        "\n你敗下陣來。"
    };
    (format!("{prefix}{log}{suffix}"), winner, a_hp, d_hp)
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
    let narrate_text = format!("傳來消息：【{name}】倒下了。");
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

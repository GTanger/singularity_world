//! 登入／創角／登入後推視野。

use std::collections::HashMap;

use crate::db::{self, compute_resource_maxes, expand_soul_seed_to_origin_sentence, expand_soul_seed_to_topology_costs};
use crate::entity::Character;
use crate::game;
use crate::gametext;
use crate::store;

use super::super::broadcast::send_room_view_to_session;
use super::super::hex_editor;
use super::super::protocol::{ClientMsg, MeMsg};
use super::super::session::Session;
use super::movement::send_grid_view;
use super::{current_game_hour, parse_activated_nodes, WsConnection};

pub(super) fn handle_login(conn: &WsConnection, msg: &ClientMsg) {
    tracing::info!("[handle_login] start pid={} conn={}", msg.player_id, conn.conn_id);
    if msg.player_id.is_empty() {
        conn.send_error(gametext::client("login_need_id"));
        return;
    }
    if msg.password.is_empty() {
        conn.send_error(gametext::client("login_need_password"));
        return;
    }
    tracing::info!("[handle_login] a before db::get_entity pid={}", msg.player_id);
    let Ok(ent) = db::get_entity(&msg.player_id) else {
        conn.send_error(gametext::client("login_not_found"));
        return;
    };
    tracing::info!("[handle_login] b after db::get_entity pid={}", msg.player_id);
    let Some(ent) = ent else {
        conn.send_error(gametext::client("login_not_found"));
        return;
    };
    if !matches!(ent.kind, crate::entity::EntityKind::Player) {
        conn.send_error(gametext::client("login_not_player"));
        return;
    }
    tracing::info!("[handle_login] c before has_password pid={}", msg.player_id);
    if !db::has_password_for_entity(&msg.player_id) {
        conn.send_error(gametext::client("login_no_password"));
        return;
    }
    tracing::info!("[handle_login] d before verify_password pid={}", msg.player_id);
    let Ok(ok) = db::verify_password(&msg.player_id, &msg.password) else {
        conn.send_error(gametext::client("login_verify_failed"));
        return;
    };
    tracing::info!("[handle_login] e after verify_password pid={} ok={ok}", msg.player_id);
    if !ok {
        conn.send_error(gametext::client("login_wrong_password"));
        return;
    }
    login_success(conn, &msg.player_id);
}

pub(super) fn handle_create_character(conn: &WsConnection, msg: &ClientMsg) {
    if msg.player_id.is_empty() {
        conn.send_error(gametext::client("create_need_id"));
        return;
    }
    if msg.password.is_empty() {
        conn.send_error(gametext::client("create_need_password"));
        return;
    }
    if msg.password.len() < 6 {
        conn.send_error(gametext::client("create_password_short"));
        return;
    }
    if msg.player_id.len() < 2 || msg.player_id.len() > 32 {
        conn.send_error(gametext::client("create_id_len"));
        return;
    }
    let Ok(existing) = db::get_entity(&msg.player_id) else {
        conn.send_error(gametext::client("create_failed"));
        return;
    };
    if existing.is_some() {
        conn.send_error(gametext::client("create_id_taken"));
        return;
    }
    let mut display_char = msg.display_char.clone();
    if display_char.is_empty() {
        display_char = gametext::client("display_char_default");
    }
    let chs: Vec<char> = display_char.chars().collect();
    if chs.len() > 1 {
        display_char = chs[0].to_string();
    }
    let gender = if msg.gender == "女" {
        "F"
    } else {
        "M"
    };
    if db::insert_entity(&msg.player_id, &display_char, gender).is_err() {
        conn.send_error(gametext::client("create_entity_failed"));
        return;
    }
    // 出生唯一規則：Hex 格 (0,0) 契約為草原；權威座標寫入 PG（不再於創角時綁 room_id）。
    let spawn_hex = hex_editor::ensure_player_spawn_grassland_coord();
    let Some(st) = store::get_store() else {
        conn.send_error(gametext::client("create_spawn_hex_failed"));
        return;
    };
    match spawn_hex {
        Ok((q, r)) => {
            let mut g = st.write();
            if g.set_entity_hex(&msg.player_id, q, r).is_err() {
                conn.send_error(gametext::client("create_spawn_hex_failed"));
                return;
            }
        }
        Err(e) => {
            tracing::error!("ensure_player_spawn_grassland_coord: {e}");
            conn.send_error(gametext::client("create_spawn_hex_failed"));
            return;
        }
    }
    if db::create_auth(&msg.player_id, &msg.password).is_err() {
        conn.send_error(gametext::client("create_auth_failed"));
        return;
    }
    login_success(conn, &msg.player_id);
}

fn login_success(conn: &WsConnection, player_id: &str) {
    tracing::info!("[login_success] start pid={player_id} conn={}", conn.conn_id);
    if let Ok(mut g) = conn.player_id.write() {
        *g = Some(player_id.to_string());
    }
    tracing::info!("[login_success] 1 player_id write OK pid={player_id}");
    let gh = current_game_hour(&conn.cfg);
    let mut ent_pre = db::get_entity(player_id).ok().flatten();
    tracing::info!("[login_success] 2 db::get_entity OK pid={player_id}");
    let Some(e0) = ent_pre.as_ref() else {
        conn.send_error(gametext::client("load_view_failed"));
        return;
    };
    let mut hq = e0.hex_q;
    let mut hr = e0.hex_r;
    if hq.is_none() || hr.is_none() {
        match hex_editor::ensure_player_spawn_grassland_coord() {
            Ok((q, r)) => {
                if db::set_entity_hex(player_id, q, r).is_err() {
                    conn.send_error(gametext::client("create_spawn_hex_failed"));
                    return;
                }
                hq = Some(q);
                hr = Some(r);
            }
            Err(e) => {
                tracing::error!("login spawn hex: {e}");
                conn.send_error(gametext::client("create_spawn_hex_failed"));
                return;
            }
        }
        ent_pre = db::get_entity(player_id).ok().flatten();
    }
    let (hq, hr) = match (hq, hr) {
        (Some(q), Some(r)) => (q, r),
        _ => {
            conn.send_error(gametext::client("create_spawn_hex_failed"));
            return;
        }
    };
    // 確保玩家也有 grid 座標（正方格）
    let ent_now = db::get_entity(player_id).ok().flatten();
    let (gx, gy) = match ent_now.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)) {
        Some(pair) => pair,
        None => {
            let _ = db::set_entity_grid(player_id, 0, 0);
            (0, 0)
        }
    };
    tracing::info!("[login_success] 3 before with_square_grid_mut pid={player_id}");
    crate::server::grid_manager::with_square_grid_mut(|grid| {
        crate::grid::reveal_grid_disk(grid, crate::grid::SquareCoord::new(gx, gy), 5);
    });
    tracing::info!("[login_success] 4 with_square_grid_mut OK pid={player_id}");

    let view = match game::get_hex_room_view(player_id, hq, hr, gh) {
        Ok(Some(v)) => v,
        Ok(None) | Err(_) => {
            conn.send_error(gametext::client("load_view_failed"));
            return;
        }
    };
    tracing::info!("[login_success] 5 get_hex_room_view OK pid={player_id}");
    let session = Session::with_outbound(player_id, conn.conn_id, conn.tx.clone());
    conn.sessions.set(player_id, session.clone());
    tracing::info!("[login_success] 6 sessions.set OK pid={player_id}");
    let ent = ent_pre.or_else(|| db::get_entity(player_id).ok().flatten());
    let (vit, qi, dex) = ent
        .as_ref()
        .map(|e| (e.vit, e.qi, e.dex))
        .unwrap_or((10, 10, 10));
    let rm = compute_resource_maxes(vit, qi, dex);
    send_room_view_to_session(&session, &view, player_id, &conn.cfg);
    tracing::info!("[login_success] 7 send_room_view OK pid={player_id}");

    // 額外發送正方格視野
    send_grid_view(conn, player_id, gx, gy, gh);
    tracing::info!("[login_success] 8 send_grid_view OK pid={player_id}");

    let now = game::now_unix();
    for e in &view.entities {
        if matches!(e.kind, crate::entity::EntityKind::Npc) && e.id != player_id {
            let _ = db::record_meet(&e.id, player_id);
        }
    }
    game::observe_hex(hq, hr, player_id, now);
    send_me_with_status(conn, &session, ent.as_ref(), player_id, &view.room.id, &view.room.name, vit, qi, dex, &rm);
    tracing::info!("[login_success] 9 DONE pid={player_id}");
}

#[allow(clippy::too_many_arguments)]
fn send_me_with_status(
    conn: &WsConnection,
    _session: &Session,
    ent: Option<&Character>,
    player_id: &str,
    room_id: &str,
    room_name: &str,
    vit: i32,
    qi: i32,
    dex: i32,
    rm: &db::ResourceMaxes,
) {
    let mut msg = MeMsg {
        msg_type: "me".into(),
        player_id: player_id.to_string(),
        room_id: room_id.to_string(),
        room_name: room_name.to_string(),
        vit,
        qi,
        dex,
        hp_cur: rm.hp_max as i32,
        hp_max: rm.hp_max as i32,
        inner_cur: rm.inner_max as i32,
        inner_max: rm.inner_max as i32,
        spirit_cur: rm.spirit_max as i32,
        spirit_max: rm.spirit_max as i32,
        stamina_cur: rm.stamina_max as i32,
        stamina_max: rm.stamina_max as i32,
        display_title: String::new(),
        origin_sentence: String::new(),
        activated_nodes: vec![],
        topology_costs: vec![],
        equipment_slots: HashMap::new(),
        equipment_names: HashMap::new(),
    };
    if let Some(ent) = ent {
        if !ent.display_title.is_empty() {
            msg.display_title.clone_from(&ent.display_title);
        }
        if let Some(seed) = ent.soul_seed {
            msg.origin_sentence = expand_soul_seed_to_origin_sentence(seed);
            msg.topology_costs = expand_soul_seed_to_topology_costs(seed);
        }
        msg.activated_nodes = parse_activated_nodes(&ent.activated_nodes);
        if !ent.equipment_slots.is_empty() {
            if let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(&ent.equipment_slots) {
                msg.equipment_slots = slots;
            }
            msg.equipment_names = db::get_item_names(&ent.equipment_slots);
        }
    } else {
        msg.activated_nodes = vec!["N000".into()];
    }
    conn.send_json(&msg);
}

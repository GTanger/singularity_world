//! 登入／創角／登入後推視野。

use std::collections::HashMap;

use crate::db::{
    self, compute_resource_maxes, expand_soul_seed_to_origin_sentence,
    expand_soul_seed_to_topology_costs,
};
use crate::entity::Character;
use crate::game;
use crate::gametext;

use super::super::protocol::{ClientMsg, MeMsg};
use super::super::session::Session;
use super::{WsConnection, current_game_hour, parse_activated_nodes};

pub(super) fn handle_login(conn: &WsConnection, msg: &ClientMsg) {
    if msg.player_id.is_empty() {
        conn.send_error(gametext::client("login_need_id"));
        return;
    }
    if msg.password.is_empty() {
        conn.send_error(gametext::client("login_need_password"));
        return;
    }
    let Ok(ent) = db::get_entity(&msg.player_id) else {
        conn.send_error(gametext::client("login_not_found"));
        return;
    };
    let Some(ent) = ent else {
        conn.send_error(gametext::client("login_not_found"));
        return;
    };
    if !matches!(ent.kind, crate::entity::EntityKind::Player) {
        conn.send_error(gametext::client("login_not_player"));
        return;
    }
    if !db::has_password_for_entity(&msg.player_id) {
        conn.send_error(gametext::client("login_no_password"));
        return;
    }
    let Ok(ok) = db::verify_password(&msg.player_id, &msg.password) else {
        conn.send_error(gametext::client("login_verify_failed"));
        return;
    };
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
    let gender = if msg.gender == "女" { "F" } else { "M" };
    if db::insert_entity(&msg.player_id, &display_char, gender).is_err() {
        conn.send_error(gametext::client("create_entity_failed"));
        return;
    }
    // login_success 會把新玩家放進白塔半島的正式起點。
    if db::create_auth(&msg.player_id, &msg.password).is_err() {
        conn.send_error(gametext::client("create_auth_failed"));
        return;
    }
    login_success(conn, &msg.player_id);
}

fn login_success(conn: &WsConnection, player_id: &str) {
    tracing::info!(
        "[login_success] start pid={player_id} conn={}",
        conn.conn_id
    );
    if let Ok(mut g) = conn.player_id.write() {
        *g = Some(player_id.to_string());
    }
    tracing::info!("[login_success] 1 player_id write OK pid={player_id}");
    let gh = current_game_hour(&conn.cfg);
    let ent_pre = db::get_entity(player_id).ok().flatten();
    tracing::info!("[login_success] 2 db::get_entity OK pid={player_id}");
    let Some(_e0) = ent_pre.as_ref() else {
        conn.send_error(gametext::client("load_view_failed"));
        return;
    };
    let session = Session::with_outbound(player_id, conn.conn_id, conn.tx.clone());
    conn.sessions.set(player_id, session.clone());
    tracing::info!("[login_success] 3 sessions.set OK pid={player_id}");
    let ent = ent_pre.or_else(|| db::get_entity(player_id).ok().flatten());
    let (vit, qi, dex) = ent
        .as_ref()
        .map(|e| (e.vit, e.qi, e.dex))
        .unwrap_or((10, 10, 10));
    let rm = compute_resource_maxes(vit, qi, dex);

    let runtime_grid = crate::server::grid_manager::get_runtime_square_grid();
    let (gx, gy) = match ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)) {
        Some((x, y))
            if runtime_grid
                .as_ref()
                .and_then(|g| g.get(crate::grid::SquareCoord::new(x, y)))
                .map_or(false, |c| c.terrain.walkable()) =>
        {
            (x, y)
        }
        _ => {
            let _ = db::set_entity_grid(player_id, 0, 0);
            (0, 0)
        }
    };
    let grid_room_id = crate::grid::grid_room_id_from_coord(gx, gy);
    let grid_room_name = crate::server::grid_manager::get_runtime_square_grid()
        .and_then(|g| g.get(crate::grid::SquareCoord::new(gx, gy)).cloned())
        .map(|c| {
            if c.name.trim().is_empty() {
                crate::grid::terrain_name_zh(c.terrain).to_string()
            } else {
                c.name.clone()
            }
        })
        .unwrap_or_else(|| "未知荒野".to_string());

    // 玩家已經通過驗證、位置與視野也已就緒；先把本人狀態送到前端，讓登入畫面
    // 立即切換。
    send_me_with_status(
        conn,
        &session,
        ent.as_ref(),
        player_id,
        &grid_room_id,
        &grid_room_name,
        vit,
        qi,
        dex,
        &rm,
    );
    tracing::info!("[login_success] 4 player ready pid={player_id}");

    super::movement::send_grid_view(conn, player_id, gx, gy, gh);
    super::inventory::handle_get_tactical_state(conn);
    let recaps = crate::npc::causal_core::drain_player_offline_recap(player_id, game::now_unix());
    for (cat, title, body) in recaps {
        conn.send_json(&serde_json::json!({
            "type": "recap_item",
            "category": cat,
            "title": title,
            "body": body,
        }));
    }
    // 異步登記房間觀測（不再於每次登入自動累加全房 NPC meet_count）與切換為玩家親自控制
    let observed_player_id = player_id.to_string();
    let observed_room_id = grid_room_id;
    let observed_at = game::now_unix();
    drop(tokio::task::spawn_blocking(move || {
        game::observe_room(&observed_room_id, &observed_player_id, observed_at);
        if let Some(pool) = crate::pg::pool()
            && let Ok(mut client) = pool.get()
        {
            let _ = client.execute(
                "INSERT INTO resident_continuity (
                    entity_id, control_mode, life_status, current_intent,
                    intent_reason, destination_place_id, intent_started_at,
                    next_wake_at, updated_at, source_kind, source_ref
                ) VALUES (
                    $1, 'player', 'alive', 'player_directed',
                    '人物本人親自行動', NULL, $2,
                    NULL, $2, 'player_auth', 'login_success'
                )
                ON CONFLICT (entity_id) DO UPDATE SET
                    control_mode = 'player',
                    current_intent = 'player_directed',
                    intent_reason = '人物本人親自行動',
                    next_wake_at = NULL,
                    updated_at = EXCLUDED.updated_at",
                &[&observed_player_id, &observed_at],
            );
        }
        tracing::info!("[login_success] post-login observations done pid={observed_player_id}");
    }));
    tracing::info!("[login_success] 5 post-login observations scheduled pid={player_id}");
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
        offline_directive: {
            let dir = crate::npc::causal_core::get_player_offline_directive(player_id);
            dir.code().to_string()
        },
        offline_directive_label: {
            let dir = crate::npc::causal_core::get_player_offline_directive(player_id);
            dir.label().to_string()
        },
        offline_directive_desc: {
            let dir = crate::npc::causal_core::get_player_offline_directive(player_id);
            dir.desc().to_string()
        },
        has_openrouter_key: crate::ai::get_player_openrouter_key(player_id).is_some(),
        openrouter_model: crate::ai::DEFAULT_OPENROUTER_TALK_MODEL.to_string(),
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
            if let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(&ent.equipment_slots)
            {
                msg.equipment_slots = slots;
            }
            msg.equipment_names = db::get_item_names(&ent.equipment_slots);
        }
    } else {
        msg.activated_nodes = vec!["N000".into()];
    }
    conn.send_json(&msg);
}

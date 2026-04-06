//! WebSocket 訊息分派（對齊既有 `server/handler` + `handler_auth` + `handler_move` 子集）。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::config::Server;
use crate::db::{
    self, add_to_inventory, clear_equipment_slot, compute_resource_maxes, expand_soul_seed_to_origin_sentence,
    expand_soul_seed_to_topology_costs, get_inventory, get_item_def, inventory_has_item, inventory_weight,
    remove_from_inventory, update_equipment_slot,
};
use crate::entity::Character;
use crate::event::{self, types::BLOCKED};
use crate::game;
use crate::gametext;
use crate::store;

use super::action::handle_do_action;
use super::hex_editor;
use super::broadcast::send_room_view_to_session;
use super::hub::Hub;
use super::protocol::{
    BlockedMsg, ClientMsg, EntityStatusMsg, InventoryItemView, InventoryMsg, MeMsg, MovedMsg, PongMsg,
    TopologyDebugAckMsg,
};
use super::session::{Session, SessionStore};
use uuid::Uuid;

/// 單一 WS 連線脈絡（對齊既有 `Client` + session 綁定）。
pub struct WsConnection {
    pub conn_id: Uuid,
    pub tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    pub player_id: RwLock<Option<String>>,
    pub sessions: Arc<SessionStore>,
    pub hub: Arc<Hub>,
    pub cfg: Server,
}

impl WsConnection {
    pub fn send_error(&self, message: impl Into<String>) {
        let msg = serde_json::json!({
            "type": "error",
            "message": message.into(),
        });
        let _ = self.tx.blocking_send(msg.to_string().into_bytes());
    }

    pub(super) fn send_json<T: serde::Serialize>(&self, v: &T) {
        if let Ok(bytes) = serde_json::to_vec(v) {
            let _ = self.tx.blocking_send(bytes);
        }
    }
}

pub fn handle_message(conn: &WsConnection, raw: &[u8]) {
    let msg: ClientMsg = match serde_json::from_slice(raw) {
        Ok(m) => m,
        Err(_) => {
            conn.send_error("invalid json");
            return;
        }
    };
    match msg.msg_type.as_str() {
        "login" => handle_login(conn, &msg),
        "create_character" => handle_create_character(conn, &msg),
        "move" => handle_move(conn, &msg),
        "ping" => {
            conn.send_json(&PongMsg {
                msg_type: "pong".into(),
            });
        }
        "get_entity_status" => handle_get_entity_status(conn, &msg),
        "get_inventory" => handle_get_inventory(conn),
        "equip_item" => handle_equip_item(conn, &msg),
        "unequip_item" => handle_unequip_item(conn, &msg),
        "print_topology_debug" => handle_print_topology_debug(conn),
        "do_action" => handle_do_action(conn, &msg),
        other => {
            conn.send_error(format!("unknown type: {other}"));
        }
    }
}

fn handle_login(conn: &WsConnection, msg: &ClientMsg) {
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

fn handle_create_character(conn: &WsConnection, msg: &ClientMsg) {
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
            if let Ok(mut g) = st.write() {
                if g.set_entity_hex(&msg.player_id, q, r).is_err() {
                    conn.send_error(gametext::client("create_spawn_hex_failed"));
                    return;
                }
            } else {
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
    if let Ok(mut g) = conn.player_id.write() {
        *g = Some(player_id.to_string());
    }
    let gh = current_game_hour(&conn.cfg);
    let mut ent_pre = db::get_entity(player_id).ok().flatten();
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
    let view = match game::get_hex_room_view(player_id, hq, hr, gh) {
        Ok(Some(v)) => v,
        Ok(None) | Err(_) => {
            conn.send_error(gametext::client("load_view_failed"));
            return;
        }
    };
    let session = Session::with_outbound(player_id, conn.conn_id, conn.tx.clone());
    conn.sessions.set(player_id, session.clone());
    let ent = ent_pre.or_else(|| db::get_entity(player_id).ok().flatten());
    let (vit, qi, dex) = ent
        .as_ref()
        .map(|e| (e.vit, e.qi, e.dex))
        .unwrap_or((10, 10, 10));
    let rm = compute_resource_maxes(vit, qi, dex);
    send_room_view_to_session(&session, &view, player_id, &conn.cfg);
    let now = game::now_unix();
    for e in &view.entities {
        if matches!(e.kind, crate::entity::EntityKind::Npc) && e.id != player_id {
            let _ = db::record_meet(&e.id, player_id);
        }
    }
    game::observe_hex(hq, hr, player_id, now);
    send_me_with_status(conn, &session, ent.as_ref(), player_id, &view.room.id, &view.room.name, vit, qi, dex, &rm);
}

fn current_game_hour(cfg: &Server) -> i32 {
    if cfg.game_time_epoch_unix == 0 {
        return 12;
    }
    let (_, h, _, _) = game::game_time_now(
        game::now_unix(),
        cfg.game_time_epoch_unix,
        cfg.game_time_scale,
    );
    h
}

fn parse_activated_nodes(raw: &str) -> Vec<String> {
    if raw.trim().is_empty() {
        return vec!["N000".into()];
    }
    let list: Vec<String> = serde_json::from_str(raw).unwrap_or_else(|_| vec!["N000".into()]);
    if list.is_empty() {
        vec!["N000".into()]
    } else {
        list
    }
}

pub(super) fn player_id(conn: &WsConnection) -> Option<String> {
    conn.player_id.read().ok().and_then(|g| g.clone())
}

/// 解析裝備槽 JSON 並補物品名稱／描述（對齊既有 `parseEquipment`）。
fn parse_equipment_triple(raw: &str) -> (HashMap<String, String>, HashMap<String, String>, HashMap<String, String>) {
    if raw.is_empty() {
        return (HashMap::new(), HashMap::new(), HashMap::new());
    }
    let slots: HashMap<String, String> = serde_json::from_str(raw).unwrap_or_default();
    let names = db::get_item_names(raw);
    let descs = db::get_item_descs(raw);
    (slots, names, descs)
}

/// 組裝 `entity_status` 訊息（第一版當前資源＝上限，對齊既有）。
fn entity_status_from_character(ent: &Character, is_self: bool) -> EntityStatusMsg {
    let rm = compute_resource_maxes(ent.vit, ent.qi, ent.dex);
    let hp = rm.hp_max as i32;
    let inner = rm.inner_max as i32;
    let spirit = rm.spirit_max as i32;
    let stamina = rm.stamina_max as i32;
    let magnesium = if is_self { Some(ent.magnesium) } else { None };
    let (equipment_slots, equipment_names, equipment_descs) = parse_equipment_triple(&ent.equipment_slots);
    let mut msg = EntityStatusMsg {
        msg_type: "entity_status".into(),
        entity_id: ent.id.clone(),
        display_char: ent.display_char.clone(),
        vit: ent.vit,
        qi: ent.qi,
        dex: ent.dex,
        hp_cur: hp,
        hp_max: hp,
        inner_cur: inner,
        inner_max: inner,
        spirit_cur: spirit,
        spirit_max: spirit,
        stamina_cur: stamina,
        stamina_max: stamina,
        magnesium,
        is_self,
        display_title: String::new(),
        origin_sentence: String::new(),
        activated_nodes: vec![],
        topology_costs: vec![],
        equipment_slots,
        equipment_names,
        equipment_descs,
    };
    if !ent.display_title.is_empty() {
        msg.display_title.clone_from(&ent.display_title);
    }
    if is_self {
        msg.activated_nodes = parse_activated_nodes(&ent.activated_nodes);
        if let Some(seed) = ent.soul_seed {
            msg.origin_sentence = expand_soul_seed_to_origin_sentence(seed);
            msg.topology_costs = expand_soul_seed_to_topology_costs(seed);
        }
    }
    msg
}

/// 自實體組裝 `inventory` 訊息。
fn inventory_msg_for_character(ent: &Character) -> InventoryMsg {
    let inv = get_inventory(&ent.inventory, ent.vit);
    InventoryMsg {
        msg_type: "inventory".into(),
        items: inv
            .items
            .into_iter()
            .map(|it| InventoryItemView {
                item_id: it.item_id,
                name: it.name,
                item_type: it.item_type,
                qty: it.qty,
                weight: it.weight,
                sub_total: it.sub_total,
                description: it.description,
                slot: it.slot,
            })
            .collect(),
        current_weight: inv.current_weight,
        max_weight: inv.max_weight,
    }
}

/// 裝備變更後推播背包與自身狀態（對齊既有 `pushRefresh`）。
pub(super) fn push_refresh(conn: &WsConnection, player_id: &str) {
    let Ok(Some(ent)) = db::get_entity(player_id) else {
        return;
    };
    conn.send_json(&inventory_msg_for_character(&ent));
    conn.send_json(&entity_status_from_character(&ent, true));
}

fn handle_get_entity_status(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let mut entity_id = msg.entity_id.clone();
    if entity_id.is_empty() {
        entity_id.clone_from(&pid);
    }
    let Ok(Some(ent)) = db::get_entity(&entity_id) else {
        conn.send_error(gametext::client("status_entity_not_found"));
        return;
    };
    let is_self = entity_id == pid;
    conn.send_json(&entity_status_from_character(&ent, is_self));
}

fn handle_get_inventory(conn: &WsConnection) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    conn.send_json(&inventory_msg_for_character(&ent));
}

fn handle_equip_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.item_id.is_empty() {
        conn.send_error(gametext::client("inv_no_item"));
        return;
    }
    let Some(def) = get_item_def(&msg.item_id) else {
        conn.send_error(gametext::client("inv_item_not_found"));
        return;
    };
    if def.item_type != "equipment" || def.slot.is_empty() {
        conn.send_error(gametext::client("inv_cannot_equip"));
        return;
    }
    let mut target_slot = def.slot.clone();
    if target_slot == "hold" {
        target_slot.clone_from(&msg.target_slot);
        if target_slot != "hold_l" && target_slot != "hold_r" {
            conn.send_error(gametext::client("inv_hand_slot"));
            return;
        }
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    if !inventory_has_item(&ent.inventory, &msg.item_id) {
        conn.send_error(gametext::client("inv_bag_missing"));
        return;
    }
    let current_slots: HashMap<String, String> = if ent.equipment_slots.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_str(&ent.equipment_slots).unwrap_or_default()
    };
    let old_item_id = current_slots.get(&target_slot).cloned().unwrap_or_default();
    if remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    if update_equipment_slot(&pid, &target_slot, &msg.item_id).is_err() {
        conn.send_error(gametext::client("inv_equip_fail"));
        return;
    }
    if !old_item_id.is_empty() && add_to_inventory(&pid, &old_item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_unequip_old_fail"));
        return;
    }
    push_refresh(conn, &pid);
}

fn handle_unequip_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.slot.is_empty() {
        conn.send_error(gametext::client("unequip_no_slot"));
        return;
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    let current_slots: HashMap<String, String> = if ent.equipment_slots.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_str(&ent.equipment_slots).unwrap_or_default()
    };
    if current_slots.is_empty() {
        conn.send_error(gametext::client("unequip_nothing"));
        return;
    }
    let item_id = current_slots.get(&msg.slot).cloned().unwrap_or_default();
    if item_id.is_empty() {
        conn.send_error(gametext::client("unequip_slot_empty"));
        return;
    }
    let weight = get_item_def(&item_id).map(|d| d.weight).unwrap_or(0.0);
    let cur_w = inventory_weight(&ent.inventory);
    let max_w = ent.vit as f64 * 10.0;
    if cur_w + weight > max_w {
        conn.send_error(gametext::client("unequip_bag_full"));
        return;
    }
    if clear_equipment_slot(&pid, &msg.slot).is_err() {
        conn.send_error(gametext::client("unequip_slot_fail"));
        return;
    }
    if add_to_inventory(&pid, &item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    push_refresh(conn, &pid);
}

fn handle_print_topology_debug(conn: &WsConnection) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    let seed = match ent.soul_seed {
        Some(s) if s != 0 => s,
        _ => {
            tracing::info!(target: "topology_debug", "角色無 soul_seed（可能為舊資料）");
            conn.send_error(gametext::client("seed_no_soul"));
            return;
        }
    };
    let costs = expand_soul_seed_to_topology_costs(seed);
    let sum: f64 = costs.iter().sum();
    tracing::info!(target: "topology_debug", "========== 361 拓撲除錯（當前角色） ==========");
    tracing::info!(target: "topology_debug", soul_seed = seed, "SoulSeed (int64)");
    tracing::info!(target: "topology_debug", "N000（生之奇點）→ 前三條電漿流 Cost：");
    if costs.len() >= 3 {
        tracing::info!(target: "topology_debug", c0 = costs[0], "N000 → N001");
        tracing::info!(target: "topology_debug", c1 = costs[1], "N000 → N002");
        tracing::info!(target: "topology_debug", c2 = costs[2], "N000 → N003");
    }
    tracing::info!(target: "topology_debug", sum, "全 760 條連線 Cost 總和（規格常數應為 10000）");
    tracing::info!(target: "topology_debug", "=============================================");
    conn.send_json(&TopologyDebugAckMsg {
        msg_type: "topology_debug".into(),
        message: "已於伺服器終端印出".into(),
    });
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

fn handle_move(conn: &WsConnection, msg: &ClientMsg) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    if msg.direction.is_empty() {
        conn.send_error("direction required");
        return;
    }
    let old_ent = db::get_entity(&player_id).ok().flatten();
    let Some(oe) = old_ent else {
        conn.send_error(gametext::client("move_failed"));
        return;
    };
    let (Some(old_q), Some(old_r)) = (oe.hex_q, oe.hex_r) else {
        conn.send_error(gametext::client("move_failed"));
        return;
    };
    let (new_room, ok, err) = match game::move_by_hex_direction(&player_id, &msg.direction) {
        Ok(v) => (v.0, v.1, None),
        Err(e) => (String::new(), false, Some(e)),
    };
    if err.is_some() {
        conn.send_error(gametext::client("move_failed"));
        return;
    }
    if !ok {
        let _ = event::append(
            game::now_unix(),
            &player_id,
            BLOCKED,
            &msg.direction,
        );
        conn.send_json(&BlockedMsg {
            msg_type: "blocked".into(),
            direction: msg.direction.clone(),
        });
        return;
    }
    on_leave_hex(conn.sessions.as_ref(), old_q, old_r, &player_id);
    let Some((nq, nr)) = crate::hex::parse_hex_room_id(&new_room) else {
        conn.send_error(gametext::client("move_failed"));
        return;
    };
    let gh = current_game_hour(&conn.cfg);
    let view = match game::get_hex_room_view(&player_id, nq, nr, gh) {
        Ok(Some(v)) => v,
        Ok(None) | Err(_) => {
            conn.send_error(gametext::client("load_view_failed"));
            return;
        }
    };
    let Some(sess) = conn.sessions.get(&player_id) else {
        conn.send_error("session lost");
        return;
    };
    send_room_view_to_session(&sess, &view, &player_id, &conn.cfg);
    let now = game::now_unix();
    game::observe_hex(nq, nr, &player_id, now);
    for e in &view.entities {
        if matches!(e.kind, crate::entity::EntityKind::Npc) && e.id != player_id {
            let _ = db::record_meet(&e.id, &player_id);
        }
    }
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: player_id.clone(),
        room_id: new_room.clone(),
        room_name: view.room.name.clone(),
    };
    let bytes = serde_json::to_vec(&moved).unwrap_or_default();
    conn.hub.broadcast(bytes);
}

/// 離開某六角格時，若無其他玩家仍在該格，清除該格 NPC 之最後觀測。
fn on_leave_hex(store: &SessionStore, q: i32, r: i32, left_player_id: &str) {
    for s in store.all_sessions() {
        if s.player_id.is_empty() || s.player_id == left_player_id {
            continue;
        }
        if let Ok(Some(ch)) = db::get_entity(&s.player_id)
            && ch.hex_q == Some(q) && ch.hex_r == Some(r)
        {
            return;
        }
    }
    let Ok(entities) = db::get_entities_at_hex(q, r, -1) else {
        return;
    };
    for e in entities {
        if matches!(e.kind, crate::entity::EntityKind::Npc) {
            let _ = db::clear_last_observed(&e.id);
        }
    }
}

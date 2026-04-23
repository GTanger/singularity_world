//! 背包／裝備／狀態查詢 + 拓撲除錯。

use std::collections::HashMap;

use crate::db::{
    self, add_to_inventory, clear_equipment_slot, compute_resource_maxes,
    expand_soul_seed_to_origin_sentence, expand_soul_seed_to_topology_costs, get_inventory,
    get_item_def, inventory_has_item, inventory_weight, remove_from_inventory,
    update_equipment_slot,
};
use crate::entity::Character;
use crate::gametext;

use super::super::protocol::{
    ClientMsg, EntityStatusMsg, InventoryItemView, InventoryMsg, TopologyDebugAckMsg,
};
use super::{parse_activated_nodes, player_id, WsConnection};

fn parse_equipment_triple(
    raw: &str,
) -> (HashMap<String, String>, HashMap<String, String>, HashMap<String, String>) {
    if raw.is_empty() {
        return (HashMap::new(), HashMap::new(), HashMap::new());
    }
    let slots: HashMap<String, String> = serde_json::from_str(raw).unwrap_or_default();
    let names = db::get_item_names(raw);
    let descs = db::get_item_descs(raw);
    (slots, names, descs)
}

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
pub(crate) fn push_refresh(conn: &WsConnection, player_id: &str) {
    let Ok(Some(ent)) = db::get_entity(player_id) else {
        return;
    };
    conn.send_json(&inventory_msg_for_character(&ent));
    conn.send_json(&entity_status_from_character(&ent, true));
}

pub(super) fn handle_get_entity_status(conn: &WsConnection, msg: &ClientMsg) {
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

pub(super) fn handle_get_inventory(conn: &WsConnection) {
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

pub(super) fn handle_equip_item(conn: &WsConnection, msg: &ClientMsg) {
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

pub(super) fn handle_unequip_item(conn: &WsConnection, msg: &ClientMsg) {
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

pub(super) fn handle_print_topology_debug(conn: &WsConnection) {
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
